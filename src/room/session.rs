use super::Message;
use worker::WebSocket;
use ohkami::serde::{Serialize, Deserialize};


#[derive(Clone)]
pub(super) enum Session {
    Active(ActiveSession),
    Preparing(PreparingSession),
}
#[derive(Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct ActiveSession {
    username: String,
}
#[derive(Clone)]
pub(super) struct PreparingSession {
    queue: Vec<Message>,
}

impl Session {
    pub(super) fn new() -> Self {
        Self::Preparing(PreparingSession {
            queue: Vec::new()
        })
    }

    pub(super) fn restore(ws: &WebSocket) -> Option<Self> {
        ws.deserialize_attachment().ok().flatten().map(Self::Active)
    }
    pub(super) fn memorize_to(&self, ws: &WebSocket) -> worker::Result<()> {
        match self {
            Self::Active(a)    => ws.serialize_attachment(a),
            Self::Preparing(_) => Err(worker::Error::Infallible)
        }
    }

    pub(super) fn activate_for(&mut self, username: String) -> worker::Result<()> {
        match self {
            Self::Active(_)    => Err(worker::Error::Infallible),
            Self::Preparing(_) => Ok(*self = Self::Active(ActiveSession { username }))
        }
    }
}

impl ActiveSession {
    pub(super) fn username(&self) -> &str {
        &self.username
    }
}

impl PreparingSession {
    pub(super) fn queued_messages(&self) -> &[Message] {
        &self.queue
    }
    pub(super) fn enqueue_message(&mut self, message: Message) {
        self.queue.push(message);
    }
}
