use dominator::{
    Dom, DomBuilder,
    __internal::{HtmlElement, SvgElement},
    class,
    events::MouseDown,
    html, svg,
};
use futures_signals::signal::{Signal, SignalExt};
use once_cell::sync::Lazy;
use onitama_lib::{get_offset, in_card, ClientMsg, GameState, Piece, PieceKind, Player};
use rmp_serde::Serializer;
use serde::Serialize;
use web_sys::WebSocket;

use crate::Game;

#[derive(Clone, Copy, PartialEq)]
pub enum Overlay {
    Highlight,
    Dot,
}

static OVERLAY_CLASS: Lazy<String> = Lazy::new(|| {
    class! {
        .style("position", "absolute")
    }
});

impl Game {
    pub fn render_square(&self, pos: usize, socket: &WebSocket) -> Dom {
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

        let app = self.clone();
        let socket_clone = socket.clone();

        html!("span", {
            .class(if pos % 2 == 1 {
                &*SPAN_DARK
            } else {
                &*SPAN_LIGHT
            })
            .event(move |_: MouseDown|{
                let selected = app.selected.get();
                let square = app.board[pos].get();
                if app.state.get() != GameState::Playing {
                    return ;
                }
                if selected != Some(pos) && square.is_some() && square.unwrap().0 == Player::White {
                    app.selected.set(Some(pos));
                } else if app.calculate_overlay(pos) == Some(Overlay::Dot) {
                    app.state.set(GameState::Waiting);

                    let from = selected.unwrap();
                    app.selected.set(None);

                    app.board[pos].set_neq(app.board[from].get());
                    app.board[from].set_neq(None);

                    let mut buf = Vec::new();
                    let msg = ClientMsg { from, to: pos };
                    msg.serialize(&mut Serializer::new(&mut buf)).unwrap();
                    socket_clone.send_with_u8_array(&buf).unwrap();
                } else {
                    app.selected.set(None);
                }
            })
            .apply(|mut dom| {
                for player in [Player::Black, Player::White] {
                    for kind in [PieceKind::Pawn, PieceKind::King] {
                        let piece = Piece(player, kind);
                        dom = dom.child(
                            piece_render(&piece)
                            .class(&*OVERLAY_CLASS)
                            .visible_signal(
                                self.board[pos].signal().map(move |p|p == Some(piece))
                            ).into_dom()
                        )
                    }
                };
                for overlay in [Overlay::Highlight, Overlay::Dot] {
                    dom = dom.child(
                        overlay.render().class(&*OVERLAY_CLASS)
                        .visible_signal(
                            self.get_overlay(pos).map(move |o|o==Some(overlay))
                        ).into_dom()
                    )
                }
                dom
            })
        })
    }

    fn get_overlay(&self, pos: usize) -> impl Signal<Item = Option<Overlay>> {
        let app = self.clone();
        self.selected
            .signal_ref(move |_| app.calculate_overlay(pos))
    }

    fn calculate_overlay(&self, pos: usize) -> Option<Overlay> {
        let (card1, card2) = (&self.cards[0], &self.cards[1]);
        let square = &self.board[pos];
        let offset = get_offset(pos, self.selected.get()?)?;
        let possible = in_card(offset, card1.get()) || in_card(offset, card2.get());
        let square = square.get();
        if offset == 12 {
            Some(Overlay::Highlight)
        } else if possible && (square.is_none() || square.unwrap().0 != Player::White) {
            Some(Overlay::Dot)
        } else {
            None
        }
    }
}

fn piece_render(piece: &Piece) -> DomBuilder<HtmlElement> {
    let file = match piece {
        Piece(Player::White, PieceKind::King) => "wB.svg",
        Piece(Player::White, PieceKind::Pawn) => "wP.svg",
        Piece(Player::Black, PieceKind::King) => "bB.svg",
        Piece(Player::Black, PieceKind::Pawn) => "bP.svg",
    };

    DomBuilder::new_html("img")
        .attr("src", file)
        .attr("width", "80")
        .attr("height", "80")
}

impl Overlay {
    fn render(&self) -> DomBuilder<SvgElement> {
        let dom = DomBuilder::new_svg("svg")
            .attr("width", "80")
            .attr("height", "80");
        match self {
            Overlay::Highlight => dom.attr("viewBox", "0 0 1 1").child(svg!("rect", {
                .attr("fill", "none")
                .attr("stroke", "green")
                .attr("stroke-width", "0.1")
                .attr("width", "1")
                .attr("height", "1")
                .attr("stroke-dasharray", "0.5")
                .attr("stroke-dashoffset", "0.25")
                .attr("stroke-opacity", "0.5")
            })),
            Overlay::Dot => dom.attr("viewBox", "-1 -1 2 2").child(svg!("circle", {
                .attr("r", "0.25")
                .attr("fill", "green")
                .attr("fill-opacity", "0.5")
            })),
        }
    }
}
