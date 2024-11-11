/*
    This is Rust version of https://github.com/cloudflare/workers-chat-demo/blob/master/src/chat.mjs
    with some other demo
*/

mod room;

use ohkami::prelude::*;
use ohkami::format::HTML;
use ohkami::typed::status;
use ohkami::ws::{WebSocketContext, WebSocket, Message};

#[ohkami::bindings]
struct Bindings;

#[ohkami::worker]
async fn main() -> Ohkami {
    #[cfg(debug_assertions)]
    console_error_panic_hook::set_once();

    Ohkami::new((
        "/".GET(index_page),
        "/ws".GET(ws_without_durable_object),

        "/room"
            .GET(chat_page),
        "/api/room"
            .POST(create_private_chatroom),
        "/api/room/:roomname/websocket"
            .GET(ws_chatroom),
    ))
}

async fn index_page() -> HTML<&'static str> {
    HTML(include_str!("../pages/index.html"))
}

/// Demo to show the same WebSocket interface as native async runtimes
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

async fn chat_page() -> HTML<&'static str> {
    HTML(include_str!("../pages/chat.html"))
}

async fn create_private_chatroom(
    Bindings { ROOMS }: Bindings
) -> status::Created<String> {
    let id = ROOMS.unique_id().unwrap();
    status::Created(id.to_string())
}

/// Demo of WebSocket with DurableObjects
async fn ws_chatroom((roomname,): (&str,),
    _: WebSocketContext<'_>,
    Bindings { ROOMS }: Bindings
) -> WebSocket {
    let room = ROOMS
        .id_from_name(roomname).unwrap()
        .get_stub().unwrap();
    room.fetch_with_str(&format!("http://rooms?roomname={roomname}")).await.unwrap()
        .websocket().unwrap().into()
}
