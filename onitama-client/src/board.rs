use std::sync::LazyLock;

use dominator::{
    Dom, DomBuilder,
    __internal::{HtmlElement, SvgElement},
    class,
    events::MouseDown,
    html, svg,
};
use futures_signals::signal::{Signal, SignalExt};

use onitama_lib::{check_move, ClientMsg, Piece, PieceKind, Player};
use web_sys::WebSocket;

use crate::App;

#[derive(Clone, Copy, PartialEq)]
pub enum Overlay {
    Highlight,
    Dot,
}

static OVERLAY_CLASS: LazyLock<String> = LazyLock::new(|| {
    class! {
        .style("position", "absolute")
    }
});

impl App {
    pub fn render_square(&self, pos: usize, socket: &WebSocket) -> Dom {
        static SPAN_DARK: LazyLock<String> = LazyLock::new(|| {
            class! {
                .style("display", "inline-block")
                .style("vertical-align", "bottom")
                .style("background", "#b58863")
                .style("width", "80px")
                .style("height", "80px")
            }
        });

        static SPAN_LIGHT: LazyLock<String> = LazyLock::new(|| {
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
                if g.turn != Player::You {
                    return ;
                }
                if from != Some(pos) && square.is_some() && square.unwrap().0 == Player::You {
                    selected.set(Some(pos));
                } else if from.is_some() && check_move(&mut*g, from.unwrap(), pos).is_some() {
                    selected.set(None);
                    g.turn = Player::Other;

                    let msg = ClientMsg { from: from.unwrap(), to: pos };
                    let buf = serde_json::to_string(&msg).unwrap();
                    socket_clone.send_with_str(&buf).unwrap();
                } else {
                    selected.set(None);
                }
            })
            .apply(|mut dom| {
                for player in [Player::Other, Player::You] {
                    for kind in [PieceKind::Pawn, PieceKind::King] {
                        let piece = Piece(player, kind);
                        dom = dom.child(
                            piece_render(&piece)
                            .class(&*OVERLAY_CLASS)
                            .visible_signal(
                                self.game.signal_ref(move |g| {
                                    g.board[pos] == Some(piece)
                                }).dedupe()
                            ).into_dom()
                        )
                    }
                };
                for overlay in [Overlay::Highlight, Overlay::Dot] {
                    dom = dom.child(
                        overlay.render().class(&*OVERLAY_CLASS)
                        .visible_signal(
                            self.get_overlay(pos).map(move |o|o==Some(overlay)).dedupe()
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
            let mut game = game.lock_mut();
            if from == pos {
                Some(Overlay::Highlight)
            } else if check_move(&mut *game, from, pos).is_some() {
                Some(Overlay::Dot)
            } else {
                None
            }
        })
    }
}

fn piece_render(piece: &Piece) -> DomBuilder<HtmlElement> {
    let file = match piece {
        Piece(Player::You, PieceKind::King) => "wB.svg",
        Piece(Player::You, PieceKind::Pawn) => "wP.svg",
        Piece(Player::Other, PieceKind::King) => "bB.svg",
        Piece(Player::Other, PieceKind::Pawn) => "bP.svg",
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
