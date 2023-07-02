use crate::config::Config;
use keyed_priority_queue::KeyedPriorityQueue;
use std::hash::Hash;
use std::hash::Hasher;
use std::sync::Arc;
use std::{fs, io};
use tokio::sync::Mutex;
use tokio::task;
use tokio::time::{sleep, Duration, Instant};
use uuid::Uuid;

#[derive(Debug)]
pub struct QueueItem {
    pub id: String,
    pub expiry: Duration,
    pub created_at: Instant,
}

impl Eq for QueueItem {}

impl PartialEq for QueueItem {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Hash for QueueItem {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

#[derive(Debug)]
pub struct MessageHandler {
    pub pq: KeyedPriorityQueue<QueueItem, i32>,
    cfg: Config,
}

impl MessageHandler {
    pub fn new(cfg: Config) -> Arc<tokio::sync::Mutex<MessageHandler>> {
        let pq: KeyedPriorityQueue<QueueItem, i32> = KeyedPriorityQueue::new();
        let handler = MessageHandler { pq: pq, cfg: cfg };

        let handler_arc = Arc::new(Mutex::new(handler));

        let handler_background = Arc::clone(&handler_arc);

        // Spawn a new async task
        task::spawn(async move {
            loop {
                // Lock the message handler
                let mut handler = handler_background.lock().await;

                // Remove expired messages
                handler
                    .remove_expired_messages()
                    .await
                    .expect("Error removing messages"); // unwrap here is just for simplicity, proper error handling should be done

                drop(handler);

                // Sleep for a certain duration (for example, 1 second)
                sleep(Duration::from_secs(5)).await;
            }
        });

        handler_arc
    }

    pub async fn push_message(&mut self, data: String, priority: i32) -> Result<String, io::Error> {
        let id = Uuid::new_v4().to_string();
        let id_clone = id.clone();

        let item = QueueItem {
            expiry: Duration::from_secs(self.cfg.message_retention_seconds),
            id: id,
            created_at: Instant::now(),
        };

        self.pq.push(item, priority);

        fs::write(format!("messages/{id_clone}.data"), data)?;

        Ok(id_clone)
    }

    async fn remove_expired_messages(&mut self) -> io::Result<()> {
        // Go through each message in the queue
        // If the current time is later than created_at + expiry, remove the message
        let now = Instant::now();
        let ids_to_remove: Vec<String> = self
            .pq
            .iter()
            .filter(|(item, _)| item.created_at + item.expiry <= now)
            .map(|(item, _)| item.id.clone())
            .collect();

        for id in ids_to_remove {
            let id_to_remove = id.clone();

            // using dummy values for expiry and created_at since we implemented custom Eq and Hash traits
            // to cmp by id
            self.pq.remove(&QueueItem {
                id: id_to_remove.clone(),
                expiry: Duration::new(0, 0), // dummy value
                created_at: Instant::now(),  // dummy value
            });

            fs::remove_file(format!("messages/{}.data", id_to_remove))?; // remove the associated file
        }

        Ok(())
    }
}
