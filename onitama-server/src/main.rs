use onitama_lib::state::{NamedField, Piece, PlayerColor, State};
use onitama_lib::{
    board_from_str, card_to_pos, Cards, Color, ExtraState, LitamaMsg, PieceKind, Sides, StateMsg,
    DEFAULT_BOARD,
};
use rand::seq::SliceRandom;
use rand::Rng;
use rust_query::migration::{schema, Config};
use rust_query::{
    Database, FromExpr, IntoExpr, LocalClient, TableRow, Transaction, TransactionMut, Update,
};
use std::error::Error;
use std::iter::FromIterator;
use std::mem::swap;
use std::str::FromStr;
use std::time::{Duration, Instant};
use uuid::Uuid;

use simple_websockets::{Event, Message, Responder};
use std::collections::HashMap;

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

        // either "red" or "blue", this is secret until the second player joins
        // "blue" is always the starting player.
        pub create_color: String,
        // concatenation of blue1,blue2,red1,red2,side
        pub starting_cards: String,
    }
}
use v0::*;

impl<'t> IntoExpr<'t, Schema> for Uuid {
    type Typ = String;
    fn into_expr(self) -> rust_query::Expr<'t, Schema, Self::Typ> {
        self.to_string().into_expr()
    }
}

pub struct Client {
    responder: Responder,
    // these are game_ids
    subscriptions: Vec<String>,
}

impl Client {
    pub fn send_msg(&self, msg: LitamaMsg) {
        todo!()
    }
}

fn main() {
    // listen for WebSockets on port 8080:
    let event_hub = simple_websockets::launch(8080).expect("failed to listen on port 8080");
    // map between client ids and the client's `Responder`:
    let mut clients: HashMap<u64, Client> = HashMap::new();

    let mut db_client = LocalClient::try_new().unwrap();
    let db: Database<Schema> = db_client
        .migrator(Config::open_in_memory())
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
                        subscriptions: vec![],
                    },
                );
            }
            Event::Disconnect(client_id) => {
                println!("Client #{} disconnected.", client_id);
                // remove the disconnected client from the clients map:
                clients.remove(&client_id);
            }
            Event::Message(client_id, message) => {
                let txn = db_client.transaction_mut(&db);
                if let Err(err) = handle_message(message.clone(), client_id, &mut clients, txn) {
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
    mut txn: TransactionMut<Schema>,
) -> Result<(), Box<dyn Error>> {
    let client = clients.get_mut(&client_id).unwrap();

    let Message::Text(msg) = message else {
        return Err("recieved binary!".into());
    };
    let parts: Vec<_> = msg.split(" ").collect();
    let cmd = *parts.get(0).ok_or("expected command")?;

    if cmd == "create" {
        let username = *parts.get(1).ok_or("expected username")?;

        let match_id = uuid::Uuid::new_v4();
        let blue_token = uuid::Uuid::new_v4();
        let red_token = uuid::Uuid::new_v4();
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

        txn.commit();
        return Ok(());
    }

    // all other commands require a valid match_id
    let match_id = *parts.get(1).ok_or("expected match_id")?;
    let m_row = txn
        .query_one(Match::unique(match_id))
        .ok_or("match does not exist")?;

    match cmd {
        "join" => {
            let username = *parts.get(2).ok_or("expected username")?;

            let m: Match!(join_token, join_name) = txn.query_one(FromExpr::from_expr(m_row));

            if m.join_name.is_some() {
                return Err("match is already joined".into());
            }

            txn.update_ok(
                m_row,
                Match {
                    join_name: Update::set(Some(username)),
                    ..Default::default()
                },
            );

            client.send_msg(LitamaMsg::Join {
                match_id: match_id.to_owned(),
                token: m.join_token,
                index: 1,
            });
        }
        "state" => {
            client.send_msg(LitamaMsg::State {
                match_id: match_id.to_owned(),
                state: read_state(&txn, m_row),
            });
        }
        "move" => {
            let token = *parts.get(2).ok_or("expected token")?;
            let card = *parts.get(3).ok_or("expected card")?;
            let movee = *parts.get(4).ok_or("expected move")?;
            if movee.len() != 4 || !movee.is_ascii() {
                return Err("move has unexpected len or is not ascii".into());
            }

            let m: Match!(
                create_token,
                join_token,
                create_color,
                history,
                starting_cards
            ) = txn.query_one(FromExpr::from_expr(m_row));

            client.send_msg(LitamaMsg::Move {
                match_id: match_id.to_owned(),
            });

            for other in clients.values() {
                other.send_msg(LitamaMsg::State {
                    match_id: match_id.to_owned(),
                    state: read_state(&txn, m_row), // TODO: maybe optimize this?
                });
            }
        }
        "spectate" => {
            client.subscriptions.push(match_id.to_owned());

            client.send_msg(LitamaMsg::Spectate {
                match_id: match_id.to_owned(),
            });

            client.send_msg(LitamaMsg::State {
                match_id: match_id.to_owned(),
                state: read_state(&txn, m_row),
            });
        }
        _ => return Err("unknown command".into()),
    };

    txn.commit();
    Ok(())
}

pub fn read_state<'a>(txn: &Transaction<'a, Schema>, m_row: TableRow<'a, Match>) -> StateMsg {
    let m: Match!(
        create_name,
        join_name,
        history,
        create_color,
        starting_cards
    ) = txn.query_one(FromExpr::from_expr(m_row));

    let Some(join_name) = m.join_name else {
        return StateMsg::Waiting {
            usernames: Sides {
                blue: m.create_name.clone(),
                red: m.create_name,
            },
        };
    };

    let (blue_name, red_name) = if m.create_color == "blue" {
        (m.create_name, join_name)
    } else {
        (join_name, m.create_name)
    };

    let moves: Vec<_> = m.history.split(',').map(ToOwned::to_owned).collect();
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
        active_eq_red: false,
        _p: std::marker::PhantomData::<NamedField>,
    };
    let starting_cards = state.cards();

    let mut state: State = state.translate();
    for m in &moves {
        let (card, from_to) = m.split_once(':').unwrap();
        let from = NamedField::from_str(&from_to[..2]).unwrap();
        let to = NamedField::from_str(&from_to[2..]).unwrap();
        state.make_move(card, from, to).unwrap();
    }

    let state: State<NamedField, PlayerColor> = state.translate();

    StateMsg::InProgress {
        usernames: Sides {
            blue: blue_name,
            red: red_name,
        },
        extra: ExtraState {
            indices: Sides {
                blue: (m.create_color == "red") as usize,
                red: (m.create_color == "blue") as usize,
            },
            current_turn: [Color::Blue, Color::Red][moves.len() % 2],
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
            winner: "none".to_owned(),
        },
    }
}

