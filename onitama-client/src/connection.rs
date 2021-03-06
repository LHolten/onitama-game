use crate::App;
use bincode::deserialize;
use dominator::Dom;
use onitama_lib::ServerMsg;
use wasm_bindgen::{prelude::Closure, JsCast, JsValue};
use web_sys::{window, MessageEvent};

pub fn game_dom(url: &str) -> Dom {
    let socket = web_sys::WebSocket::new(url).unwrap();
    socket.set_binary_type(web_sys::BinaryType::Arraybuffer);

    let app = App::new();

    let game_clone = app.game.clone();
    let timestamp_clone = app.timestamp.clone();
    let onmessage = Closure::wrap(Box::new(move |e: MessageEvent| {
        let buf = e.data().dyn_into::<js_sys::ArrayBuffer>().unwrap();
        let array = js_sys::Uint8Array::new(&buf).to_vec();

        let msg: ServerMsg = deserialize(&array[..]).unwrap();
        game_clone.set(msg);
        timestamp_clone.set(window().unwrap().performance().unwrap().now())
    }) as Box<dyn FnMut(MessageEvent)>);

    socket.set_onmessage(Some(onmessage.as_ref().unchecked_ref()));
    onmessage.forget();

    let done_clone = app.done.clone();
    let onclose = Closure::wrap(Box::new(move |_| {
        done_clone.set(true);
    }) as Box<dyn FnMut(JsValue)>);

    socket.set_onclose(Some(onclose.as_ref().unchecked_ref()));
    onclose.forget();

    app.render(&socket)
}
