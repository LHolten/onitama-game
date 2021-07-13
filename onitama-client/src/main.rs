mod board;
mod connection;

use dominator::{class, html, text, Dom};
use futures_signals::signal::{Mutable, Signal, SignalExt};
use once_cell::sync::Lazy;
use onitama_lib::{GameState, Piece, Player, ServerMsg};
use web_sys::WebSocket;

use crate::connection::game_dom;

#[derive(Clone)]
pub struct Game {
    board: Vec<Mutable<Option<Piece>>>,
    cards: [Mutable<usize>; 5], // things that depend on this are updated  with "selected"
    selected: Mutable<Option<usize>>,
    state: Mutable<GameState>,
}

pub fn main() {
    #[cfg(debug_assertions)]
    console_error_panic_hook::set_once();

    dominator::append_dom(&dominator::body(), game_dom("ws://127.0.0.1:9001"));
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
            state: Mutable::new(GameState::Waiting),
        }
    }

    fn render(&self, socket: &WebSocket) -> Dom {
        static TEXT: Lazy<String> = Lazy::new(|| {
            class! {
                .style("color", "white")
            }
        });

        html!("div", {
            .class(class! {
                .style("display", "flex")
                .style("flex-wrap", "wrap-reverse")
                .style("align-items", "center")
                .style("justify-content", "center")
                .style("position", "absolute")
                .style("background", "#161512")
                .style("top", "0")
                .style("right", "0")
                .style("bottom", "0")
                .style("left", "0")
            })
            .child(html!("main", {
                .children((0..5).map(|y|{
                    html!("div", {
                        .children((0..5).map(|x|{
                            self.render_square(y * 5 + x, socket)
                        }))
                    })
                }))
            }))
            .child(html!("div", {
                .class(class!{
                    .style("display", "flex")
                    .style("flex-direction", "column")
                })
                .child(html!("div", {
                    .class(&*TEXT)
                    .text("Opponent cards: ")
                    .text_signal(card_name(&self.cards[4]))
                    .text(", ")
                    .text_signal(card_name(&self.cards[3]))
                }))
                .child(html!("div", {
                    .class(&*TEXT)
                    .text("Table card: ")
                    .text_signal(card_name(&self.cards[2]))
                }))
                .child(html!("div", {
                    .class(&*TEXT)
                    .text("Your cards: ")
                    .text_signal(card_name(&self.cards[1]))
                    .text(", ")
                    .text_signal(card_name(&self.cards[0]))
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
        self.state.set_neq(msg.state);
    }
}

fn card_name(card: &Mutable<usize>) -> impl Signal<Item = &'static str> {
    card.signal().map(|card| CARD_NAMES[card])
}

const CARD_NAMES: &[&str] = &[
    "ox", "boar", "horse", "elephant", "crab", "tiger", "monkey", "crane", "dragon", "mantis",
    "frog", "rabbit", "goose", "rooster", "eel", "cobra",
];
