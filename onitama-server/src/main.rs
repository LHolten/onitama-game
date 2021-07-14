use bincode::{deserialize, serialize};
use native_tls::{Identity, TlsAcceptor, TlsStream};
use onitama_lib::{
    check_move, get_offset, in_card, is_mate, ClientMsg, Piece, PieceKind, Player, ServerMsg,
};
use rand::prelude::SliceRandom;
use rand::{thread_rng, Rng};
use std::convert::TryInto;
use std::fs::File;
use std::io::Read;
use std::mem::swap;
use std::net::{TcpListener, TcpStream};
use std::thread;
use std::time::{Duration, Instant};
use tungstenite::server::accept;
use tungstenite::util::NonBlockingResult;
use tungstenite::{Message, WebSocket};

fn main() {
    let mut file = File::open("certificate.pfx").unwrap();
    let mut identity = vec![];
    file.read_to_end(&mut identity).unwrap();
    let identity = Identity::from_pkcs12(&identity, "grid").unwrap();

    let acceptor = TlsAcceptor::new(identity).unwrap();

    let server = TcpListener::bind("0.0.0.0:9001").unwrap();
    let mut incoming = server.incoming();
    while let Some(Ok(client1)) = incoming.next() {
        if let Some(mut client1) = acceptor.accept(client1).ok().and_then(|s| accept(s).ok()) {
            while let Some(Ok(client2)) = incoming.next() {
                if let Some(client2) = acceptor.accept(client2).ok().and_then(|s| accept(s).ok()) {
                    client1.get_ref().get_ref().set_nonblocking(true).unwrap();
                    let disconnected = loop {
                        match client1.read_message().no_block() {
                            Ok(None) => break false,
                            Ok(_) => {}
                            Err(_) => break true,
                        }
                    };
                    client1.get_ref().get_ref().set_nonblocking(false).unwrap();

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

type WS = WebSocket<TlsStream<TcpStream>>;

fn handle_game(mut conn1: WS, mut conn2: WS) {
    conn1.get_ref().get_ref().set_nodelay(true).unwrap();
    conn2.get_ref().get_ref().set_nodelay(true).unwrap();

    let mut game = new_game();

    while game_turn(&mut game, &mut conn1, &mut conn2).is_some() {
        swap(&mut conn1, &mut conn2);
    }

    close_connection(conn1);
    close_connection(conn2);

    println!("game over");
}

fn close_connection(mut conn: WS) {
    conn.close(None).unwrap();
    while conn.read_message().is_ok() {}
}

fn game_turn(game: &mut ServerMsg, conn_curr: &mut WS, conn_other: &mut WS) -> Option<()> {
    let buf = serialize(&game).unwrap();
    conn_other.write_message(Message::Binary(buf)).ok()?;
    conn_other.write_pending().ok()?;

    mirror_game(game);

    let buf = serialize(&game).unwrap();
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
        conn_curr
            .get_ref()
            .get_ref()
            .set_read_timeout(Some(timeout))
            .unwrap();
        match conn_curr.read_message().ok()? {
            Message::Binary(data) => {
                break deserialize(&data[..]).ok()?;
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
