use dominator::{
    Dom, DomBuilder,
    __internal::{HtmlElement, SvgElement},
    class,
    events::MouseDown,
    html, svg,
};
use futures_signals::signal::{Signal, SignalExt};
use once_cell::sync::Lazy;
use onitama_lib::{check_move, ClientMsg, GameState, Piece, PieceKind, Player};
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

        let selected = self.selected.clone();
        let game = self.game.clone();
        let socket_clone = socket.clone();

        html!("span", {
            .class(if pos % 2 == 1 {
                &*SPAN_DARK
            } else {
                &*SPAN_LIGHT
            })
            .event(move |_: MouseDown|{
                let from = selected.get();
                let mut g = game.lock_mut();
                let square = g.board[pos];
                if g.state != GameState::Playing {
                    return ;
                }
                if from != Some(pos) && square.is_some() && square.unwrap().0 == Player::White {
                    selected.set(Some(pos));
                } else if from.is_some() && check_move(&*g, from.unwrap(), pos).is_some() {
                    selected.set(None);

                    g.state = GameState::Waiting;
                    let from = from.unwrap();
                    g.board[pos] = g.board[from].take();

                    let mut buf = Vec::new();
                    let msg = ClientMsg { from, to: pos };
                    msg.serialize(&mut Serializer::new(&mut buf)).unwrap();
                    socket_clone.send_with_u8_array(&buf).unwrap();
                } else {
                    selected.set(None);
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
                                self.game.signal_ref(move |g| {
                                    g.board[pos] == Some(piece)
                                })
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
        let game = self.game.clone();
        self.selected.signal_ref(move |&from| {
            let from = from?;
            let game = game.lock_ref();
            if from == pos {
                Some(Overlay::Highlight)
            } else if check_move(&*game, from, pos).is_some() {
                Some(Overlay::Dot)
            } else {
                None
            }
        })
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
