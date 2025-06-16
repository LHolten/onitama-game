pub mod state;

extern crate serde;

use boolinator::Boolinator;
use serde::{Deserialize, Serialize};
use std::{
    cmp::{max, min},
    collections::HashMap,
    convert::TryInto,
    iter::FromIterator,
    mem::{replace, take},
    ops::Not,
};

use crate::state::{NamedField, Perspective, Piece, PlayerColor, PlayerTurn, PosRange, Translate};

#[derive(Debug, Serialize, Deserialize)]
pub struct ClientMsg {
    pub from: usize,
    pub to: usize,
    pub card: &'static str,
}

impl ClientMsg {
    pub fn format_litama(self, match_id: String, token: String, active_eq_red: bool) -> String {
        let from: NamedField = Perspective::range()
            .nth(self.from)
            .unwrap()
            .translate(active_eq_red);
        let to: NamedField = Perspective::range()
            .nth(self.to)
            .unwrap()
            .translate(active_eq_red);

        format!("move {match_id} {token} {} {from}{to}", self.card)
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "messageType")]
#[serde(rename_all = "camelCase")]
#[serde(rename_all_fields = "camelCase")]
pub enum LitamaMsg {
    Create {
        match_id: String,
        token: String,
        index: usize,
    },
    Join {
        match_id: String,
        token: String,
        index: usize,
    },
    State {
        match_id: String,
        #[serde(flatten)]
        state: StateMsg,
    },
    Move {
        match_id: String,
    },
    Spectate {
        match_id: String,
    },
    Error {
        error: String,
        query: String,
    },
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all_fields = "camelCase")]
#[serde(tag = "gameState")]
pub enum StateMsg {
    #[serde(rename = "waiting for player")]
    Waiting { usernames: Sides<String> },
    #[serde(rename = "in progress")]
    InProgress {
        usernames: Sides<String>,
        #[serde(flatten)]
        extra: ExtraState,
    },
    #[serde(rename = "ended")]
    Ended {
        usernames: Sides<String>,
        #[serde(flatten)]
        extra: ExtraState,
    },
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtraState {
    pub indices: Sides<usize>,
    pub current_turn: Color,
    pub cards: Cards,
    pub starting_cards: Cards,
    pub moves: Vec<String>,
    pub board: String,
    pub winner: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Sides<T> {
    pub blue: T,
    pub red: T,
}

impl<T> Sides<T> {
    pub fn get(self, c: Color) -> (T, T) {
        match c {
            Color::Blue => (self.blue, self.red),
            Color::Red => (self.red, self.blue),
        }
    }
}

impl Sides<usize> {
    pub fn find(&self, idx: usize) -> Color {
        if self.blue == idx {
            Color::Blue
        } else {
            assert_eq!(self.red, idx);
            Color::Red
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Cards {
    #[serde(flatten)]
    pub players: Sides<Vec<String>>,
    pub side: String,
}

#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Color {
    Blue,
    Red,
}

pub fn card_to_pos(name: String) -> usize {
    CARDS.iter().position(|c| c.0 == name).unwrap()
}

pub fn player_card_to_pos(name: Vec<String>) -> [usize; 2] {
    name.into_iter()
        .map(card_to_pos)
        .collect::<Vec<_>>()
        .try_into()
        .unwrap()
}

pub fn collect_array<T, const N: usize>(iter: impl IntoIterator<Item = T>) -> [T; N] {
    iter.into_iter()
        .collect::<Vec<_>>()
        .try_into()
        .map_err(|_| "f")
        .unwrap()
}

impl state::State<NamedField, PlayerColor> {
    pub fn from_state(extra: ExtraState) -> Self {
        crate::state::State {
            board: collect_array(extra.board.chars().map(|c| {
                [
                    None,
                    Some(state::Piece(state::PlayerColor::BLUE, PieceKind::Pawn)),
                    Some(state::Piece(state::PlayerColor::BLUE, PieceKind::King)),
                    Some(state::Piece(state::PlayerColor::RED, PieceKind::Pawn)),
                    Some(state::Piece(state::PlayerColor::RED, PieceKind::King)),
                ][c.to_digit(10).unwrap() as usize]
            })),
            table_card: card_to_pos(extra.cards.side),
            cards: HashMap::from_iter([
                (
                    state::PlayerColor::RED,
                    player_card_to_pos(extra.cards.players.red),
                ),
                (
                    state::PlayerColor::BLUE,
                    player_card_to_pos(extra.cards.players.blue),
                ),
            ]),
            active_eq_red: extra.current_turn == Color::Red,
            _p: std::marker::PhantomData::<NamedField>,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PieceKind {
    Pawn,
    King,
}

pub fn get_offset(pos: usize, from: usize) -> Option<usize> {
    let (pos_x, pos_y) = (pos % 5, pos / 5);
    let (from_x, from_y) = (from % 5, from / 5);

    if diff(pos_x, from_x) > 2 || diff(pos_y, from_y) > 2 {
        None
    } else {
        let (offset_x, offset_y) = (2 + pos_x - from_x, 2 + pos_y - from_y);
        Some(offset_y * 5 + offset_x)
    }
}

pub fn apply_offset(offset: usize, from: usize) -> Option<usize> {
    let (offset_x, offset_y) = (offset % 5, offset / 5);
    let (from_x, from_y) = (from % 5, from / 5);

    let (pos_x, pos_y) = (from_x + offset_x, from_y + offset_y);
    if !(2..7).contains(&pos_x) || !(2..7).contains(&pos_y) {
        None
    } else {
        Some((pos_y - 2) * 5 + (pos_x - 2))
    }
}

const KING: Option<Piece> = Some(Piece(PlayerTurn::ACTIVE, PieceKind::King));
const OPP_KING: Option<Piece> = Some(Piece(PlayerTurn::WAITING, PieceKind::King));

// check for mate assuming that there is no check on the active player
pub fn is_mate(game: &mut state::State) -> bool {
    let opp_king = game.board.iter().position(|&p| p == OPP_KING).unwrap();
    if let Some(offset) = get_offset(opp_king, 22) {
        if game.cards[&PlayerTurn::WAITING]
            .iter()
            .any(|c| in_card(offset, *c))
        {
            return true;
        }
    }
    !(0..25).any(|from| (0..25).any(|to| check_move(game, from, to).is_some()))
}

pub fn check_move(game: &mut state::State, from: usize, to: usize) -> Option<&'static str> {
    let piece = game.board[from]?;
    (piece.0 == PlayerTurn::ACTIVE).as_option()?;
    let other = game.board[to];
    (other.is_none() || other.unwrap().0 == PlayerTurn::WAITING).as_option()?;

    let piece = take(&mut game.board[from]);
    let tmp = replace(&mut game.board[to], piece);
    let check = is_check(game);
    game.board[from] = replace(&mut game.board[to], tmp);

    check.not().as_option()?;

    let offset = get_offset(to, from)?;
    game.cards[&PlayerTurn::ACTIVE]
        .iter()
        .find(|x| in_card(offset, **x))
        .map(|c| CARDS[*c].0)
}

fn is_check(game: &state::State) -> bool {
    let king = game.board.iter().position(|&p| p == KING).unwrap();
    game.cards[&PlayerTurn::WAITING]
        .iter()
        .any(|c| is_check_card(game, king, *c))
}

fn is_check_card(game: &state::State, from: usize, card: usize) -> bool {
    CARDS[card]
        .1
        .iter()
        .map(|&offset| apply_offset(offset, from))
        .flatten()
        .any(|pos| {
            let piece = game.board[pos];
            piece.is_some() && piece.unwrap().0 == PlayerTurn::WAITING
        })
}

fn diff(a: usize, b: usize) -> usize {
    max(a, b) - min(a, b)
}

pub fn in_card(offset: usize, card: usize) -> bool {
    CARDS[card].1.contains(&offset)
}

// 0 1 2 3 4  00
// 5 6 7 8 9  00
// 0 1 2 3 4  10
// 5 6 7 8 9  10
// 0 1 2 3 4  20

pub const CARDS: &[(&str, &[usize])] = &[
    ("ox", &[7, 13, 17]),
    ("boar", &[7, 11, 13]),
    ("horse", &[7, 11, 17]),
    ("elephant", &[6, 8, 11, 13]),
    ("crab", &[7, 10, 14]),
    ("tiger", &[2, 17]),
    ("monkey", &[6, 8, 16, 18]),
    ("crane", &[7, 16, 18]),
    ("dragon", &[5, 9, 16, 18]),
    ("mantis", &[6, 8, 17]),
    ("frog", &[6, 10, 18]),
    ("rabbit", &[8, 14, 16]),
    ("goose", &[6, 11, 13, 18]),
    ("rooster", &[8, 11, 13, 16]),
    ("eel", &[6, 13, 16]),
    ("cobra", &[8, 11, 18]),
];
