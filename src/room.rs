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
            sessions.set(&ws, Session::restore_on(&ws));
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

        let mut session = self.sessions.get(&ws).unwrap();
        if session.is_preparing() {
            let Message::JoinRequest { name } = message else {
                return Err(worker::Error::Infallible)
            };

            if !session.queued_messages().is_empty() {
                for queued_message in session.take_queued_messages() {
                    ws.send(&queued_message)?;
                }
                // apply `take_queued_messages` for the actual (JS-side's) `session`
                self.sessions.set(&ws, session.clone());
            }

            self.broadcast(Message::MemberJoined {
                joined: name.clone()
            });

            session.mark_as_prepared_for(name);
            ws.send(&Message::Ready { ready: true })?;

        } else {
            let Message::Text { message } = message else {
                return Err(worker::Error::Infallible);
            };

            let message = Message::BroadCast {
                timestamp,
                message,
                name: session.username().unwrap().to_string(),
            };

            self.broadcast(message);
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

        self.sessions.set(&ws, {
            let mut session = Session::new();
            {
                // queue other members' joining message
                for other_ws in self.sessions.sockets() {
                    let other_session = self.sessions.get(&other_ws).unwrap();
                    session.enqueue_message(Message::MemberJoined {
                        joined: other_session.username_or_anonymous().to_string()
                    });
                }

                // load last messages up to 100
                let mut last_messages = Message::load_last_n(100, self.state.storage()).await?;
                // enqueue them from older ones
                while let Some(message) = last_messages.pop() {
                    session.enqueue_message(message);
                }
            }
            session.memorize_on(&ws)?;
            session
        });

        Ok(())
    }

    fn broadcast(&mut self, message: Message) {
        let mut quitted_sessions = Vec::new();

        for (ws, mut session) in self.sessions.iter() {
            if session.is_preparing() {
                session.enqueue_message(message.clone());
            } else {
                if let Err(_) = ws.send(&message) {
                    quitted_sessions.push((ws, session));
                }
            }
        }

        for (ws, quitter) in quitted_sessions {
            self.sessions.remove(&ws);
            self.broadcast(Message::MemberQuitted {
                quit: quitter.username_or_anonymous().to_string()
            });
        }
    }
}
