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
                    render_card_square(game, card, pos, rotated)
                }))
            })
        }))
    })
}

fn render_card_square(game: &Mutable<ServerMsg>, card: usize, pos: usize, rotated: bool) -> Dom {
    static CARD_YES: Lazy<String> = Lazy::new(|| card_colour("#4b8c27"));
    static CARD_NO: Lazy<String> = Lazy::new(|| card_colour("#302e2c"));
    static CARD_WHITE: Lazy<String> = Lazy::new(|| card_colour("white"));
    static CARD_BLACK: Lazy<String> = Lazy::new(|| card_colour("black"));

    if pos == 12 {
        html!("div", {
            .class(if rotated {&*CARD_BLACK} else {&*CARD_WHITE})
        })
    } else {
        html!("div", {
            .class_signal(&*CARD_YES, game.signal_ref(move|g|{g.cards[card]}).dedupe().map(move |card|{
                in_card(pos, card)
            }).dedupe())
            .class_signal(&*CARD_NO, game.signal_ref(move|g|{g.cards[card]}).dedupe().map(move|card|{
                !in_card(pos, card)
            }).dedupe())
        })
    }
}

fn card_colour(colour: &str) -> String {
    class! {
        .style("display", "inline-block")
        .style("vertical-align", "bottom")
        .style("background", colour)
        .style("width", "20px")
        .style("height", "20px")
        // .style("border", "1px solid")
    }
}
