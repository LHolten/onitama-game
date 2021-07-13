mod board;
mod connection;

use dominator::{class, html, Dom};
use futures_signals::signal::{Mutable, Signal};
use once_cell::sync::Lazy;
use onitama_lib::{GameState, ServerMsg};
use web_sys::WebSocket;

use crate::connection::game_dom;

#[derive(Clone)]
pub struct Game {
    game: Mutable<ServerMsg>,
    selected: Mutable<Option<usize>>,
}

pub fn main() {
    #[cfg(debug_assertions)]
    console_error_panic_hook::set_once();

    dominator::append_dom(&dominator::body(), game_dom("ws://127.0.0.1:9001"));
}

impl Game {
    fn new() -> Self {
        Self {
            game: Mutable::new(ServerMsg {
                board: Default::default(),
                cards: Default::default(),
                state: GameState::Waiting,
            }),
            selected: Mutable::new(None),
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
                    .text_signal(self.card_name(4))
                    .text(", ")
                    .text_signal(self.card_name(3))
                }))
                .child(html!("div", {
                    .class(&*TEXT)
                    .text("Table card: ")
                    .text_signal(self.card_name(2))
                }))
                .child(html!("div", {
                    .class(&*TEXT)
                    .text("Your cards: ")
                    .text_signal(self.card_name(1))
                    .text(", ")
                    .text_signal(self.card_name(0))
                }))
            }))
        })
    }

    fn card_name(&self, card_pos: usize) -> impl Signal<Item = &'static str> {
        self.game
            .signal_ref(move |game| CARD_NAMES[game.cards[card_pos]])
    }
}

const CARD_NAMES: &[&str] = &[
    "ox", "boar", "horse", "elephant", "crab", "tiger", "monkey", "crane", "dragon", "mantis",
    "frog", "rabbit", "goose", "rooster", "eel", "cobra",
];
