use crate::message_handler::MessageHandler;
use hyper::{Body, Request, Response, StatusCode};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::convert::Infallible;
use std::error::Error;
use std::sync::Arc;
use tokio::sync::Mutex;

type MessageHandlerMutex = Arc<Mutex<MessageHandler>>;

#[derive(Serialize, Deserialize)]
struct EnqueueRequest {
    priority: i32,
    data: Value,
}

#[derive(Serialize, Deserialize)]
struct EnqueueResponse {
    message_id: String,
}

#[derive(Serialize, Deserialize)]
struct DequeueResponse {
    message_id: String,
    data: Value,
}

fn bad_request(m: String) -> Response<Body> {
    Response::builder()
        .status(StatusCode::BAD_REQUEST)
        .body(Body::from(m))
        .unwrap()
}

fn internal_server_error(m: String) -> Response<Body> {
    Response::builder()
        .status(StatusCode::INTERNAL_SERVER_ERROR)
        .body(Body::from(m))
        .unwrap()
}

fn no_content() -> Response<Body> {
    Response::builder()
        .status(StatusCode::NO_CONTENT)
        .body(Body::from("".to_string()))
        .unwrap()
}

fn ok(m: String) -> Response<Body> {
    Response::builder().status(200).body(Body::from(m)).unwrap()
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
        Err(e) => {
            log::error!("Error destructuring request body: {e}");
            return Ok(bad_request("error reading request body".to_string()));
        }
    };

    let mut heap_inst = heap.lock().await;
    let data_string = req_body.data.to_string();
    let push_result = heap_inst.push_message(data_string, req_body.priority).await;

    drop(heap_inst);

    let id = match push_result {
        Ok(v) => v,
        Err(e) => return Ok(internal_server_error(format!("Error: {}", e))),
    };

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

pub async fn dequeue_handler(heap: MessageHandlerMutex) -> Result<Response<Body>, Infallible> {
    let mut heap_inst = heap.lock().await;
    let pull_result = match heap_inst.pull_message().await {
        Ok(result) => result,
        _ => return Ok(internal_server_error("Error reading message".to_string())),
    };

    drop(heap_inst);

    let item = match pull_result {
        Some(i) => i,
        None => return Ok(no_content()),
    };

    let data: Value = match serde_json::from_str(&item.data) {
        Ok(v) => v,
        Err(e) => {
            log::error!("Error destructuring message data: {e}");
            return Ok(internal_server_error(
                "Error destructuring message data".to_string(),
            ));
        }
    };

    let resp = DequeueResponse {
        message_id: item.id,
        data,
    };

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
