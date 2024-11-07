mod room;
use room::Room;

use ohkami::prelude::*;
use ohkami::format::HTML;
use ohkami::typed::status;
use ohkami::ws::{WebSocketContext, WebSocket, StreamExt};
use worker::{WebsocketEvent, DurableObject, ResponseBody};

#[ohkami::bindings]
struct Bindings;

#[ohkami::worker]
async fn main() -> Ohkami {
    #[cfg(debug_assertions)]
    console_error_panic_hook::set_once();

    Ohkami::new((
        "/hello".GET(|| async {"Hello, world!"}),
        "/".GET(index),
        "/ws".GET(ws_without_durable_object),
        "/chat".GET(ws_chatroom)
    ))
}

async fn index() -> HTML<&'static str> {
    HTML(include_str!("../index.html"))
}

async fn ws_without_durable_object(
    ctx: WebSocketContext<'_>
) -> WebSocket {
    ctx.upgrade(|ws| async move {
        ws.send_with_str("Hello!").unwrap();

        let mut e = ws.events().unwrap();
        while let Some(Ok(e)) = e.next().await {
            match e {
                WebsocketEvent::Close(e) => {
                    worker::console_log!("requested close: `{e:?}`");
                    ws.close(Some(1000), Some("close frame")).unwrap();
                }
                WebsocketEvent::Message(e) => {
                    if let Some(text) = e.text() {
                        worker::console_log!("got text: `{text:?}`");
                        ws.send_with_str(&text).unwrap();
                        if text == "close" {
                            ws.close(Some(1000), Some("close text")).unwrap();
                            break
                        }
                    }
                }
            }
        }
    })
}

async fn create_chatroom(
    Bindings { ROOMS }: Bindings
) -> status::Created<String> {
    let id = ROOMS.unique_id().unwrap();
    status::Created(id.to_string())
}

async fn ws_chatroom((id,): (&str,),
    _: WebSocketContext<'_>,
    Bindings { ROOMS }: Bindings
) -> WebSocket {
    let room = ROOMS
        .id_from_string(id).unwrap()
        .get_stub().unwrap();
    room.fetch_with_str(&format!("http://rooms/{id}")).await.unwrap()
        .websocket().unwrap()
        .into()
}
