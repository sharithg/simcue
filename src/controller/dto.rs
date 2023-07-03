use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Serialize, Deserialize)]
pub struct EnqueueRequest {
    pub priority: i32,
    pub data: Value,
}

#[derive(Serialize, Deserialize)]
pub struct EnqueueResponse {
    pub message_id: String,
}

#[derive(Serialize, Deserialize)]
pub struct DequeueResponse {
    pub message_id: String,
    pub data: Value,
}
