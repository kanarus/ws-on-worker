use super::session::Session;
use worker::WebSocket;


pub(super) struct SessionMap(
    Vec<(WebSocket, Session)>
);

impl SessionMap {
    pub(super) fn new() -> Self {
        Self(Vec::new())
    }

    pub(super) fn insert(&mut self, ws: WebSocket, session: Session) {
        self.0.push((ws, session));
    }
    pub(super) fn remove(&mut self, ws: &WebSocket) {
        if let Some(index) = self.index_of(ws) {
            self.0.remove(index);
        }
    }
    fn index_of(&self, ws: &WebSocket) -> Option<usize> {
        self.0.iter().position(|(w, _)| w == ws)
    }

    pub(super) fn get_mut(&mut self, ws: &WebSocket) -> Option<&mut Session> {
        let index = self.index_of(&ws)?;
        let (_, session) = self.0.get_mut(index)?;
        Some(session)
    }

    pub(super) fn iter(&self) -> impl Iterator<Item = &(WebSocket, Session)> {
        self.0.iter()
    }
    pub(super) fn iter_mut(&mut self) -> impl Iterator<Item = &mut (WebSocket, Session)> {
        self.0.iter_mut()
    }
}
