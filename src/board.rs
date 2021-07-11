use dominator::{class, events::MouseDown, html, svg, Dom};
use futures_signals::signal::Mutable;
use once_cell::sync::Lazy;

use crate::render::{Render, RenderOpt};

pub enum PieceKind {
    Pawn,
    King,
}

pub enum Player {
    Black,
    White,
}

pub struct Piece(pub Player, pub PieceKind);

pub enum Overlay {
    Highlight,
    Dot,
}

#[derive(Clone)]
pub struct Square(pub Mutable<Option<Piece>>);

static OVERLAY_CLASS: Lazy<String> = Lazy::new(|| {
    class! {
        .style("position", "absolute")
    }
});

impl Render for Piece {
    fn render(&self) -> Dom {
        let file = match self {
            Piece(Player::White, PieceKind::King) => "wB.svg",
            Piece(Player::White, PieceKind::Pawn) => "wP.svg",
            Piece(Player::Black, PieceKind::King) => "bB.svg",
            Piece(Player::Black, PieceKind::Pawn) => "bP.svg",
        };

        html!("img", {
            .attr("src", file)
            .attr("width", "80")
            .attr("height", "80")
        })
    }
}

fn overlay_render(pos: usize, selected: Mutable<Option<usize>>) -> Dom {
    html!("div", {
        .class(&*OVERLAY_CLASS)
        .child(svg!("svg", {
            .class(&*OVERLAY_CLASS)
            .visible_signal(selected.signal_ref(move |selected| {
                selected.map(|s| s==pos).unwrap_or(false)
            }))
            .attr("width", "80")
            .attr("height", "80")
            .attr("viewBox", "0 0 1 1")
            .child(svg!("rect", {
                .attr("fill", "none")
                .attr("stroke", "green")
                .attr("stroke-width", "0.1")
                .attr("width", "1")
                .attr("height", "1")
                .attr("stroke-dasharray", "0.5")
                .attr("stroke-dashoffset", "0.25")
                .attr("stroke-opacity", "0.5")
            }))
        }))
        .child(svg!("svg", {
            .class(&*OVERLAY_CLASS)
            .visible(false)
            // .visible_signal(selected.signal_ref(|overlay| {
            //     matches!(overlay, Some(Overlay::Dot))
            // }))
            .attr("width", "80")
            .attr("height", "80")
            .attr("viewBox", "-1 -1 2 2")
            .child(svg!("circle", {
                .attr("r", "0.25")
                .attr("fill", "green")
                .attr("fill-opacity", "0.5")
            }))
        }))
    })
}

impl Square {
    pub fn render(&self, pos: usize, selected: Mutable<Option<usize>>) -> Dom {
        static SPAN_DARK: Lazy<String> = Lazy::new(|| {
            class! {
                .style("display", "inline-block")
                .style("vertical-align", "bottom")
                .style("background", "#b58863")
                .style("width", "80px")
                .style("height", "80px")
            }
        });

        static SPAN_LIGHT: Lazy<String> = Lazy::new(|| {
            class! {
                .style("display", "inline-block")
                .style("vertical-align", "bottom")
                .style("background", "#f0d9b5")
                .style("width", "80px")
                .style("height", "80px")
            }
        });

        html!("span", {
            .class(if pos % 2 == 1 {
                &*SPAN_DARK
            } else {
                &*SPAN_LIGHT
            })
            .child(overlay_render(pos, selected.clone()))
            .child_signal(self.0.signal_ref(RenderOpt::render_opt))
            .event(move |_: MouseDown|{
                let s = selected.get();
                if s.is_some() && s.unwrap() == pos{
                    selected.set(None)
                } else {
                    selected.set(Some(pos));
                }
            })
        })
    }
}
