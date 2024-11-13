use super::session::{ActiveSession, Session};
use worker::WebSocket;


pub(super) struct SessionMap(
    Vec<(WebSocket, Session)>
);

impl SessionMap {
    pub(super) fn new() -> Self {
        Self(Vec::new())
    }

    pub(super) fn insert(&mut self, ws: WebSocket, session: Session) -> worker::Result<()> {
        if let Some(previos_session) = self.get_mut(&ws) {
            *previos_session = session
        } else {
            self.0.push((ws, session));
        }
        Ok(())
    }
    pub(super) fn remove(&mut self, ws: &WebSocket) -> Option<Session> {
        ws.close::<&str>(None, None).ok();
        if let Some(index) = self.index_of(ws) {
            Some(self.0.remove(index).1)
        } else {
            None
        }
    }
    fn index_of(&self, ws: &WebSocket) -> Option<usize> {
        self.0.iter().position(|(w, _)| w == ws)
    }

    pub(super) fn get(&self, ws: &WebSocket) -> Option<&Session> {
        let index = self.index_of(&ws)?;
        let (_, session) = self.0.get(index)?;
        Some(session)
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
    pub(super) fn iter_actives(&self) -> impl Iterator<Item = &ActiveSession> {
        self.iter()
            .flat_map(|(_, s)| match s {
                Session::Active(a) => Some(a),
                Session::Preparing(_) => None
            })
    }
}
