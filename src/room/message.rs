use ohkami::serde::{json, Deserialize, Serialize};


#[derive(Deserialize, Serialize, Clone)]
#[serde(untagged)]
pub(super) enum Message {
    TextBroadcast { name: String, message: String, timestamp: u64 },
    Text { message: String },
    JoinRequest { name: String },
    ReadyResponse { ready: bool },
    MemberJoinedBroadcast  { joined: String },
    MemberQuittedBroadcast { quit: String },
}

impl Message {
    pub(super) fn parse(raw: String) -> worker::Result<Self> {
        json::from_str(&raw).map_err(worker::Error::SerdeJsonError)
    }

    pub(super) async fn load_last_n(
        n: usize,
        storage: worker::Storage
    ) -> worker::Result<Vec<Self>> {
        let mut last_n = Vec::with_capacity(n);
        for m in storage.list_with_options(
            worker::ListOptions::new()
            .reverse(true)
            .limit(n)
        ).await?.values() {
            last_n.push(serde_wasm_bindgen::from_value(m?).unwrap());
        }
        Ok(last_n)
    }
}
