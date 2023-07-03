use crate::config::Config;
use keyed_priority_queue::KeyedPriorityQueue;
use std::fs::File;
use std::hash::Hash;
use std::hash::Hasher;
use std::sync::Arc;
use std::{fs, io};
use tokio::sync::Mutex;
use tokio::task;
use tokio::time::{sleep, Duration, Instant};
use uuid::Uuid;

pub trait Fs {
    fn set_len(&mut self, len: u64) -> io::Result<()>;
}

impl Fs for File {
    fn set_len(&mut self, len: u64) -> io::Result<()> {
        File::set_len(self, len)
    }
}

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
    pub message_store_dir: String,
}

impl MessageHandler {
    pub fn new(
        cfg: Config,
        message_store_dir: String,
        delete_expired_messages: bool,
    ) -> Arc<tokio::sync::Mutex<MessageHandler>> {
        let pq: KeyedPriorityQueue<QueueItem, i32> = KeyedPriorityQueue::new();
        let handler = MessageHandler {
            pq,
            cfg,
            message_store_dir,
        };

        let handler_arc = Arc::new(Mutex::new(handler));

        if !delete_expired_messages {
            return handler_arc;
        }

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
        let file_path = self.get_message_file_path(id.clone());

        let item = QueueItem {
            expiry: Duration::from_secs(self.cfg.message_retention_seconds),
            id: id.clone(),
            created_at: Instant::now(),
        };

        self.pq.push(item, priority);

        fs::write(file_path, data)?;

        Ok(id)
    }

    pub async fn pull_message(&mut self) -> Result<Option<MessageItem>, io::Error> {
        let msg = match self.pq.pop() {
            Some(m) => m,
            _ => return Ok(None),
        };

        let item = msg.0;
        let item_id = item.id;
        let file_path = self.get_message_file_path(item_id.clone());

        let data = fs::read_to_string(file_path.clone())?;

        fs::remove_file(file_path)?;

        Ok(Some(MessageItem { data, id: item_id }))
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
            let file_path = self.get_message_file_path(id.clone());

            // using dummy values for expiry and created_at since we implemented custom Eq and Hash traits
            // to cmp by id
            self.pq.remove(&QueueItem {
                id: id.clone(),
                expiry: Duration::new(0, 0), // dummy value
                created_at: Instant::now(),  // dummy value
            });

            fs::remove_file(file_path)?; // remove the associated file
        }

        Ok(())
    }

    fn get_message_file_path(&self, id: String) -> String {
        let msg_dir = &self.message_store_dir;
        format!("{}/{}.data", msg_dir, id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;
    use tempdir::TempDir;
    use tokio::time::Duration;

    #[tokio::test]
    async fn it_pushes_messages() {
        let tmp_dir = TempDir::new("messages").unwrap();
        let tempdir_path = tmp_dir.path().to_str().unwrap().to_string();

        let cfg = Config {
            message_retention_seconds: 0,
        };
        let queue_handler = MessageHandler::new(cfg, tempdir_path.clone(), false);

        let mut queue_handler_inst = queue_handler.lock().await;

        let id = queue_handler_inst
            .push_message("test1".to_string(), 1)
            .await
            .unwrap();

        assert!(Path::new(&format!("{}/{}.data", tempdir_path, id)).exists());
    }

    #[tokio::test]
    async fn it_pulls_messages() {
        let tmp_dir = TempDir::new("messages").unwrap();
        let tempdir_path = tmp_dir.path().to_str().unwrap().to_string();

        let cfg = Config {
            message_retention_seconds: 0,
        };
        let queue_handler = MessageHandler::new(cfg, tempdir_path.clone(), false);

        let mut queue_handler_inst = queue_handler.lock().await;

        queue_handler_inst
            .push_message("test1".to_string(), 1)
            .await
            .unwrap();

        let message_item = queue_handler_inst.pull_message().await.unwrap().unwrap();
        assert_eq!(message_item.data, "test1");

        assert!(!Path::new(&format!("{}/{}.data", tempdir_path, message_item.id)).exists());
    }

    #[tokio::test]
    async fn it_pushes_and_pulls_in_priority() {
        let tmp_dir = TempDir::new("messages").unwrap();
        let tempdir_path = tmp_dir.path().to_str().unwrap().to_string();

        let cfg = Config {
            message_retention_seconds: 0,
        };
        let queue_handler = MessageHandler::new(cfg, tempdir_path.clone(), false);

        let mut queue_handler_inst = queue_handler.lock().await;

        queue_handler_inst
            .push_message("test1".to_string(), 1)
            .await
            .unwrap();
        queue_handler_inst
            .push_message("test2".to_string(), 1)
            .await
            .unwrap();
        queue_handler_inst
            .push_message("test3".to_string(), 2)
            .await
            .unwrap();
        let message_item1 = queue_handler_inst.pull_message().await.unwrap().unwrap();
        let message_item2 = queue_handler_inst.pull_message().await.unwrap().unwrap();
        let message_item3 = queue_handler_inst.pull_message().await.unwrap().unwrap();

        assert_eq!(message_item1.data, "test3");
        assert_eq!(message_item2.data, "test1");
        assert_eq!(message_item3.data, "test2");

        assert!(!Path::new(&format!("{}/{}.data", tempdir_path, message_item1.id)).exists());
        assert!(!Path::new(&format!("{}/{}.data", tempdir_path, message_item2.id)).exists());
        assert!(!Path::new(&format!("{}/{}.data", tempdir_path, message_item3.id)).exists());
    }

    #[tokio::test]
    async fn it_removes_expired_messages() {
        let tmp_dir = TempDir::new("messages").unwrap();
        let tempdir_path = tmp_dir.path().to_str().unwrap().to_string();

        let cfg = Config {
            message_retention_seconds: 1,
        };
        let queue_handler = MessageHandler::new(cfg, tempdir_path.clone(), false);

        let mut queue_handler_inst = queue_handler.lock().await;

        let id = queue_handler_inst
            .push_message("test1".to_string(), 10)
            .await
            .unwrap();

        sleep(Duration::from_secs(2)).await;

        queue_handler_inst.remove_expired_messages().await.unwrap();

        let entry = queue_handler_inst.pq.get_priority(&QueueItem {
            id: id.clone(),
            expiry: Duration::new(0, 0),
            created_at: Instant::now(),
        });

        assert!(entry.unwrap_or(&-1).eq(&-1));

        assert!(!Path::new(&format!("{}/{}.data", tempdir_path, id)).exists());
    }

    #[tokio::test]
    async fn it_builds_file_path() {
        let tmp_dir = TempDir::new("messages").unwrap();
        let tempdir_path = tmp_dir.path().to_str().unwrap().to_string();

        let cfg = Config {
            message_retention_seconds: 0,
        };
        let queue_handler = MessageHandler::new(cfg, tempdir_path.clone(), false);

        let queue_handler_inst = queue_handler.lock().await;

        let id = "test-id";
        let expected_path = format!("{}/{}.data", tempdir_path, id);

        assert_eq!(
            queue_handler_inst.get_message_file_path(id.to_string()),
            expected_path
        );
    }
}
