mod room;

use ohkami::prelude::*;
use ohkami::format::{HTML, Query};
use ohkami::typed::status;
use ohkami::ws::{WebSocketContext, WebSocket, Message};

#[ohkami::bindings]
struct Bindings;

#[ohkami::worker]
async fn main() -> Ohkami {
    #[cfg(debug_assertions)]
    console_error_panic_hook::set_once();

    Ohkami::new((
        "/".GET(index),
        "/ws".GET(ws_without_durable_object),
        "/chat"
            .GET(get_chatrooms)
            .POST(create_chatroom),
        "/chat/:id".GET(ws_chatroom)
    ))
}

async fn index() -> HTML<&'static str> {
    HTML(include_str!("../pages/index.html"))
}

async fn ws_without_durable_object(
    ctx: WebSocketContext<'_>
) -> WebSocket {
    ctx.upgrade(|mut conn| async move {
        conn.send("Hello!").await.expect("failed to say hello");
        while let Ok(Some(Message::Text(text))) = conn.recv().await {
            if text == "close" {break}
            conn.send(text).await.expect("failed to echo");
        }
    })
}

async fn get_chatrooms() -> HTML<&'static str> {
    HTML(include_str!("../pages/chat.html"))
}

async fn create_chatroom(
    Bindings { ROOMS }: Bindings
) -> status::Created<String> {
    let id = ROOMS.unique_id().unwrap();
    status::Created(id.to_string())
}

#[derive(Deserialize)]
struct ChatroomSessionRequest<'req> {
    username: Option<&'req str>,
}

async fn ws_chatroom((id,): (&str,),
    Query(meta): Query<ChatroomSessionRequest<'_>>,
    _: WebSocketContext<'_>,
    Bindings { ROOMS }: Bindings
) -> WebSocket {
    let room = ROOMS
        .id_from_string(id).unwrap()
        .get_stub().unwrap();

    let mut url = format!("http://rooms");
    if let Some(username) = meta.username {
        url.push_str("?username=");
        url.push_str(username);
    }

    room.fetch_with_str(&url).await.unwrap()
        .websocket().unwrap().into()
}
