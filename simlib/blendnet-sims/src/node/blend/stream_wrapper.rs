use std::{
    pin::Pin,
    task::{Context, Poll},
};

use crossbeam::channel;
use futures::Stream;
use serde::{Deserialize, Serialize};

pub struct CrossbeamReceiverStream<T> {
    name: String,
    receiver: channel::Receiver<T>,
}

impl<T> CrossbeamReceiverStream<T> {
    pub fn new(name: &str, receiver: channel::Receiver<T>) -> Self {
        Self {
            name: name.to_string(),
            receiver,
        }
    }

    fn log(&self) {
        tracing::info!(
            "{}: {}",
            self.name,
            serde_json::to_string(&Log {
                len: self.receiver.len(),
            })
            .unwrap()
        );
    }
}

impl<T> Stream for CrossbeamReceiverStream<T> {
    type Item = T;

    fn poll_next(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match self.receiver.try_recv() {
            Ok(item) => {
                self.log();
                Poll::Ready(Some(item))
            }
            Err(channel::TryRecvError::Empty) => Poll::Pending,
            Err(channel::TryRecvError::Disconnected) => Poll::Ready(None),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct Log {
    len: usize,
}
