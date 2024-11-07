use worker::durable_object;
use worker::{async_trait, wasm_bindgen, wasm_bindgen_futures};
use worker::{State, Env, WebSocket, WebSocketPair};

#[durable_object]
pub struct Room {
    state: State,
    env:   Env,
}

#[durable_object]
impl DurableObject for Room {
    fn new(state: State, env: Env) -> Self {
        Self { state, env }
    }

    async fn fetch(&mut self, req: worker::Request) -> worker::Result<worker::Response> {
        let path = req.path();
        let id = path.strip_prefix('/').unwrap();

        let WebSocketPair { client, server } = WebSocketPair::new().unwrap();
        self.handle_session(id, server).await;

        worker::Response::from_websocket(client)
    }
}

impl Room {
    async fn handle_session(&self, id: &str, ws: WebSocket) {
        self.state.accept_web_socket(&ws);

        
    }
}
