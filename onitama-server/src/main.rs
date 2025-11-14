use onitama_lib::state::{NamedField, Piece, PlayerColor, PlayerTurn, State};
use onitama_lib::{
    board_from_str, card_to_pos, Color, ExtraState, LitamaMsg, PieceKind, Sides, StateMsg, CARDS,
    DEFAULT_BOARD,
};
use rand::random;
use rand::seq::SliceRandom;
use rust_query::migration::{schema, Config};
use rust_query::{Database, TableRow, Transaction, Update};
use std::error::Error;
use std::iter::FromIterator;
use std::str::FromStr;

use simple_websockets::{Event, Message, Responder};
use std::collections::{HashMap, HashSet};

#[schema(Schema)]
pub mod vN {
    pub struct Match {
        #[unique]
        pub match_id: String,
        // concatenation of moves like "elephant:a1b2,boar:a2c3"
        pub history: String,
        // these are used for authentication
        pub create_token: String,
        pub join_token: String,

        pub create_name: String,
        pub join_name: Option<String>,

        // either "red" or "blue"
        pub create_color: String,
        // concatenation of blue1,blue2,red1,red2,side
        pub starting_cards: String,
    }
}
use v0::*;

pub struct Client {
    responder: Responder,
    // these are game_ids
    subscriptions: HashSet<String>,
}

impl Client {
    pub fn send_msg(&self, msg: LitamaMsg) {
        let msg = serde_json::to_string(&msg).unwrap();
        println!("{} <== {msg}", self.responder.client_id());
        self.responder.send(Message::Text(msg));
    }
}

fn main() {
    // listen for WebSockets on port 8080:
    let event_hub = simple_websockets::launch(5000).expect("failed to listen on port 5000");
    // map between client ids and the client's `Responder`:
    let mut clients: HashMap<u64, Client> = HashMap::new();

    let db: Database<Schema> = Database::migrator(Config::open("db.sqlite"))
        .unwrap()
        .finish()
        .unwrap();

    loop {
        match event_hub.poll_event() {
            Event::Connect(client_id, responder) => {
                println!("A client connected with id #{}", client_id);
                // add their Responder to our `clients` map:
                clients.insert(
                    client_id,
                    Client {
                        responder,
                        subscriptions: HashSet::new(),
                    },
                );
            }
            Event::Disconnect(client_id) => {
                println!("Client #{} disconnected.", client_id);
                // remove the disconnected client from the clients map:
                clients.remove(&client_id);
            }
            Event::Message(client_id, message) => {
                match &message {
                    Message::Text(msg) => println!("{client_id} ==> {msg}"),
                    Message::Binary(_) => println!("{client_id} ==> <binary>"),
                }

                if let Err(err) = db.transaction_mut(|txn| {
                    handle_message(message.clone(), client_id, &mut clients, txn)
                }) {
                    let client = clients.get(&client_id).unwrap();

                    let query = match message {
                        Message::Text(txt) => txt,
                        Message::Binary(_) => "<binary data>".to_owned(),
                    };
                    let error = err.to_string();
                    client.send_msg(LitamaMsg::Error { error, query });
                    client.responder.close();
                    clients.remove(&client_id);
                }
            }
        }
    }
}

pub fn handle_message(
    message: Message,
    client_id: u64,
    clients: &mut HashMap<u64, Client>,
    txn: &mut Transaction<Schema>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let client = clients.get_mut(&client_id).unwrap();

    let Message::Text(msg) = message else {
        return Err("recieved binary!".into());
    };
    let parts: Vec<_> = msg.split(" ").collect();
    let cmd = *parts.get(0).ok_or("expected command")?;

    if cmd == "create" {
        let username = *parts.get(1).ok_or("expected username")?;

        let match_id = &random::<[u8; 12]>().map(|x| format!("{x:x}")).join("");
        let blue_token = &random::<[u8; 32]>().map(|x| format!("{x:x}")).join("");
        let red_token = &random::<[u8; 32]>().map(|x| format!("{x:x}")).join("");
        let mut rng = rand::thread_rng();
        let cards: Vec<_> = onitama_lib::CARDS
            .choose_multiple(&mut rng, 5)
            .map(|x| x.0)
            .collect();

        txn.insert(Match {
            match_id,
            history: "",
            create_token: blue_token,
            join_token: red_token,
            create_name: username,
            join_name: None::<String>,
            create_color: ["red", "blue"].choose(&mut rng).unwrap(),
            starting_cards: cards.join(","),
        })
        .unwrap();

        client.send_msg(LitamaMsg::Create {
            match_id: match_id.to_string(),
            token: blue_token.to_string(),
            index: 0,
        });

        return Ok(());
    }

    // all other commands require a valid match_id
    let match_id = *parts.get(1).ok_or("expected match_id")?;
    let m = txn
        .lazy(Match.match_id(match_id))
        .ok_or("match does not exist")?;
    let m_row = m.table_row();

    match cmd {
        "join" => {
            let username = *parts.get(2).ok_or("expected username")?;

            if m.join_name.is_some() {
                return Err("match is already joined".into());
            }
            let token = m.join_token.clone();

            txn.update_ok(
                m_row,
                Match {
                    join_name: Update::set(Some(username)),
                    ..Default::default()
                },
            );

            client.send_msg(LitamaMsg::Join {
                match_id: match_id.to_owned(),
                token,
                index: 1,
            });

            for other in clients.values() {
                if other.subscriptions.contains(match_id) {
                    other.send_msg(LitamaMsg::State {
                        match_id: match_id.to_owned(),
                        state: read_state_msg(&txn, m_row), // TODO: maybe optimize this?
                    });
                }
            }
        }
        "state" => {
            client.send_msg(LitamaMsg::State {
                match_id: match_id.to_owned(),
                state: read_state_msg(&txn, m_row),
            });
        }
        "move" => {
            let token = *parts.get(2).ok_or("expected token")?;
            let card = *parts.get(3).ok_or("expected card")?;
            let from_to = *parts.get(4).ok_or("expected move")?;
            if from_to.len() != 4 || !from_to.is_ascii() {
                return Err("move has unexpected len or is not ascii".into());
            }
            let from = NamedField::from_str(&from_to[..2])?;
            let to = NamedField::from_str(&from_to[2..])?;

            let is_red = if token == m.create_token {
                m.create_color == "red"
            } else if token == m.join_token {
                m.create_color != "red"
            } else {
                return Err("token not recognized".into());
            };

            let msg = read_state_msg(&txn, m_row);
            let StateMsg::InProgress { extra, .. } = msg else {
                return Err("game must be in progress".into());
            };

            let state = State::from_state(extra);
            if state.active_eq_red != is_red {
                return Err("it is not your turn".into());
            }

            let state: State = state.translate();
            state.make_move(card, from, to)?;

            let mut history = m.history.clone();
            if !history.is_empty() {
                history.push(',');
            }
            history.push_str(&format!("{card}:{from_to}"));

            txn.update_ok(
                m.table_row(),
                Match {
                    history: Update::set(history),
                    ..Default::default()
                },
            );

            client.send_msg(LitamaMsg::Move {
                match_id: match_id.to_owned(),
            });

            for other in clients.values() {
                if other.subscriptions.contains(match_id) {
                    other.send_msg(LitamaMsg::State {
                        match_id: match_id.to_owned(),
                        state: read_state_msg(&txn, m_row), // TODO: maybe optimize this?
                    });
                }
            }
        }
        "spectate" => {
            client.subscriptions.insert(match_id.to_owned());

            client.send_msg(LitamaMsg::Spectate {
                match_id: match_id.to_owned(),
            });

            client.send_msg(LitamaMsg::State {
                match_id: match_id.to_owned(),
                state: read_state_msg(&txn, m_row),
            });
        }
        _ => return Err("unknown command".into()),
    };

    Ok(())
}

