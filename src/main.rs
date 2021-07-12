mod board;
mod card;
mod render;

use dominator::{class, clone, events, html, text, Dom};
use futures_signals::signal::{Mutable, SignalExt};
use once_cell::sync::Lazy;
use std::sync::Arc;

use crate::board::{Overlay, Piece, PieceKind, Player};
// use wasm_bindgen::prelude::*;

pub struct App {
    board: Arc<Vec<Mutable<Option<Piece>>>>,
    cards: Vec<Mutable<usize>>, // things that depend on this are updated  with "selected"
    selected: Mutable<Option<usize>>,
}

impl App {
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
            board: Arc::new(board),
            cards: vec![Mutable::new(0), Mutable::new(1)],
            selected: Mutable::new(None),
        }
    }

    fn render(self) -> Dom {
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
                            self.render_square(y * 5 + x)
                        }))
                    })
                }))
            }))
        })
    }
}

pub fn main() {
    // #[cfg(debug_assertions)]
    // console_error_panic_hook::set_once();

    dominator::append_dom(&dominator::body(), App::render(App::new()));
}
