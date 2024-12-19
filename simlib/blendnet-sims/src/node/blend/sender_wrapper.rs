use crossbeam::channel;
use serde::{Deserialize, Serialize};

pub struct CrossbeamSenderWrapper<T> {
    name: String,
    sender: channel::Sender<T>,
}

impl<T> CrossbeamSenderWrapper<T> {
    pub fn new(name: &str, sender: channel::Sender<T>) -> Self {
        Self {
            name: name.to_string(),
            sender,
        }
    }

    pub fn send(&self, item: T) -> Result<(), channel::SendError<T>> {
        self.sender.send(item)?;
        self.log();
        Ok(())
    }

    fn log(&self) {
        tracing::info!(
            "{}: {}",
            self.name,
            serde_json::to_string(&Log {
                len: self.sender.len(),
            })
            .unwrap()
        );
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct Log {
    len: usize,
}
