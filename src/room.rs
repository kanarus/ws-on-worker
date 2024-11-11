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
            if let Some(session) = Session::restore_on(&ws) {
                sessions.insert(ws, session);
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

    async fn websocket_message(
        &mut self,
        ws:      WebSocket,
        message: worker::WebSocketIncomingMessage,
    ) -> worker::Result<()> {
        let timestamp = ohkami::util::unix_timestamp();

        let message = {
            let worker::WebSocketIncomingMessage::String(message) = message else {
                return Err(worker::Error::BadEncoding)
            };
            Message::parse(message)?
        };

        self.state.storage().put(&timestamp.to_string(), &message).await?;

        let session = self.sessions.get_mut(&ws).expect("No session found for the WebSocket");
        match session {
            Session::Prepaing(p) => {
                let Message::JoinRequest { name } = message else {
                    return Err(worker::Error::Infallible)
                };

                if !p.queued_messages().is_empty() {
                    for queued_message in p.queued_messages() {
                        ws.send(queued_message)?;
                    }
                }

                session.activate_for(name.clone())?;
                session.memorize_on(&ws)?;
                
                self.broadcast(Message::MemberJoined {
                    joined: name
                })?;

                ws.send(&Message::Ready { ready: true })?;
            }
            Session::Active(a) => {
                let Message::Text { message } = message else {
                    return Err(worker::Error::Infallible);
                };

                let message = Message::BroadCast {
                    timestamp,
                    message,
                    name: a.username().to_string(),
                };

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
                let Session::Prepaing(session) = &mut session else {unreachable!()};

                // queue other members' joining message
                for (_, other_session) in self.sessions.iter() {
                    if let Session::Active(other_session) = other_session {
                        session.enqueue_message(Message::MemberJoined {
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

        self.sessions.insert(ws, session);

        Ok(())
    }

    fn broadcast(&mut self, message: Message) -> worker::Result<()> {
        let mut quitters = Vec::new();

        for ws in self.sessions.websockets() {
            match self.sessions.get_mut(&ws).unwrap() {
                Session::Prepaing(p) => {
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
            self.broadcast(Message::MemberQuitted {
                quit: name.to_string()
            })?;
        }

        Ok(())
    }
}
