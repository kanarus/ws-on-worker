use super::Session;
use worker::WebSocket;
use worker::js_sys::Map as JsMap;
use worker::wasm_bindgen::{JsValue, JsCast};
use ohkami::serde::json;


// We can't use `HashMap` or other Rust struct
// to use `WebSocket` as the key.
pub(super) struct SessionMap(JsMap);

impl SessionMap {
    pub(super) fn new() -> Self {
        Self(JsMap::new())
    }

    pub(super) fn set(&mut self, ws: &WebSocket, session: Session) {
        let raw_session = JsValue::from_str(&json::to_string(&session).unwrap());
        self.0.set(ws.as_ref(), &raw_session);
    }

    pub(super) fn get(&self, ws: &WebSocket) -> Option<Session> {
        self.0.get(ws.as_ref())
            .as_string()
            .map(|s| json::from_str(&s).unwrap())
    }

    pub(super) fn remove(&mut self, ws: &WebSocket) {
        self.0.delete(ws.as_ref());
    }

    pub(super) fn sockets(&self) -> impl Iterator<Item = WebSocket> {
        self.0.keys().into_iter()
            .map(|result| result.unwrap().unchecked_into::<worker::worker_sys::web_sys::WebSocket>().into())
    }
    pub(super) fn iter(&self) -> impl Iterator<Item = (WebSocket, Session)> + '_ {
        self.sockets().map(|ws| {
            let session = self.get(&ws).unwrap();
            (ws, session)
        })
    }
}
