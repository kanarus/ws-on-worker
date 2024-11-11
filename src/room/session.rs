use super::Message;
use worker::WebSocket;
use ohkami::serde::{Serialize, Deserialize};


#[derive(Serialize, Deserialize, Clone)]
pub(super) struct Session {
    /// hiberbatable data of session
    meta: Metadata,
    /// messages that are queued while this session is preparing to be active
    queue: Vec<Message>,
}

#[derive(Serialize, Deserialize, Clone, Default)]
struct Metadata {
    pub(super) username: Option<String>,
}

impl Session {
    pub(super) fn new() -> Self {
        Self {
            meta:  Metadata::default(),
            queue: Vec::new()
        }
    }

    /// create `Session` with metadata restored from the `WebSocket`
    pub(super) fn restore_on(ws: &WebSocket) -> Self {
        Self {
            meta: ws.deserialize_attachment().ok().flatten().unwrap_or_default(),
            ..Self::new()
        }
    }
    /// memorize coresponded metadata into the `WebSocket` to make it survive hibernation
    pub(super) fn memorize_on(&self, ws: &WebSocket) -> worker::Result<()> {
        ws.serialize_attachment(&self.meta)
    }

    pub(super) fn is_preparing(&self) -> bool {
        self.meta.username.is_none()
    }
    pub(super) fn mark_as_prepared_for(&mut self, username: String) {
        self.meta.username = Some(username)
    }

    pub(super) fn username(&self) -> Option<&str> {
        self.meta.username.as_deref()
    }
    pub(super) fn username_or_anonymous(&self) -> &str {
        self.meta.username.as_deref().unwrap_or("<anonymous>")
    }

    pub(super) fn queued_messages(&self) -> &[Message] {
        &self.queue
    }
    pub(super) fn take_queued_messages(&mut self) -> Vec<Message> {
        core::mem::take(&mut self.queue)
    }
    pub(super) fn enqueue_message(&mut self, message: Message) {
        self.queue.push(message);
    }
}
