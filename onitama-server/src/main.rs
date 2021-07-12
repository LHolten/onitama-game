use onitama_lib::{Piece, PieceKind, Player, ServerMsg};
use rmp_serde::Serializer;
use serde::Serialize;
use std::convert::TryInto;
use std::net::TcpListener;
use tungstenite::server::accept;
use tungstenite::Message;

fn main() {
    let server = TcpListener::bind("127.0.0.1:9001").unwrap();
    let mut incoming = server.incoming();
    while let Some(client1) = incoming.next() {
        let mut client1 = accept(client1.unwrap()).unwrap();
        let mut client2 = accept(incoming.next().unwrap().unwrap()).unwrap();

        let mut game = new_game();

        let mut buf = Vec::new();
        game.serialize(&mut Serializer::new(&mut buf)).unwrap();
        client1.write_message(Message::Binary(buf)).unwrap();

        mirror_game(&mut game);

        let mut buf = Vec::new();
        game.serialize(&mut Serializer::new(&mut buf)).unwrap();
        client2.write_message(Message::Binary(buf)).unwrap();
    }
}

fn mirror_game(game: &mut ServerMsg) {
    game.board.reverse();
    for piece in game.board.iter_mut().flatten() {
        flip_player(&mut piece.0);
    }
    flip_player(&mut game.turn);
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

    ServerMsg {
        board: board.try_into().unwrap(),
        cards: [0; 5],
        turn: Player::White,
    }
}

fn home_row(player: Player) -> [Option<Piece>; 5] {
    let pawn = Some(Piece(player, PieceKind::Pawn));
    let king = Some(Piece(player, PieceKind::King));

    [pawn, pawn, king, pawn, pawn]
}
