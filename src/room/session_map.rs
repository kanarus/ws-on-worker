use super::session::Session;
use worker::WebSocket;
use worker::worker_sys::web_sys::WebSocket as JsWebSocket;
use worker::js_sys::{Map as JsMap, Array as JsArray};
use worker::wasm_bindgen::{JsValue, JsCast};


pub(super) struct SessionMap {
    websockets: JsMap,
    sessions:   Vec<Session>,
}

impl SessionMap {
    pub(super) fn new() -> Self {
        Self {
            websockets: JsMap::new(),
            sessions:   Vec::new()
        }
    }

    pub(super) fn insert(&mut self, ws: WebSocket, session: Session) {
        self.websockets.set(ws.as_ref(), &JsValue::from_f64(self.websockets.size() as _));
        self.sessions.push(session);
    }
    fn index_of(&self, ws: &WebSocket) -> Option<usize> {
        let index = self.websockets
            .entries().into_iter()
            .map(|pair| JsArray::from(&pair.unwrap()))
            .find(|p| p.get(0) == ****ws.as_ref())?
            .get(1).as_f64()? as usize;
        Some(index)
    }

    pub(super) fn get(&self, ws: &WebSocket) -> Option<&Session> {
        let index = self.index_of(&ws)?;
        self.sessions.get(index)
    }
    pub(super) fn get_mut(&mut self, ws: &WebSocket) -> Option<&mut Session> {
        let index = self.index_of(&ws)?;
        self.sessions.get_mut(index)
    }

    pub(super) fn remove(&mut self, ws: &WebSocket) {
        if let Some(index) = self.index_of(ws) {
            self.sessions.remove(index);
        }
    }

    pub(super) fn websockets(&self) -> impl Iterator<Item = WebSocket> {
        self.websockets.keys().into_iter().map(|js| js.unwrap()
            .dyn_into::<JsWebSocket>()
            .expect("can't convert to web_sys::WebSocket")
            .into()
        )
    }
    pub(super) fn iter(&self) -> impl Iterator<Item = &Session> {
        self.websockets().map(|ws| self.get(&ws).unwrap())
    }
}
