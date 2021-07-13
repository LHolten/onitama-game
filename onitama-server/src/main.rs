use onitama_lib::{get_offset, in_card, ClientMsg, Piece, PieceKind, Player, ServerMsg};
use rand::prelude::SliceRandom;
use rand::thread_rng;
use rmp_serde::{Deserializer, Serializer};
use serde::{Deserialize, Serialize};
use std::convert::TryInto;
use std::mem::swap;
use std::net::{TcpListener, TcpStream};
use tungstenite::server::accept;
use tungstenite::{Message, WebSocket};

fn main() {
    let server = TcpListener::bind("127.0.0.1:9001").unwrap();
    let mut incoming = server.incoming();
    while let Some(client1) = incoming.next() {
        let mut client1 = accept(client1.unwrap()).unwrap();
        let mut client2 = accept(incoming.next().unwrap().unwrap()).unwrap();

        let mut game = new_game();

        while game_turn(&mut game, &mut client1, &mut client2).is_some() {
            swap(&mut client1, &mut client2);
        }

        println!("game over");
    }
}

fn game_turn(
    game: &mut ServerMsg,
    conn_curr: &mut WebSocket<TcpStream>,
    conn_other: &mut WebSocket<TcpStream>,
) -> Option<()> {
    let mut buf = Vec::new();
    game.serialize(&mut Serializer::new(&mut buf)).unwrap();
    conn_other.write_message(Message::Binary(buf)).ok()?;

    mirror_game(game);

    let mut buf = Vec::new();
    game.serialize(&mut Serializer::new(&mut buf)).unwrap();
    conn_curr.write_message(Message::Binary(buf)).ok()?;

    let action: ClientMsg = loop {
        match conn_curr.read_message().ok()? {
            Message::Binary(data) => {
                break ClientMsg::deserialize(&mut Deserializer::new(&data[..])).ok()?;
            }
            Message::Ping(_) => conn_curr.write_pending().ok()?,
            _ => return None,
        }
    };

    println!("got action");

    let piece_from = *game.board.get(action.from)?;
    if piece_from.is_none() || piece_from.unwrap().0 == Player::Black {
        return None;
    }

    let piece_to = *game.board.get(action.to)?;
    if piece_to.is_some() && piece_to.unwrap().0 == Player::White {
        return None;
    }

    let offset = get_offset(action.to, action.from)?;
    if !in_card(offset, game.cards[0]) && !in_card(offset, game.cards[1]) {
        return None;
    }

    // more checks

    game.board[action.to] = game.board[action.from].take();

    Some(())
}

fn mirror_game(game: &mut ServerMsg) {
    game.board.reverse();
    for piece in game.board.iter_mut().flatten() {
        flip_player(&mut piece.0);
    }
    game.cards.reverse();
}

fn flip_player(player: &mut Player) {
    if *player == Player::White {
        *player = Player::Black
    } else {
        *player = Player::White
    }
}

fn new_game() -> ServerMsg {
    let mut board = Vec::default();
    board.extend_from_slice(&home_row(Player::Black));
    for _ in 0..15 {
        board.push(None);
    }
    board.extend_from_slice(&home_row(Player::White));

    let cards: [usize; 16] = (0..16).collect::<Vec<usize>>().try_into().unwrap();

    ServerMsg {
        board: board.try_into().unwrap(),
        cards: cards
            .choose_multiple(&mut thread_rng(), 5)
            .copied()
            .collect::<Vec<usize>>()
            .try_into()
            .unwrap(),
        state: onitama_lib::GameState::Playing,
    }
}

fn home_row(player: Player) -> [Option<Piece>; 5] {
    let pawn = Some(Piece(player, PieceKind::Pawn));
    let king = Some(Piece(player, PieceKind::King));

    [pawn, pawn, king, pawn, pawn]
}
