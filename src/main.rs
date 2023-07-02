use hyper::service::{make_service_fn, service_fn};
use hyper::Method;
use hyper::{Body, Request, Response, Server, StatusCode};
use std::convert::Infallible;
use std::sync::Arc;
mod config;
mod controller;
mod message_handler;
use crate::config::Config;
use message_handler::MessageHandler;
use serde::{Deserialize, Serialize};
use simple_logger::SimpleLogger;
use tokio::sync::Mutex;

type SharedHeap = Arc<Mutex<MessageHandler>>;

#[derive(Serialize, Deserialize)]
struct EnqueueResponse {
    message_id: String,
}

async fn router(req: Request<Body>, heap: SharedHeap) -> Result<Response<Body>, Infallible> {
    match (req.method(), req.uri().path()) {
        (&Method::POST, "/enqueue") => controller::enqueue_handler(req, heap).await,
        (&Method::GET, "/dequeue") => controller::dequeue_handler(heap).await,
        _ => {
            let mut not_found = Response::default();
            *not_found.status_mut() = StatusCode::NOT_FOUND;
            Ok(not_found)
        }
    }
}

#[tokio::main]
async fn main() {
    let cfg = Config::get();
    let queue_handler = MessageHandler::new(cfg);
    SimpleLogger::new().init().unwrap();

    let make_svc = make_service_fn(move |_conn| {
        let queue_handler = Arc::clone(&queue_handler);
        async move {
            Ok::<_, Infallible>(service_fn(move |req| {
                let uri = req.uri().clone();
                let method = req.method().clone();
                let router = router(req, Arc::clone(&queue_handler));
                async move {
                    let start = std::time::Instant::now();
                    let response = router.await;
                    let duration = std::time::Instant::now() - start;
                    log::info!(
                        "method = {}, uri = {}, elapsed = {}μs",
                        uri.to_string(),
                        method.as_str(),
                        duration.as_micros().to_string()
                    );
                    response
                }
            }))
        }
    });

    let addr = ([127, 0, 0, 1], 4433).into();
    let server = Server::bind(&addr).serve(make_svc);

    println!("Server running on http://{}", addr);

    if let Err(e) = server.await {
        eprintln!("server error: {}", e);
    }
}
