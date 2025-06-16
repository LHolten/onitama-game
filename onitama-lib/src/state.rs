use std::{collections::HashMap, hash::Hash};

use crate::PieceKind;

pub struct Piece<Player> {
    player: Player,
    kind: PieceKind,
}

pub struct State<Pos, Player> {
    pieces: HashMap<Pos, Piece<Player>>,
    table_card: usize,
    player_cards: HashMap<Player, [usize; 2]>,
    active_eq_red: bool,
}

pub struct PlayerTurn {
    is_active: bool,
}

pub struct PlayerColor {
    // blue starting row is 1, a to e is left to right for blue
    // red is the starting player?
    is_red: bool,
}

pub struct NamedField {
    col: char, // one of a, b, c, d, e
    row: char, // one of 1, 2, 3, 4, 5
}

impl NamedField {
    pub fn range() -> impl Iterator<Item = Self> {
        ('1'..='5').flat_map(|row| ('a'..='e').map(move |col| Self { col, row }))
    }
}

pub struct Perspective {
    col: u8, // left to right for active player
    row: u8, // back to front for active player
}

impl Perspective {
    pub fn range() -> impl Iterator<Item = Self> {
        (0..5).flat_map(|row| (0..5).map(move |col| Self { col, row }))
    }
}

pub trait Translate<To> {
    fn translate(self, active_eq_red: bool) -> To;
}

impl<X> Translate<X> for X {
    fn translate(self, _active_eq_red: bool) -> X {
        self
    }
}

impl Translate<NamedField> for Perspective {
    fn translate(mut self, active_eq_red: bool) -> NamedField {
        if active_eq_red {
            self.col = 4 - self.col;
            self.row = 4 - self.row;
        }
        NamedField {
            col: ('a' as u8 + self.col) as char,
            row: ('5' as u8 - self.row) as char,
        }
    }
}

impl Translate<Perspective> for NamedField {
    fn translate(self, active_eq_red: bool) -> Perspective {
        let mut res = Perspective {
            col: self.col as u8 - 'a' as u8,
            row: '5' as u8 - self.row as u8,
        };
        if active_eq_red {
            res.col = 4 - res.col;
            res.row = 4 - res.row;
        }
        res
    }
}

impl Translate<PlayerColor> for PlayerTurn {
    fn translate(self, active_eq_red: bool) -> PlayerColor {
        PlayerColor {
            is_red: self.is_active == active_eq_red,
        }
    }
}

impl Translate<PlayerTurn> for PlayerColor {
    fn translate(self, active_eq_red: bool) -> PlayerTurn {
        PlayerTurn {
            is_active: self.is_red == active_eq_red,
        }
    }
}

impl<A, B> State<A, B> {
    pub fn translate<X: Eq + Hash, Y: Eq + Hash>(self) -> State<X, Y>
    where
        A: Translate<X>,
        B: Translate<Y>,
    {
        let State {
            pieces,
            active_eq_red,
            table_card,
            player_cards,
        } = self;

        State {
            pieces: pieces
                .into_iter()
                .map(|(k, v)| {
                    (
                        k.translate(active_eq_red),
                        Piece {
                            player: v.player.translate(active_eq_red),
                            kind: v.kind,
                        },
                    )
                })
                .collect(),
            active_eq_red,
            table_card,
            player_cards: player_cards
                .into_iter()
                .map(|(k, v)| (k.translate(active_eq_red), v))
                .collect(),
        }
    }
}
