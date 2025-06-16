mod board;
mod card;
mod connection;

use std::{collections::HashMap, marker::PhantomData, sync::LazyLock, time::Duration};

use dominator::{animation::timestamps, class, html, Dom};
use futures_signals::signal::{Mutable, SignalExt};
use onitama_lib::state::State;
use web_sys::WebSocket;

use crate::{card::render_card, connection::game_dom};

#[derive(Clone)]
pub struct App {
    game: Mutable<ServerMsg>,
    selected: Mutable<Option<usize>>,
    timestamp: Mutable<f64>,
    done: Mutable<bool>,
    info: Mutable<(String, String)>,
}

pub struct ServerMsg {
    pub state: State,
    pub my_turn: bool,
    pub timers: [Duration; 2],
}

pub fn main() {
    #[cfg(debug_assertions)]
    console_error_panic_hook::set_once();

    // dominator::append_dom(
    //     &dominator::body(),
    //     game_dom("wss://server.lucasholten.com:9001"),
    // );
    dominator::append_dom(
        &dominator::body(),
        game_dom("wss://server.lucasholten.com/onitama"),
    );
}

impl App {
    fn new() -> Self {
        Self {
            game: Mutable::new(ServerMsg {
                state: State {
                    board: [None; 25],
                    table_card: 0,
                    cards: HashMap::new(),
                    active_eq_red: true,
                    _p: PhantomData,
                },
                timers: [Duration::ZERO; 2],
                my_turn: false,
            }),
            selected: Mutable::new(None),
            timestamp: Mutable::new(0.),
            done: Mutable::new(false),
            info: Mutable::new(("game_id".to_owned(), "token".to_owned())),
        }
    }

    fn render(&self, socket: &WebSocket) -> Dom {
        static TEXT: LazyLock<String> = LazyLock::new(|| {
            class! {
                .style("color", "white")
                .style("font-size", "xxx-large")
                .style("margin", "10px")
            }
        });

        static HIDDEN: LazyLock<String> = LazyLock::new(|| {
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
                    .class_signal(&*HIDDEN, self.game.signal_ref(|g|g.my_turn))
                    .child(render_card(&self.game, 2, true))
                }))
                .child(html!("div", {
                    .class_signal(&*HIDDEN, self.game.signal_ref(|g|!g.my_turn))
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
                            match g.my_turn {
                                true => "You lost",
                                false => "You won",
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
                            if !game.my_turn {
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
                            if game.my_turn {
                                let elapsed = now.unwrap_or(0.) - start.get();
                                let elapsed = Duration::from_secs_f64((elapsed / 1000.).abs());
                                time_left = time_left.saturating_sub(elapsed);
                            }
                            format_time(time_left)
                    })})
                }))
            }))
            .child(html!("a", {
                .class(&*TEXT)
                .attr_signal("href", {
                    self.info.signal_ref(|(game_id, _)| {
                        format!("https://l0laapk3.github.io/Onitama-client/#{game_id}")
                    })
                })
                .text("join")
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
