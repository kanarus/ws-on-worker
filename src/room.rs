mod message;
mod session;
mod session_map;

use self::message::Message;
use self::session::Session;
use self::session_map::SessionMap;
use worker::{durable_object, async_trait, wasm_bindgen, wasm_bindgen_futures};
use worker::{WebSocket, WebSocketPair};

#[durable_object]
pub struct Room {
    name:     Option<String>,
    state:    worker::State,
    sessions: SessionMap,
}

#[durable_object]
impl DurableObject for Room {
    fn new(state: worker::State, _: worker::Env) -> Self {
        let mut sessions = SessionMap::new();

        // restore sessions if woken up from hibernation
        for ws in state.get_websockets() {
            if let Some(session) = Session::restore(&ws) {
                sessions.insert(ws, session).unwrap();
            }
        }

        Self { state, sessions, name:None }
    }

    async fn fetch(&mut self, req: worker::Request) -> worker::Result<worker::Response> {
        self.name = req.url()?.query_pairs()
            .find(|(k, _)| k == "roomname")
            .map(|(_, v)| v.into_owned());

        let WebSocketPair { client, server } = WebSocketPair::new()?;
        self.register_session(server).await?;

        worker::Response::from_websocket(client)
    }

    async fn websocket_close(
        &mut self,
        ws: WebSocket,
        code: usize,
        reason: String,
        was_clean: bool,
    ) -> worker::Result<()> {
        worker::console_log!("websocket_close: code={code}, reason={reason}, was_clean={was_clean}");
        if let Some(Session::Active(quit)) = self.sessions.remove(&ws) {
            self.broadcast(Message::MemberQuittedBroadcast {
                quit: quit.username().to_string()
            }).ok();
        };
        ws.close(Some(code as u16), Some(reason)).ok();
        Ok(())
    }

    async fn websocket_message(
        &mut self,
        ws:      WebSocket,
        message: worker::WebSocketIncomingMessage,
    ) -> worker::Result<()> {
        let message = {
            let worker::WebSocketIncomingMessage::String(message) = message else {
                return ws.send(&Message::ErrorResponse { error: String::from("expected text frame") })
            };
            let Ok(message) = Message::parse(message) else {
                return ws.send(&Message::ErrorResponse { error: String::from("unexpected format of message") })
            };
            message
        };

        match self.sessions.get(&ws).expect("No session found for the WebSocket") {
            Session::Preparing(p) => {
                let Message::JoinRequest { name } = message else {
                    return ws.send(&Message::ErrorResponse { error: String::from("expected JoinRequest message") })
                };
                if self.sessions.iter_actives().any(|a| a.username() == name) {
                    return ws.send(&Message::ErrorResponse { error: format!("username `{name}` is already used") })
                }

                if !p.queued_messages().is_empty() {
                    for queued_message in p.queued_messages() {
                        ws.send(queued_message)?;
                    }
                }

                let session = self.sessions.get_mut(&ws).unwrap();
                session.activate_by(name.clone())?;
                session.memorize_to(&ws)?;
                
                self.broadcast(Message::MemberJoinedBroadcast {
                    joined: name
                })?;

                ws.send(&Message::ReadyResponse { ready: true })?;
            }
            Session::Active(a) => {
                let Message::Text { message } = message else {
                    return ws.send(&Message::ErrorResponse { error: String::from("expected Text message") })
                };

                let timestamp = ohkami::util::unix_timestamp();

                let message = Message::TextBroadcast {
                    timestamp,
                    message,
                    name: a.username().to_string(),
                };

                self.state.storage().put(&timestamp.to_string(), &message).await?;

                self.broadcast(message)?;
            }
        }

        Ok(())
    }
}

impl Room {
    async fn register_session(
        &mut self,
        ws: WebSocket,
    ) -> worker::Result<()> {
        self.state.accept_web_socket(&ws);

        let session = {
            let mut session = Session::new();
            {
                let Session::Preparing(session) = &mut session else {unreachable!()};

                // enqueue other members' joining message
                for (_, other_session) in self.sessions.iter() {
                    if let Session::Active(other_session) = other_session {
                        session.enqueue_message(Message::MemberJoinedBroadcast {
                            joined: other_session.username().to_string()
                        });
                    }
                }

                // load last messages up to 100
                let mut last_messages = Message::load_last_n(100, self.state.storage()).await?;
                // enqueue them from older ones
                while let Some(message) = last_messages.pop() {
                    session.enqueue_message(message);
                }
            }
            session
        };

        self.sessions.insert(ws, session)?;

        Ok(())
    }

    fn broadcast(&mut self, message: Message) -> worker::Result<()> {
        let mut quitters = Vec::new();

        for (ws, session) in self.sessions.iter_mut() {
            match session {
                Session::Preparing(p) => {
                    p.enqueue_message(message.clone());
                }
                Session::Active(a) => {
                    if let Err(_) = ws.send(&message) {
                        quitters.push((ws.clone(), a.username().to_string()));
                    }
                }
            }
        }

        for (ws, _) in &quitters {
            self.sessions.remove(ws);
        }
        for (_, name) in quitters {
            self.broadcast(Message::MemberQuittedBroadcast {
                quit: name
            })?;
        }

        Ok(())
    }
}
