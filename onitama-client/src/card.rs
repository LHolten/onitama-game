use dominator::{class, html, Dom};
use futures_signals::signal::{Mutable, SignalExt};
use once_cell::sync::Lazy;
use onitama_lib::{in_card, ServerMsg};

pub fn render_card(game: &Mutable<ServerMsg>, card: usize, rotated: bool) -> Dom {
    static CARD: Lazy<String> = Lazy::new(|| {
        class! {
            .style("display", "inline-block")
            .style("margin", "10px")
        }
    });

    html!("div", {
        .class(&*CARD)
        .children((0..5).map(|y|{
            html!("div", {
                .children((0..5).map(|x|{
                    let mut pos = y * 5 + x;
                    if rotated {pos = 24 - pos}
                    render_card_square(game, card, pos)
                }))
            })
        }))
    })
}

fn render_card_square(game: &Mutable<ServerMsg>, card: usize, pos: usize) -> Dom {
    static CARD_YES: Lazy<String> = Lazy::new(|| {
        class! {
            .style("display", "inline-block")
            .style("vertical-align", "bottom")
            .style("background", "green")
            .style("width", "20px")
            .style("height", "20px")
        }
    });

    static CARD_NO: Lazy<String> = Lazy::new(|| {
        class! {
            .style("display", "inline-block")
            .style("vertical-align", "bottom")
            .style("background", "#f0d9b5")
            .style("width", "20px")
            .style("height", "20px")
        }
    });

    html!("div", {
        .class_signal(&*CARD_YES, game.signal_ref(move|g|{g.cards[card]}).dedupe().map(move |card|{
            in_card(pos, card)
        }).dedupe())
        .class_signal(&*CARD_NO, game.signal_ref(move|g|{g.cards[card]}).dedupe().map(move|card|{
            !in_card(pos, card)
        }).dedupe())
    })
}
