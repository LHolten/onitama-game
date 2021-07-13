use onitama_lib::{
    check_move, get_offset, in_card, is_mate, ClientMsg, Piece, PieceKind, Player, ServerMsg,
};
use rand::prelude::SliceRandom;
use rand::{thread_rng, Rng};
use rmp_serde::{Deserializer, Serializer};
use serde::{Deserialize, Serialize};
use std::convert::TryInto;
use std::mem::swap;
use std::net::{TcpListener, TcpStream};
use std::thread;
use std::time::{Duration, Instant};
use tungstenite::server::accept;
use tungstenite::util::NonBlockingResult;
use tungstenite::{Message, WebSocket};

fn main() {
    let server = TcpListener::bind("127.0.0.1:9001").unwrap();
    let mut incoming = server.incoming();
    while let Some(Ok(client1)) = incoming.next() {
        client1.set_nodelay(true).unwrap();
        if let Ok(mut client1) = accept(client1) {
            while let Some(Ok(client2)) = incoming.next() {
                client2.set_nodelay(true).unwrap();
                if let Ok(client2) = accept(client2) {
                    client1.get_ref().set_nonblocking(true).unwrap();
                    let disconnected = loop {
                        match client1.read_message().no_block() {
                            Ok(None) => break false,
                            Ok(_) => {}
                            Err(_) => break true,
                        }
                    };
                    client1.get_ref().set_nonblocking(false).unwrap();

                    if !disconnected {
                        thread::spawn(|| handle_game(client1, client2));
                        break;
                    }
                    client1 = client2;
                }
            }
        }
    }
}

fn handle_game(mut conn1: WebSocket<TcpStream>, mut conn2: WebSocket<TcpStream>) {
    let mut game = new_game();

    while game_turn(&mut game, &mut conn1, &mut conn2).is_some() {
        swap(&mut conn1, &mut conn2);
    }

    close_connection(conn1);
    close_connection(conn2);

    println!("game over");
}

fn close_connection(mut conn: WebSocket<TcpStream>) {
    conn.close(None).unwrap();
    while conn.read_message().is_ok() {}
}

fn game_turn(
    game: &mut ServerMsg,
    conn_curr: &mut WebSocket<TcpStream>,
    conn_other: &mut WebSocket<TcpStream>,
) -> Option<()> {
    let mut buf = Vec::new();
    game.serialize(&mut Serializer::new(&mut buf)).unwrap();
    conn_other.write_message(Message::Binary(buf)).ok()?;
    conn_other.write_pending().ok()?;

    mirror_game(game);

    let mut buf = Vec::new();
    game.serialize(&mut Serializer::new(&mut buf)).unwrap();
    conn_curr.write_message(Message::Binary(buf)).ok()?;
    conn_curr.write_pending().ok()?;

    if is_mate(game) {
        return None;
    }

    let now = Instant::now();

    let action: ClientMsg = loop {
        let timeout = game.timers[0].saturating_sub(now.elapsed());
        if timeout.is_zero() {
            return None;
        }
        conn_curr.get_ref().set_read_timeout(Some(timeout)).unwrap();
        match conn_curr.read_message().ok()? {
            Message::Binary(data) => {
                break ClientMsg::deserialize(&mut Deserializer::new(&data[..])).ok()?;
            }
            Message::Ping(_) => conn_curr.write_pending().ok()?,
            _ => return None,
        }
    };

    game.timers[0] = game.timers[0].saturating_sub(now.elapsed()) + Duration::from_secs(2);

    println!("got action");

    check_move(game, action.from, action.to)?;

    let offset = get_offset(action.to, action.from).unwrap();
    let index = match (
        in_card(offset, game.cards[0]),
        in_card(offset, game.cards[1]),
    ) {
        (true, true) => thread_rng().gen::<bool>() as usize,
        (true, false) => 0,
        (false, true) => 1,
        (false, false) => unreachable!(),
    };

    game.cards.swap(index, 2);
    game.board[action.to] = game.board[action.from].take();
    flip_player(&mut game.turn);

    Some(())
}

fn mirror_game(game: &mut ServerMsg) {
    game.board.reverse();
    for piece in game.board.iter_mut().flatten() {
        flip_player(&mut piece.0);
    }
    game.cards.reverse();
    flip_player(&mut game.turn);
    game.timers.reverse();
}

fn flip_player(player: &mut Player) {
    if *player == Player::You {
        *player = Player::Other
    } else {
        *player = Player::You
    }
}

fn new_game() -> ServerMsg {
    let mut board = Vec::default();
    board.extend_from_slice(&home_row(Player::Other));
    for _ in 0..15 {
        board.push(None);
    }
    board.extend_from_slice(&home_row(Player::You));

    let cards: [usize; 16] = (0..16).collect::<Vec<usize>>().try_into().unwrap();

    ServerMsg {
        board: board.try_into().unwrap(),
        cards: cards
            .choose_multiple(&mut thread_rng(), 5)
            .copied()
            .collect::<Vec<usize>>()
            .try_into()
            .unwrap(),
        timers: [Duration::from_secs(60 * 3); 2],
        turn: Player::Other,
    }
}

fn home_row(player: Player) -> [Option<Piece>; 5] {
    let pawn = Some(Piece(player, PieceKind::Pawn));
    let king = Some(Piece(player, PieceKind::King));

    [pawn, pawn, king, pawn, pawn]
}
