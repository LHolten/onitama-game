mod board;
mod render;

use dominator::{class, clone, events, html, text, Dom};
use futures_signals::signal::{Mutable, SignalExt};
use once_cell::sync::Lazy;
use std::sync::Arc;

use crate::{
    board::{Overlay, Piece, PieceKind, Player, Square},
    render::Render,
};
// use wasm_bindgen::prelude::*;

struct App {
    board: Arc<Vec<Square>>,
    cards: Vec<Mutable<String>>,
    selected: Mutable<Option<usize>>,
}

impl App {
    fn new() -> Self {
        let mut board = Vec::default();
        for _ in 0..25 {
            board.push(Square(Mutable::new(Some(Piece(
                Player::Black,
                PieceKind::King,
            )))))
        }
        Self {
            board: Arc::new(board),
            cards: Vec::new(),
            selected: Mutable::new(None),
        }
    }

    fn render(self) -> Dom {
        // static ROOT_CLASS: Lazy<String> = Lazy::new(|| {
        //     class! {
        //         .style("display", "inline-block")
        //         .style("background-color", "black")
        //         .style("padding", "10px")
        //     }
        // });

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
                            self.board[y * 5 + x].render(y * 5 + x, self.selected.clone())
                        }))
                    })
                }))
            }))
        })

        // Overlay::Dot.render()
        // Piece(Player::Black, PieceKind::Pawn).render()

        // .children(&mut [
        //     html!("div", {
        //         .class(&*TEXT_CLASS)
        //         .text_signal(state.counter.signal().map(|x| format!("Counter: {}", x)))
        //     }),

        //     html!("button", {
        //         .class(&*BUTTON_CLASS)
        //         .text("Increase")
        //         .event(clone!(state => move |_: events::Click| {
        //             // Increment the counter
        //             state.counter.replace_with(|x| *x + 1);
        //         }))
        //     }),

        //     html!("button", {
        //         .class(&*BUTTON_CLASS)
        //         .text("Decrease")
        //         .event(clone!(state => move |_: events::Click| {
        //             // Decrement the counter
        //             state.counter.replace_with(|x| *x - 1);
        //         }))
        //     }),

        //     html!("button", {
        //         .class(&*BUTTON_CLASS)
        //         .text("Reset")
        //         .event(clone!(state => move |_: events::Click| {
        //             // Reset the counter to 0
        //             state.counter.set_neq(0);
        //         }))
        //     }),
        // ])
    }
}

pub fn main() {
    // #[cfg(debug_assertions)]
    // console_error_panic_hook::set_once();

    dominator::append_dom(&dominator::body(), App::render(App::new()));
}
