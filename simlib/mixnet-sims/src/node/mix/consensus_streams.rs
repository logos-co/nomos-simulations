use crate::node::mix::scheduler::Interval;
use crossbeam::channel;
use futures::stream::iter;
use futures::{Stream, StreamExt};
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;

pub struct CounterInterval {
    interval: Box<dyn Stream<Item = usize> + Unpin>,
}

impl CounterInterval {
    pub fn new(duration: Duration, update_receiver: channel::Receiver<Duration>) -> Self {
        let interval = Interval::new(duration, update_receiver)
            .zip(iter(0usize..))
            .map(|(_, i)| i);
        let interval = Box::new(interval);
        Self { interval }
    }
}

impl Stream for CounterInterval {
    type Item = usize;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.interval.poll_next_unpin(cx)
    }
}

pub type Epoch = CounterInterval;

pub struct Slot {
    interval: Box<dyn Stream<Item = usize> + Unpin>,
}

impl Slot {
    pub fn new(
        slots_per_epoch: usize,
        slot_duration: Duration,
        update_receiver: channel::Receiver<Duration>,
    ) -> Self {
        let interval = CounterInterval::new(slot_duration, update_receiver)
            .map(move |slot| slot % slots_per_epoch);
        let interval = Box::new(interval);
        Self { interval }
    }
}

impl Stream for Slot {
    type Item = usize;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.interval.poll_next_unpin(cx)
    }
}
