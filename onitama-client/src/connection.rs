use core::str;
use std::cell::OnceCell;

use crate::App;
use dominator::Dom;
use onitama_lib::{LitamaMsg, ServerMsg, State};
use wasm_bindgen::{prelude::Closure, JsCast, JsValue};
use web_sys::{window, MessageEvent};

thread_local! {
    pub static PLAYER_IDX: OnceCell<usize> = OnceCell::new();
}

pub fn game_dom(url: &str) -> Dom {
    let socket = web_sys::WebSocket::new(url).unwrap();
    socket.set_binary_type(web_sys::BinaryType::Arraybuffer);

    let app = App::new();

    let game_clone = app.game.clone();
    let timestamp_clone = app.timestamp.clone();
    let info_clone = app.info.clone();
    let socket_clone = socket.clone();
    let onmessage = Closure::wrap(Box::new(move |e: MessageEvent| {
        let buf = e.data().as_string().unwrap();

        let msg: LitamaMsg = serde_json::from_str(&buf).unwrap();
        match msg {
            LitamaMsg::Create {
                match_id,
                token,
                index,
            }
            | LitamaMsg::Join {
                match_id,
                token,
                index,
            } => {
                socket_clone
                    .send_with_str(&format!("spectate {match_id}"))
                    .unwrap();
                info_clone.set((match_id, token));
                PLAYER_IDX.with(|x| x.set(index)).unwrap();
            }
            LitamaMsg::State { match_id: _, state } => {
                let (State::InProgress { extra, .. } | State::Ended { extra, .. }) = state else {
                    return;
                };
                let my_color = extra.indices.find(PLAYER_IDX.with(|x| *x.get().unwrap()));

                game_clone.set(ServerMsg::from_state(extra, my_color));
                timestamp_clone.set(window().unwrap().performance().unwrap().now())
            }
            _ => {}
        }
    }) as Box<dyn FnMut(MessageEvent)>);

    socket.set_onmessage(Some(onmessage.as_ref().unchecked_ref()));
    onmessage.forget();

    let done_clone = app.done.clone();
    let onclose = Closure::wrap(Box::new(move |_| {
        done_clone.set(true);
    }) as Box<dyn FnMut(JsValue)>);

    socket.set_onclose(Some(onclose.as_ref().unchecked_ref()));
    onclose.forget();

    let socket_clone = socket.clone();
    let onopen = Closure::wrap(Box::new(move |_| {
        socket_clone.send_with_str("create Player").unwrap();
    }) as Box<dyn FnMut(JsValue)>);
    socket.set_onopen(Some(onopen.as_ref().unchecked_ref()));
    onopen.forget();

    app.render(&socket)
}
