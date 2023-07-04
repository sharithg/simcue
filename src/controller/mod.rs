use crate::message_handler::MessageHandler;
use hyper::{Body, Request, Response, StatusCode};
use serde_json::Value;
use std::convert::Infallible;
use std::sync::Arc;
use tokio::sync::Mutex;
mod dto;

type MessageHandlerMutex = Arc<Mutex<MessageHandler>>;

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

pub async fn enqueue_handler(
    req: Request<Body>,
    heap: MessageHandlerMutex,
) -> Result<Response<Body>, Infallible> {
    let full_body = hyper::body::to_bytes(req.into_body()).await.unwrap();

    // 1_048_576 bytes = 1MB
    if full_body.len() > 1_048_576 {
        log::error!("Error: data exceeds 1MB limit");
        return Ok(bad_request("Data exceeds 1MB limit".to_string()));
    }

    let req_body: dto::EnqueueRequest = match serde_json::from_slice(&full_body) {
        Ok(v) => v,
        Err(e) => {
            log::error!("Error destructuring request body: {:?}", e);
            return Ok(bad_request("error reading request body".to_string()));
        }
    };

    let mut heap_inst = heap.lock().await;
    let data_string = req_body.data.to_string();
    let push_result = heap_inst.push_message(data_string, req_body.priority).await;

    drop(heap_inst);

    let id = match push_result {
        Ok(v) => v,
        Err(e) => {
            log::error!("Error pushing message: {e}");
            return Ok(internal_server_error(format!("Error: {}", e)));
        }
    };

    let resp = dto::EnqueueResponse { message_id: id };
    let resp_str = match serde_json::to_string(&resp) {
        Ok(v) => v,
        Err(e) => {
            log::error!("Error encoding response as json: {e}");
            return Ok(internal_server_error(
                "Error encoding response as json".to_string(),
            ));
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

    let resp = dto::DequeueResponse {
        message_id: item.id,
        data,
    };

    let resp_str = match serde_json::to_string(&resp) {
        Ok(v) => v,
        Err(e) => {
            log::error!("Error encoding response as json: {e}");
            return Ok(internal_server_error(
                "Error encoding response as json".to_string(),
            ));
        }
    };

    Ok(ok(resp_str))
}
