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
    time::Duration,
};

use crate::state::{Perspective, PlayerTurn};

#[derive(Debug, Serialize, Deserialize)]
pub struct ClientMsg {
    pub from: usize,
    pub to: usize,
    pub card: &'static str,
}

impl ClientMsg {
    pub fn format_litama(self, match_id: String, token: String, color: Color) -> String {
        format!(
            "move {match_id} {token} {} {}{}",
            self.card,
            color.format_pos(self.from),
            color.format_pos(self.to)
        )
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
        state: State,
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
pub enum State {
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

impl Color {
    pub fn other(self) -> Color {
        match self {
            Color::Blue => Color::Red,
            Color::Red => Color::Blue,
        }
    }

    pub fn format_pos(self, mut pos: usize) -> String {
        if self == Color::Red {
            pos = 24 - pos;
        }
        let x = ['a', 'b', 'c', 'd', 'e'][pos % 5];
        let y = 5 - (pos / 5);
        format!("{x}{y}")
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ServerMsg {
    pub board: [Option<Piece>; 25],
    pub cards: [usize; 5],
    pub timers: [Duration; 2],
    pub turn: Player,
    pub my_color: Color,
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

impl ServerMsg {
    pub fn from_state(extra: ExtraState, my_color: Color) -> Self {
        let state = crate::state::State {
            pieces: extra
                .board
                .chars()
                .zip(state::NamedField::range())
                .filter_map(|(c, b)| {
                    [
                        None,
                        Some(state::Piece(state::PlayerColor::BLUE, PieceKind::Pawn)),
                        Some(state::Piece(state::PlayerColor::BLUE, PieceKind::King)),
                        Some(state::Piece(state::PlayerColor::RED, PieceKind::Pawn)),
                        Some(state::Piece(state::PlayerColor::RED, PieceKind::King)),
                    ][c.to_digit(10).unwrap() as usize]
                        .map(|x| (b, x))
                })
                .collect(),
            table_card: card_to_pos(extra.cards.side),
            player_cards: HashMap::from_iter([
                (
                    state::PlayerColor::RED,
                    player_card_to_pos(extra.cards.players.red),
                ),
                (
                    state::PlayerColor::BLUE,
                    player_card_to_pos(extra.cards.players.blue),
                ),
            ]),
            active_eq_red: my_color == Color::Red,
        };

        let state: state::State<Perspective, PlayerTurn> = state.translate();
        let mut board = [None; 25];
        for (loc, piece) in state.pieces {
            let idx = Perspective::range().position(|x| x == loc).unwrap();
            let player = [Player::Other, Player::You][piece.0.is_active as usize];
            board[idx] = Some(Piece(player, piece.1))
        }

        Self {
            timers: [Duration::ZERO; 2],
            turn: if extra.current_turn == my_color {
                Player::You
            } else {
                Player::Other
            },
            cards: [
                state.player_cards[&PlayerTurn::ACTIVE][0],
                state.player_cards[&PlayerTurn::ACTIVE][1],
                state.table_card,
                state.player_cards[&PlayerTurn::WAITING][0],
                state.player_cards[&PlayerTurn::WAITING][1],
            ],
            board,
            my_color,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum PieceKind {
    Pawn,
    King,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum Player {
    Other,
    You,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Piece(pub Player, pub PieceKind);

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

const KING: Option<Piece> = Some(Piece(Player::You, PieceKind::King));
const OPP_KING: Option<Piece> = Some(Piece(Player::Other, PieceKind::King));

pub fn is_mate(game: &mut ServerMsg) -> bool {
    let opp_king = game.board.iter().position(|&p| p == OPP_KING).unwrap();
    if let Some(offset) = get_offset(opp_king, 22) {
        if in_card(offset, game.cards[3]) || in_card(offset, game.cards[4]) {
            return true;
        }
    }
    !(0..25).any(|from| (0..25).any(|to| check_move(game, from, to).is_some()))
}

pub fn check_move(game: &mut ServerMsg, from: usize, to: usize) -> Option<&'static str> {
    let piece = game.board[from]?;
    (piece.0 == Player::You).as_option()?;
    let other = game.board[to];
    (other.is_none() || other.unwrap().0 == Player::Other).as_option()?;

    let piece = take(&mut game.board[from]);
    let tmp = replace(&mut game.board[to], piece);
    let check = is_check(game);
    game.board[from] = replace(&mut game.board[to], tmp);

    check.not().as_option()?;

    let offset = get_offset(to, from)?;
    if in_card(offset, game.cards[0]) {
        Some(CARDS[game.cards[0]].0)
    } else if in_card(offset, game.cards[1]) {
        Some(CARDS[game.cards[1]].0)
    } else {
        None
    }
}

fn is_check(game: &ServerMsg) -> bool {
    let king = game.board.iter().position(|&p| p == KING).unwrap();
    is_check_card(game, king, game.cards[3]) || is_check_card(game, king, game.cards[4])
}

fn is_check_card(game: &ServerMsg, from: usize, card: usize) -> bool {
    CARDS[card]
        .1
        .iter()
        .map(|&offset| apply_offset(offset, from))
        .flatten()
        .any(|pos| {
            let piece = game.board[pos];
            piece.is_some() && piece.unwrap().0 == Player::Other
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
