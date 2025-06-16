use std::{collections::HashMap, hash::Hash};

use crate::PieceKind;

#[derive(Clone, Copy)]
pub struct Piece<Player>(pub Player, pub PieceKind);

pub struct State<Pos, Player> {
    pub pieces: HashMap<Pos, Piece<Player>>,
    pub table_card: usize,
    pub player_cards: HashMap<Player, [usize; 2]>,
    pub active_eq_red: bool,
}

#[derive(PartialEq, Eq, Hash)]
pub struct PlayerTurn {
    pub is_active: bool,
}

impl PlayerTurn {
    pub const ACTIVE: Self = Self { is_active: true };
    pub const WAITING: Self = Self { is_active: false };
}

#[derive(PartialEq, Eq, Hash, Clone, Copy)]
pub struct PlayerColor {
    // blue starting row is 1, a to e is left to right for blue
    // red is the starting player?
    pub is_red: bool,
}

impl PlayerColor {
    pub const RED: Self = Self { is_red: true };
    pub const BLUE: Self = Self { is_red: false };
}

#[derive(PartialEq, Eq, Hash)]
pub struct NamedField {
    pub col: char, // one of a, b, c, d, e
    pub row: char, // one of 1, 2, 3, 4, 5
}

impl NamedField {
    pub fn range() -> impl Iterator<Item = Self> {
        ('1'..='5').flat_map(|row| ('a'..='e').map(move |col| Self { col, row }))
    }
}

#[derive(PartialEq, Eq, Hash)]
pub struct Perspective {
    pub col: u8, // left to right for active player
    pub row: u8, // back to front for active player
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
                        Piece(v.0.translate(active_eq_red), v.1),
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
