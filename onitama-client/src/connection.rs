use std::sync::Arc;

use crate::Game;
use dominator::Dom;
use onitama_lib::ServerMsg;
use rmp_serde::Deserializer;
use serde::Deserialize;
use wasm_bindgen::{prelude::Closure, JsCast};
use web_sys::MessageEvent;

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
