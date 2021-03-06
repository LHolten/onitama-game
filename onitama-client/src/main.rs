mod board;
mod card;
mod connection;

use std::time::Duration;

use dominator::{animation::timestamps, class, html, Dom};
use futures_signals::signal::{Mutable, SignalExt};
use once_cell::sync::Lazy;
use onitama_lib::{Player, ServerMsg};
use web_sys::WebSocket;

use crate::{card::render_card, connection::game_dom};

#[derive(Clone)]
pub struct App {
    game: Mutable<ServerMsg>,
    selected: Mutable<Option<usize>>,
    timestamp: Mutable<f64>,
    done: Mutable<bool>,
}

pub fn main() {
    #[cfg(debug_assertions)]
    console_error_panic_hook::set_once();

    dominator::append_dom(
        &dominator::body(),
        game_dom("wss://server.lucasholten.com:9001"),
    );
    // dominator::append_dom(&dominator::body(), game_dom("ws://127.0.0.1:9001"));
}

impl App {
    fn new() -> Self {
        Self {
            game: Mutable::new(ServerMsg {
                board: Default::default(),
                cards: Default::default(),
                turn: Player::Other,
                timers: [Duration::ZERO; 2],
            }),
            selected: Mutable::new(None),
            timestamp: Mutable::new(0.),
            done: Mutable::new(false),
        }
    }

    fn render(&self, socket: &WebSocket) -> Dom {
        static TEXT: Lazy<String> = Lazy::new(|| {
            class! {
                .style("color", "white")
                .style("font-size", "xxx-large")
                .style("margin", "10px")
            }
        });

        static HIDDEN: Lazy<String> = Lazy::new(|| {
            class! {
                .style("visibility", "hidden")
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
            .child(html!("div", {
                .class(class!{
                    .style("display", "flex")
                    .style("flex-direction", "column")
                })
                .child(html!("div", {
                    .class_signal(&*HIDDEN, self.game.signal_ref(|g|g.turn == Player::You))
                    .child(render_card(&self.game, 2, true))
                }))
                .child(html!("div", {
                    .class_signal(&*HIDDEN, self.game.signal_ref(|g|g.turn == Player::Other))
                    .child(render_card(&self.game, 2, false))
                }))
            }))
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
                        .visible_signal(self.done.signal().dedupe())
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
                    .text_signal({
                        let start = self.timestamp.clone();
                        let game = self.game.clone();
                        timestamps().map(move |now|{
                            let game = game.lock_ref();
                            let mut time_left = game.timers[1];
                            if game.turn == Player::Other {
                                let elapsed = now.unwrap_or(0.) - start.get();
                                let elapsed = Duration::from_secs_f64((elapsed / 1000.).abs());
                                time_left = time_left.saturating_sub(elapsed);
                            }
                            format_time(time_left)
                    })})
                }))
                .child(html!("div", {
                    .child(render_card(&self.game, 4, true))
                    .child(render_card(&self.game, 3, true))
                }))
                .child(html!("div", {
                    .child(render_card(&self.game, 0, false))
                    .child(render_card(&self.game, 1, false))
                }))
                .child(html!("div", {
                    .class(&*TEXT)
                    .text_signal({
                        let start = self.timestamp.clone();
                        let game = self.game.clone();
                        timestamps().map(move |now|{
                            let game = game.lock_ref();
                            let mut time_left = game.timers[0];
                            if game.turn == Player::You {
                                let elapsed = now.unwrap_or(0.) - start.get();
                                let elapsed = Duration::from_secs_f64((elapsed / 1000.).abs());
                                time_left = time_left.saturating_sub(elapsed);
                            }
                            format_time(time_left)
                    })})
                }))
            }))
        })
    }
}

fn format_time(time: Duration) -> String {
    let time = time.as_secs();
    let mut res = String::with_capacity(4);
    res.push(to_decimal(time / 60));
    res.push(':');
    res.push(to_decimal(time % 60 / 10));
    res.push(to_decimal(time % 60));
    res
}

fn to_decimal(num: u64) -> char {
    "0123456789".chars().nth(num as usize % 10).unwrap()
}
