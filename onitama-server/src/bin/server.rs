use onitama_lib::{Piece, PieceKind, Player, ServerMsg};
use std::convert::TryInto;
use std::net::TcpListener;
use std::thread::spawn;
use tungstenite::server::accept;

fn main() {
    let server = TcpListener::bind("127.0.0.1:9001").unwrap();
    let mut incoming = server.incoming();
    while let Some(client1) = incoming.next() {
        let client1 = accept(client1.unwrap()).unwrap();
        let client2 = accept(incoming.next().unwrap().unwrap()).unwrap();

        let game = new_game();
    }
    for stream in server.incoming() {
        spawn(move || {
            let mut websocket = accept(stream.unwrap()).unwrap();
            loop {
                let msg = websocket.read_message().unwrap();

                // We do not want to send back ping/pong messages.
                if msg.is_binary() || msg.is_text() {
                    websocket.write_message(msg).unwrap();
                }
            }
        });
    }
}

pub fn new_game() -> ServerMsg {
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
