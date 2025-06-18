use std::{
    array,
    collections::HashMap,
    error::Error,
    hash::Hash,
    marker::PhantomData,
    mem::{swap, take},
    str::FromStr,
};

use crate::{get_offset, in_card, Cards, PieceKind, Sides, CARDS};

#[derive(Clone, Copy, PartialEq)]
pub struct Piece<Player = PlayerTurn>(pub Player, pub PieceKind);

impl Piece {
    pub const ACTIVE_PAWN: Self = Self(PlayerTurn::ACTIVE, PieceKind::Pawn);
    pub const ACTIVE_KING: Self = Self(PlayerTurn::ACTIVE, PieceKind::King);
    pub const WAITING_PAWN: Self = Self(PlayerTurn::WAITING, PieceKind::Pawn);
    pub const WAITING_KING: Self = Self(PlayerTurn::WAITING, PieceKind::King);
}

pub struct State<Pos = Perspective, Player = PlayerTurn> {
    pub board: [Option<Piece<Player>>; 25],
    pub table_card: usize,
    pub cards: HashMap<Player, [usize; 2]>,
    pub active_eq_red: bool,
    pub _p: PhantomData<Pos>,
}

impl State<NamedField, PlayerColor> {
    pub fn cards(&self) -> Cards {
        Cards {
            players: Sides {
                blue: vec![
                    CARDS[self.cards[&PlayerColor::BLUE][0]].0.to_owned(),
                    CARDS[self.cards[&PlayerColor::BLUE][1]].0.to_owned(),
                ],
                red: vec![
                    CARDS[self.cards[&PlayerColor::RED][0]].0.to_owned(),
                    CARDS[self.cards[&PlayerColor::RED][1]].0.to_owned(),
                ],
            },
            side: CARDS[self.table_card].0.to_owned(),
        }
    }
}

// impl Default for State {
//     fn default() -> Self {
//         Self {
//             board: Default::default(),
//             table_card: 0,
//             cards: todo!(),
//             active_eq_red: todo!(),
//             _p: PhantomData,
//         }
//     }
// }

impl State {
    pub fn make_move<X: Translate<Perspective>>(
        mut self,
        card: &str,
        from: X,
        to: X,
    ) -> Result<Self, Box<dyn Error>> {
        let from = from.translate(self.active_eq_red);
        let from = Perspective::range().position(|x| x == from).unwrap();
        let to = to.translate(self.active_eq_red);
        let to = Perspective::range().position(|x| x == to).unwrap();
        if !self.board[from].is_some_and(|x| x.0 == PlayerTurn::ACTIVE) {
            return Err("can only move your own pieces".into());
        }
        if self.board[to].is_some_and(|x| x.0 == PlayerTurn::ACTIVE) {
            return Err("can not move onto your own piece".into());
        }
        self.board[to] = take(&mut self.board[from]);
        let card = CARDS
            .iter()
            .position(|x| x.0 == card)
            .ok_or("unknown card name")?;
        let offset = get_offset(to, from).ok_or("move too far")?;
        if !in_card(offset, card) {
            return Err("invalid move for card".into());
        }
        let have = self
            .cards
            .get_mut(&PlayerTurn::ACTIVE)
            .unwrap()
            .iter_mut()
            .find(|x| **x == card)
            .ok_or("you do not have that card")?;
        swap(have, &mut self.table_card);

        // go to global perspective to easily switch active player
        let mut s: State<NamedField, PlayerColor> = self.translate();
        s.active_eq_red ^= true;

        Ok(s.translate())
    }
}

#[derive(PartialEq, Eq, Hash, Clone, Copy)]
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
    // blue is the starting player?
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

impl std::fmt::Display for NamedField {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}{}", self.col, self.row)
    }
}

impl FromStr for NamedField {
    type Err = Box<dyn Error>;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut chars = s.chars();
        Ok(Self {
            col: chars
                .next()
                .filter(|x| ('a'..='e').contains(x))
                .ok_or("invalid column")?,
            row: chars
                .next()
                .filter(|x| ('1'..='5').contains(x))
                .ok_or("invalid row")?,
        })
    }
}

impl PosRange for NamedField {
    fn range() -> impl Iterator<Item = Self> {
        ('1'..='5').flat_map(|row| ('a'..='e').map(move |col| Self { col, row }))
    }
}

#[derive(PartialEq, Eq, Hash)]
pub struct Perspective {
    pub col: u8, // left to right for active player
    pub row: u8, // back to front for active player
}

impl PosRange for Perspective {
    fn range() -> impl Iterator<Item = Self> {
        (0..5).flat_map(|row| (0..5).map(move |col| Self { col, row }))
    }
}

pub trait Translate<To> {
    fn translate(self, active_eq_red: bool) -> To;
}

pub trait PosRange {
    fn range() -> impl Iterator<Item = Self>;
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
    pub fn translate<X, Y>(self) -> State<X, Y>
    where
        A: Translate<X> + PosRange,
        X: PosRange + PartialEq,
        B: Translate<Y>,
        Y: Eq + Hash,
    {
        let State {
            board,
            active_eq_red,
            table_card,
            cards,
            _p,
        } = self;

        let mut new_board = array::from_fn(|_| None);
        for (pos, piece) in A::range().zip(board) {
            let pos = pos.translate(active_eq_red);
            let new_pos = X::range().position(|x| x == pos).unwrap();
            new_board[new_pos] = piece.map(|b| Piece(b.0.translate(active_eq_red), b.1))
        }

        State {
            board: new_board,
            active_eq_red,
            table_card,
            cards: cards
                .into_iter()
                .map(|(k, v)| (k.translate(active_eq_red), v))
                .collect(),
            _p: PhantomData,
        }
    }
}
