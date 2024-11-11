use ohkami::serde::{json, Deserialize, Serialize};


#[derive(Deserialize, Serialize, Clone)]
#[serde(untagged)]
pub(super) enum Message {
    JoinRequest { name: String },
    Ready { ready: bool },
    MemberJoined  { joined: String },
    MemberQuitted { quit: String },
    Text { message: String },
    BroadCast { name: String, message: String, timestamp: u64 },
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
            last_n.push(json::from_str(&m?.as_string().unwrap()).unwrap());
        }
        Ok(last_n)
    }
}
