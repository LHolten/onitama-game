use std::sync::Arc;

use crate::{
    board::{Piece, Player},
    Game,
};
use dominator::Dom;
use rmp_serde::Deserializer;
use serde::{Deserialize, Serialize};
use wasm_bindgen::{prelude::Closure, JsCast};
use web_sys::MessageEvent;

#[derive(Serialize, Deserialize)]
pub struct ClientMsg {
    pub from: usize,
    pub to: usize,
}

#[derive(Serialize, Deserialize)]
pub struct ServerMsg {
    pub board: [Option<Piece>; 25],
    pub cards: [usize; 5],
    pub turn: Player,
}

pub fn game_dom(url: &str) -> Dom {
    let socket = web_sys::WebSocket::new(url).unwrap();
    socket.set_binary_type(web_sys::BinaryType::Arraybuffer);

    let game = Arc::new(Game::new());

    let game_clone = game.clone();
    let onmessage = Closure::wrap(Box::new(move |e: MessageEvent| {
        let buf = e.data().dyn_into::<js_sys::ArrayBuffer>().unwrap();
        let array = js_sys::Uint8Array::new(&buf).to_vec();

        let msg = ServerMsg::deserialize(&mut Deserializer::new(&array[..])).unwrap();
        game_clone.update(msg);
    }) as Box<dyn FnMut(MessageEvent)>);

    socket.set_onmessage(Some(onmessage.as_ref().unchecked_ref()));
    onmessage.forget();

    game.render(&socket)
}
