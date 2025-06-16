use std::{collections::HashMap, hash::Hash};

use crate::PieceKind;

pub struct Piece<Player> {
    player: Player,
    kind: PieceKind,
}

pub struct State<Pos, Player> {
    pieces: HashMap<Pos, Piece<Player>>,
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

pub trait Translate<From> {
    fn translate(val: From, active_eq_red: bool) -> Self;
}

impl<X> Translate<X> for X {
    fn translate(val: X, _active_eq_red: bool) -> Self {
        val
    }
}

impl Translate<NamedField> for Perspective {
    fn translate(val: NamedField, active_eq_red: bool) -> Self {
        let mut res = Self {
            col: val.col as u8 - 'a' as u8,
            row: '5' as u8 - val.row as u8,
        };
        if active_eq_red {
            res.col = 4 - res.col;
            res.row = 4 - res.row;
        }
        res
    }
}

impl Translate<Perspective> for NamedField {
    fn translate(mut val: Perspective, active_eq_red: bool) -> Self {
        if active_eq_red {
            val.col = 4 - val.col;
            val.row = 4 - val.row;
        }
        Self {
            col: ('a' as u8 + val.col) as char,
            row: ('5' as u8 - val.row) as char,
        }
    }
}

impl Translate<PlayerColor> for PlayerTurn {
    fn translate(val: PlayerColor, active_eq_red: bool) -> Self {
        Self {
            is_active: val.is_red == active_eq_red,
        }
    }
}

impl Translate<PlayerTurn> for PlayerColor {
    fn translate(val: PlayerTurn, active_eq_red: bool) -> Self {
        Self {
            is_red: val.is_active == active_eq_red,
        }
    }
}

impl<A, B> State<A, B> {
    pub fn translate<X, Y>(self) -> State<X, Y>
    where
        X: Translate<A> + Eq + Hash,
        Y: Translate<B>,
    {
        let State {
            pieces,
            active_eq_red,
        } = self;

        State {
            pieces: pieces
                .into_iter()
                .map(|(k, v)| {
                    (
                        X::translate(k, active_eq_red),
                        Piece {
                            player: Y::translate(v.player, active_eq_red),
                            kind: v.kind,
                        },
                    )
                })
                .collect(),
            active_eq_red,
        }
    }
}
