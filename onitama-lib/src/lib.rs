extern crate serde;

use boolinator::Boolinator;
use serde::{Deserialize, Serialize};
use std::{
    cmp::{max, min},
    ops::Not,
};

#[derive(Debug, Serialize, Deserialize)]
pub struct ClientMsg {
    pub from: usize,
    pub to: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ServerMsg {
    pub board: [Option<Piece>; 25],
    pub cards: [usize; 5],
    pub state: GameState,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum GameState {
    Waiting,
    Playing,
    Win,
    Loss,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum PieceKind {
    Pawn,
    King,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum Player {
    Black,
    White,
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

const KING: Option<Piece> = Some(Piece(Player::White, PieceKind::King));

pub fn is_mate(game: &ServerMsg) -> bool {
    is_check(game, 22) || {
        !(0..25).any(|from| (0..25).any(|to| check_move(game, from, to).is_some()))
    }
}

pub fn check_move(game: &ServerMsg, from: usize, to: usize) -> Option<()> {
    let piece = game.board[from]?;
    (piece.0 == Player::White).as_option()?;
    let other = game.board[to];
    (other.is_none() || other.unwrap().0 == Player::Black).as_option()?;
    let offset = get_offset(to, from)?;
    (in_card(offset, game.cards[0]) || in_card(offset, game.cards[1])).as_option()?;

    // if the king is in check it has to be moved
    let king = game.board.iter().position(|&p| p == KING).unwrap();
    (!is_check(game, king) || from == king).as_option()?;

    match piece.1 {
        PieceKind::Pawn => Some(()),
        PieceKind::King => is_check(game, to).not().as_option(),
    }
}

fn is_check(game: &ServerMsg, from: usize) -> bool {
    is_check_card(game, from, game.cards[3]) || is_check_card(game, from, game.cards[4])
}

fn is_check_card(game: &ServerMsg, from: usize, card: usize) -> bool {
    CARDS[card]
        .iter()
        .map(|&offset| apply_offset(offset, from))
        .flatten()
        .any(|pos| {
            let piece = game.board[pos];
            piece.is_some() && piece.unwrap().0 == Player::Black
        })
}

fn diff(a: usize, b: usize) -> usize {
    max(a, b) - min(a, b)
}

pub fn in_card(offset: usize, card: usize) -> bool {
    CARDS[card].contains(&offset)
}

const CARDS: &[&[usize]] = &[
    &[7, 13, 17],
    &[7, 11, 13],
    &[7, 11, 17],
    &[6, 8, 11, 13],
    &[7, 10, 14],
    &[2, 17],
    &[6, 8, 16, 18],
    &[7, 16, 18],
    &[5, 9, 16, 18],
    &[6, 8, 17],
    &[6, 10, 18],
    &[8, 14, 16],
    &[6, 11, 13, 18],
    &[8, 11, 13, 16],
    &[6, 13, 16],
    &[8, 11, 18],
];
