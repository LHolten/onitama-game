use std::cmp::{max, min};

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
