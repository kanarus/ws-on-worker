use super::Message;
use worker::WebSocket;
use ohkami::serde::{Serialize, Deserialize};


#[derive(Clone)]
pub(super) enum Session {
    Active(ActiveSession),
    Prepaing(PrepaingSession),
}
#[derive(Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct ActiveSession {
    username: String,
}
#[derive(Clone)]
pub(super) struct PrepaingSession {
    queue: Vec<Message>,
}

impl Session {
    pub(super) fn new() -> Self {
        Self::Prepaing(PrepaingSession {
            queue: Vec::new()
        })
    }

    pub(super) fn restore_on(ws: &WebSocket) -> Option<Self> {
        ws.deserialize_attachment().ok().flatten().map(Self::Active)
    }
    pub(super) fn memorize_on(&self, ws: &WebSocket) -> worker::Result<()> {
        match self {
            Self::Active(a)   => ws.serialize_attachment(a),
            Self::Prepaing(_) => Err(worker::Error::Infallible)
        }
    }

    pub(super) fn activate_for(&mut self, username: String) -> worker::Result<()> {
        match self {
            Self::Active(_)   => Err(worker::Error::Infallible),
            Self::Prepaing(_) => Ok(*self = Self::Active(ActiveSession { username }))
        }
    }
}

impl ActiveSession {
    pub(super) fn username(&self) -> &str {
        &self.username
    }
}

impl PrepaingSession {
    pub(super) fn queued_messages(&self) -> &[Message] {
        &self.queue
    }
    pub(super) fn enqueue_message(&mut self, message: Message) {
        self.queue.push(message);
    }
}
