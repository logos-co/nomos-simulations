use std::{
    pin::Pin,
    task::{Context, Poll},
};

use crossbeam::channel;
use futures::Stream;

pub struct CrossbeamReceiverStream<T> {
    receiver: channel::Receiver<T>,
}

impl<T> CrossbeamReceiverStream<T> {
    pub fn new(receiver: channel::Receiver<T>) -> Self {
        Self { receiver }
    }
}

impl<T> Stream for CrossbeamReceiverStream<T> {
    type Item = T;

    fn poll_next(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match self.receiver.try_recv() {
            Ok(item) => Poll::Ready(Some(item)),
            Err(channel::TryRecvError::Empty) => Poll::Pending,
            Err(channel::TryRecvError::Disconnected) => Poll::Ready(None),
        }
    }
}
