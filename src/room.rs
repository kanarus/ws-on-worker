use worker::{durable_object, async_trait, wasm_bindgen, wasm_bindgen_futures};
use worker::{State, Env, WebSocket, WebSocketPair, wasm_bindgen::JsValue, js_sys as js};
use ohkami::serde::{json, Deserialize, Serialize};

#[durable_object]
pub struct Room {
    state:    State,
    env:      Env,
    sessions: Sessions
}

#[derive(Deserialize, Serialize)]
enum Message {
    MemberJoined { name: String },
}
impl Message {
    fn serialize(self) -> String {
        match self {
            Self::MemberJoined { name } => format!(""),
        }
    }
}

struct Sessions(js::Map);
impl Sessions {
    fn new() -> Self {
        Self(js::Map::new())
    }

    fn set(&mut self, ws: &WebSocket, session: Session) {
        let raw_session = JsValue::from_str(&json::to_string(&session).unwrap());
        unsafe {self.set_raw(ws, raw_session)}
    }
    /// SAFETY: `raw_session` must be created from `Session`
    unsafe fn set_raw(&mut self, ws: &WebSocket, raw_session: JsValue) {
        self.0.set(ws.as_ref(), &raw_session);
    }

    fn get(&self, ws: &WebSocket) -> Option<Session> {
        self.0.get(ws.as_ref())
            .as_string()
            .map(|s| json::from_str(&s).unwrap())
    }

    fn sockets(&self) -> impl Iterator<Item = WebSocket> {
        use wasm_bindgen::JsCast;

        self.0.keys().into_iter()
            .map(|result| result.unwrap().unchecked_into::<worker::worker_sys::web_sys::WebSocket>().into())
    }
}

#[derive(Serialize, Deserialize)]
struct Session {
    meta:          Metadata,
    message_queue: Vec<Message>,
}

#[derive(Serialize, Deserialize, Default, Clone)]
struct Metadata {
    username: Option<String>,
}

#[durable_object]
impl DurableObject for Room {
    fn new(state: State, env: Env) -> Self {
        let mut sessions = Sessions::new();

        // restore sessions if woken up from hibernation
        for ws in state.get_websockets() {
            sessions.set(&ws, Session {
                meta:          ws.deserialize_attachment().unwrap().unwrap_or_default(),
                message_queue: Vec::new()
            });
        }

        Self { state, env, sessions }
    }

    async fn fetch(&mut self, req: worker::Request) -> worker::Result<worker::Response> {
        let username = req.url().unwrap().query_pairs()
            .find(|(k, _)| k == "username")
            .map(|(_, v)| v.into_owned());

        let WebSocketPair { client, server } = WebSocketPair::new().unwrap();
        self.handle_session(server, username).await;

        worker::Response::from_websocket(client)
    }

    async fn websocket_message(
        &mut self,
        ws:      WebSocket,
        message: worker::WebSocketIncomingMessage,
    ) -> worker::Result<()> {
        let session = self.sessions.get(&ws).unwrap();

        let worker::WebSocketIncomingMessage::String(message) = message else {
            return Err(worker::Error::BadEncoding)
        };

        

        Ok(())
    }
}

impl Room {
    async fn handle_session(
        &mut self,
        ws: WebSocket,
        username: Option<String>,
    ) {
        self.state.accept_web_socket(&ws);

        ws.send_with_str(format!("Hi, this is a chat room!")).unwrap();

        let meta = Metadata { username };
        ws.serialize_attachment(meta.clone()).unwrap();

        self.sessions.set(&ws, {
            let mut message_queue = vec![];
            {
                // queue other members' joining message
                for ws in self.sessions.sockets() {
                    let display_name = self.sessions.get(&ws).unwrap()
                        .meta.username.unwrap_or_else(|| String::from("anonymous"));
                    message_queue.push(Message::MemberJoined {
                        name: display_name
                    });
                }

                // load last up to 100 messages
                let mut last_messages = self.state.storage()
                    .list_with_options(worker::ListOptions::new().limit(100).reverse(true)).await.unwrap()
                    .values().into_iter()
                    .map(|v| json::from_str(&v.unwrap().as_string().unwrap()).unwrap())
                    .collect::<Vec<Message>>();
                while let Some(message) = last_messages.pop() {
                    message_queue.push(message);
                }
            }
            Session { meta, message_queue }
        });
    }
}
