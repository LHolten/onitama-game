mod board;
mod connection;

use dominator::{class, html, Dom};
use futures_signals::signal::Mutable;
use onitama_lib::{Piece, Player, ServerMsg};
use web_sys::WebSocket;

use crate::connection::game_dom;

#[derive(Clone)]
pub struct Game {
    board: Vec<Mutable<Option<Piece>>>,
    cards: [Mutable<usize>; 5], // things that depend on this are updated  with "selected"
    selected: Mutable<Option<usize>>,
    turn: Mutable<Player>,
}

pub fn main() {
    #[cfg(debug_assertions)]
    console_error_panic_hook::set_once();

    dominator::append_dom(&dominator::body(), game_dom("wss://echo.websocket.org"));
}

impl Game {
    fn new() -> Self {
        let mut board = Vec::default();
        for _ in 0..25 {
            board.push(Mutable::new(None));
        }

        Self {
            board,
            cards: Default::default(),
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