pub fn read_state_msg<'a>(txn: &Transaction<Schema>, m_row: TableRow<Match>) -> StateMsg {
    let m = txn.lazy(m_row);
    let Some(join_name) = &m.join_name else {
        return StateMsg::Waiting {
            usernames: Sides {
                blue: m.create_name.clone(),
                red: m.create_name.clone(),
            },
        };
    };

    let (blue_name, red_name) = if m.create_color == "blue" {
        (&m.create_name, join_name)
    } else {
        (join_name, &m.create_name)
    };

    let moves: Vec<_> = match m.history.is_empty() {
        true => Vec::new(),
        false => m.history.split(',').map(ToOwned::to_owned).collect(),
    };

    let starting_cards: Vec<_> = m
        .starting_cards
        .split(',')
        .map(|x| card_to_pos(x.to_owned()))
        .collect();

    let state = State {
        board: board_from_str(DEFAULT_BOARD),
        table_card: starting_cards[4],
        cards: HashMap::from_iter([
            (PlayerColor::BLUE, [starting_cards[0], starting_cards[1]]),
            (PlayerColor::RED, [starting_cards[2], starting_cards[3]]),
        ]),
        // side card determines starting player
        active_eq_red: CARDS[starting_cards[4]].2 == PlayerColor::RED,
        _p: std::marker::PhantomData::<NamedField>,
    };
    let starting_cards = state.cards();

    let mut state: State = state.translate();
    for m in &moves {
        let (card, from_to) = m.split_once(':').unwrap();
        let from = NamedField::from_str(&from_to[..2]).unwrap();
        let to = NamedField::from_str(&from_to[2..]).unwrap();
        state = state.make_move(card, from, to).unwrap();
    }

    let usernames = Sides {
        blue: blue_name.clone(),
        red: red_name.clone(),
    };

    // check if the current player has lost
    let lost = !state
        .board
        .iter()
        .any(|x| *x == Some(Piece(PlayerTurn::ACTIVE, PieceKind::King)))
        || state.board[22] == Some(Piece(PlayerTurn::WAITING, PieceKind::King));
    let winner = lost.then(|| match state.active_eq_red {
        true => "blue",
        false => "red",
    });

    let state: State<NamedField, PlayerColor> = state.translate();

    let extra = ExtraState {
        indices: Sides {
            blue: (m.create_color == "red") as usize,
            red: (m.create_color == "blue") as usize,
        },
        current_turn: [Color::Blue, Color::Red][state.active_eq_red as usize],
        cards: state.cards(),
        starting_cards,
        moves,
        board: state
            .board
            .iter()
            .map(|p| match p {
                None => '0',
                Some(Piece(PlayerColor::BLUE, PieceKind::Pawn)) => '1',
                Some(Piece(PlayerColor::BLUE, PieceKind::King)) => '2',
                Some(Piece(PlayerColor::RED, PieceKind::Pawn)) => '3',
                Some(Piece(PlayerColor::RED, PieceKind::King)) => '4',
            })
            .collect(),
        winner: winner.unwrap_or("none").to_owned(),
    };

    if winner.is_some() {
        StateMsg::Ended { usernames, extra }
    } else {
        StateMsg::InProgress { usernames, extra }
    }
}