// fn handle_game(mut conn1: WS, mut conn2: WS) {
//     conn1.get_ref().set_nodelay(true).unwrap();
//     conn2.get_ref().set_nodelay(true).unwrap();

//     let mut game = new_game();

//     while game_turn(&mut game, &mut conn1, &mut conn2).is_some() {
//         swap(&mut conn1, &mut conn2);
//     }

//     close_connection(conn1);
//     close_connection(conn2);

//     println!("game over");
// }

// fn close_connection(mut conn: WS) {
//     conn.close(None).unwrap();
//     while conn.read_message().is_ok() {}
// }

// fn game_turn(game: &mut ServerMsg, conn_curr: &mut WS, conn_other: &mut WS) -> Option<()> {
//     let buf = serialize(&game).unwrap();
//     conn_other.write_message(Message::Binary(buf)).ok()?;
//     conn_other.write_pending().ok()?;

//     mirror_game(game);

//     let buf = serialize(&game).unwrap();
//     conn_curr.write_message(Message::Binary(buf)).ok()?;
//     conn_curr.write_pending().ok()?;

//     if is_mate(game) {
//         return None;
//     }

//     let now = Instant::now();

//     let action: ClientMsg = loop {
//         let timeout = game.timers[0].saturating_sub(now.elapsed());
//         if timeout.is_zero() {
//             return None;
//         }
//         conn_curr.get_ref().set_read_timeout(Some(timeout)).unwrap();
//         match conn_curr.read_message().ok()? {
//             Message::Binary(data) => {
//                 break deserialize(&data[..]).ok()?;
//             }
//             Message::Ping(_) => conn_curr.write_pending().ok()?,
//             _ => return None,
//         }
//     };

//     game.timers[0] = game.timers[0].saturating_sub(now.elapsed()) + Duration::from_secs(2);

//     println!("got action");

//     check_move(game, action.from, action.to)?;

//     let offset = get_offset(action.to, action.from).unwrap();
//     let index = match (
//         in_card(offset, game.cards[0]),
//         in_card(offset, game.cards[1]),
//     ) {
//         (true, true) => thread_rng().gen::<bool>() as usize,
//         (true, false) => 0,
//         (false, true) => 1,
//         (false, false) => unreachable!(),
//     };

//     game.cards.swap(index, 2);
//     game.board[action.to] = game.board[action.from].take();
//     flip_player(&mut game.turn);

//     Some(())
// }

// fn mirror_game(game: &mut ServerMsg) {
//     game.board.reverse();
//     for piece in game.board.iter_mut().flatten() {
//         flip_player(&mut piece.0);
//     }
//     game.cards.reverse();
//     flip_player(&mut game.turn);
//     game.timers.reverse();
// }

// fn flip_player(player: &mut Player) {
//     if *player == Player::You {
//         *player = Player::Other
//     } else {
//         *player = Player::You
//     }
// }

// fn new_game() -> ServerMsg {
//     let mut board = Vec::default();
//     board.extend_from_slice(&home_row(Player::Other));
//     for _ in 0..15 {
//         board.push(None);
//     }
//     board.extend_from_slice(&home_row(Player::You));

//     let cards: [usize; 16] = (0..16).collect::<Vec<usize>>().try_into().unwrap();

//     ServerMsg {
//         board: board.try_into().unwrap(),
//         cards: cards
//             .choose_multiple(&mut thread_rng(), 5)
//             .copied()
//             .collect::<Vec<usize>>()
//             .try_into()
//             .unwrap(),
//         timers: [Duration::from_secs(60 * 3); 2],
//         turn: Player::Other,
//     }
// }
