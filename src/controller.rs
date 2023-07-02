use crate::message_handler::MessageHandler;
use hyper::{Body, Request, Response};
use serde::{Deserialize, Serialize};
use std::convert::Infallible;
use std::error::Error;
use std::sync::Arc;
use tokio::sync::Mutex;

type MessageHandlerMutex = Arc<Mutex<MessageHandler>>;

#[derive(Serialize, Deserialize)]
struct EnqueueRequest {
    priority: i32,
    data: String,
}

#[derive(Serialize, Deserialize)]
struct EnqueueResponse {
    message_id: String,
}

fn bad_request(m: String) -> Response<Body> {
    Response::builder()
        .status(400)
        .body(Body::from(m.clone()))
        .unwrap()
}

fn internal_server_error(m: String) -> Response<Body> {
    Response::builder()
        .status(500)
        .body(Body::from(m.clone()))
        .unwrap()
}

fn ok(m: String) -> Response<Body> {
    Response::builder()
        .status(200)
        .body(Body::from(m.clone()))
        .unwrap()
}

async fn get_req<T: for<'de> Deserialize<'de>>(
    req: Request<Body>,
) -> Result<T, Box<dyn Error + Send + Sync>> {
    let whole_body = hyper::body::to_bytes(req.into_body()).await?;
    let item = std::str::from_utf8(&whole_body)?;

    let req_body: T = serde_json::from_str(item)?;

    Ok(req_body)
}

pub async fn enqueue_handler(
    req: Request<Body>,
    heap: MessageHandlerMutex,
) -> Result<Response<Body>, Infallible> {
    let req_body: EnqueueRequest = match get_req(req).await {
        Ok(v) => v,
        Err(_) => return Ok(bad_request("error reading request body".to_string())),
    };

    let mut heap_inst = heap.lock().await;

    match heap_inst
        .push_message(req_body.data, req_body.priority)
        .await
    {
        Err(e) => {
            drop(heap_inst);
            Ok(internal_server_error(format!("Error: {}", e)))
        }
        Ok(id) => {
            drop(heap_inst);
            let resp = EnqueueResponse { message_id: id };
            let resp_str = match serde_json::to_string(&resp) {
                Ok(v) => v,
                Err(_) => {
                    return Ok(internal_server_error(
                        "Error encoding response as json".to_string(),
                    ))
                }
            };
            Ok(ok(resp_str))
        }
    }
}
