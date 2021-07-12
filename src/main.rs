mod board;
mod card;
mod connection;

extern crate serde;

use dominator::{class, html, Dom};
use futures_signals::signal::Mutable;
use web_sys::WebSocket;

use crate::{
    board::{Piece, PieceKind, Player},
    connection::{game_dom, ServerMsg},
};

#[derive(Clone)]
pub struct Game {
    board: Vec<Mutable<Option<Piece>>>,
    cards: [Mutable<usize>; 5], // things that depend on this are updated  with "selected"
    selected: Mutable<Option<usize>>,
    turn: Mutable<Player>,
}

impl Game {
    fn new() -> Self {
        let mut board = Vec::default();
        let bP = Some(Piece(Player::Black, PieceKind::Pawn));
        let bK = Some(Piece(Player::Black, PieceKind::King));
        let wP = Some(Piece(Player::White, PieceKind::Pawn));
        let wK = Some(Piece(Player::White, PieceKind::King));
        board.extend_from_slice(&[
            Mutable::new(bP),
            Mutable::new(bP),
            Mutable::new(bK),
            Mutable::new(bP),
            Mutable::new(bP),
        ]);
        for _ in 0..15 {
            board.push(Mutable::new(None));
        }
        board.extend_from_slice(&[
            Mutable::new(wP),
            Mutable::new(wP),
            Mutable::new(wK),
            Mutable::new(wP),
            Mutable::new(wP),
        ]);

        Self {
            board,
            cards: [
                Mutable::new(0),
                Mutable::new(1),
                Mutable::new(2),
                Mutable::new(3),
                Mutable::new(4),
            ],
            selected: Mutable::new(None),
            turn: Mutable::new(Player::White),
        }
    }

    fn render(&self, socket: &WebSocket) -> Dom {
        html!("div", {
            .class(class! {
                .style("display", "flex")
                .style("align-items", "center")
                .style("justify-content", "center")
                .style("position", "absolute")
                .style("background", "#161512")
                .style("top", "0")
                .style("right", "0")
                .style("bottom", "0")
                .style("left", "0")
            })
            .child(html!("div", {
                .children((0..5).map(|y|{
                    html!("div", {
                        .children((0..5).map(|x|{
                            self.render_square(y * 5 + x, socket)
                        }))
                    })
                }))
            }))
        })
    }

    pub fn update(&self, msg: ServerMsg) {
        for (from, to) in msg.board.iter().zip(self.board.iter()) {
            to.set_neq(*from);
        }
        for (from, to) in msg.cards.iter().zip(self.cards.iter()) {
            to.set_neq(*from);
        }
        self.turn.set_neq(msg.turn)
    }
}

pub fn main() {
    #[cfg(debug_assertions)]
    console_error_panic_hook::set_once();

    dominator::append_dom(&dominator::body(), game_dom("wss://echo.websocket.org"));
}
