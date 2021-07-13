mod board;
mod connection;

use dominator::{class, html, text_signal, Dom};
use futures_signals::signal::{Mutable, Signal};
use once_cell::sync::Lazy;
use onitama_lib::{Player, ServerMsg};
use web_sys::WebSocket;

use crate::connection::game_dom;

#[derive(Clone)]
pub struct App {
    game: Mutable<ServerMsg>,
    selected: Mutable<Option<usize>>,
    done: Mutable<bool>,
}

pub fn main() {
    #[cfg(debug_assertions)]
    console_error_panic_hook::set_once();

    dominator::append_dom(&dominator::body(), game_dom("ws://127.0.0.1:9001"));
}

impl App {
    fn new() -> Self {
        Self {
            game: Mutable::new(ServerMsg {
                board: Default::default(),
                cards: Default::default(),
                turn: Player::Other,
            }),
            selected: Mutable::new(None),
            done: Mutable::new(false),
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
                .child(html!("div", {
                    .class(class!{
                        .style("position", "absolute")
                        .style("width", "400px")
                        .style("height", "400px")
                        .style("display", "flex")
                        .style("align-items", "center")
                        .style("justify-content", "center")
                    })
                    .child(html!("div", {
                        .class(class!{
                            .style("background", "white")
                            .style("border", "solid")
                        })
                        .text_signal(self.game.signal_ref(move |g| {
                            match g.turn {
                                Player::You => "You lost",
                                Player::Other => "You won",
                            }
                        }))
                        .visible_signal(self.done.signal())
                    }))
                }))
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
