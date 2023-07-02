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
pub struct MessageItem {
    pub id: String,
    pub data: String,
}

#[derive(Debug)]
pub struct MessageHandler {
    pub pq: KeyedPriorityQueue<QueueItem, i32>,
    cfg: Config,
}

impl MessageHandler {
    pub fn new(cfg: Config) -> Arc<tokio::sync::Mutex<MessageHandler>> {
        let pq: KeyedPriorityQueue<QueueItem, i32> = KeyedPriorityQueue::new();
        let handler = MessageHandler { pq, cfg };

        let handler_arc = Arc::new(Mutex::new(handler));

        let handler_background = Arc::clone(&handler_arc);

        task::spawn(async move {
            loop {
                let mut handler = handler_background.lock().await;

                handler
                    .remove_expired_messages()
                    .await
                    .expect("Error removing messages");

                drop(handler);

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
            id,
            created_at: Instant::now(),
        };

        self.pq.push(item, priority);

        fs::write(format!("messages/{id_clone}.data"), data)?;

        Ok(id_clone)
    }

    pub async fn pull_message(&mut self) -> Result<Option<MessageItem>, io::Error> {
        match self.pq.pop() {
            Some(msg) => {
                let item = msg.0;
                let item_id = item.id;
                let file_path = format!("messages/{item_id}.data");

                let data = fs::read_to_string(file_path.clone())?;

                fs::remove_file(file_path)?;

                Ok(Some(MessageItem { data, id: item_id }))
            }
            _ => Ok(None),
        }
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
