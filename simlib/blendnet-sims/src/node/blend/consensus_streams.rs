use crate::node::blend::scheduler::Interval;
use crossbeam::channel;
use futures::stream::iter;
use futures::{Stream, StreamExt};
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;

pub struct CounterInterval {
    interval: Box<dyn Stream<Item = usize> + Unpin + Send + Sync>,
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
    interval: Box<dyn Stream<Item = usize> + Unpin + Send + Sync>,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counter_interval() {
        let waker = futures::task::noop_waker();
        let mut cx = Context::from_waker(&waker);

        let (update_sender, update_receiver) = channel::unbounded();
        let mut interval = CounterInterval::new(Duration::from_secs(1), update_receiver);

        update_sender.send(Duration::from_secs(0)).unwrap();
        assert_eq!(interval.poll_next_unpin(&mut cx), Poll::Ready(Some(0)));
        update_sender.send(Duration::from_secs(0)).unwrap();
        assert_eq!(interval.poll_next_unpin(&mut cx), Poll::Pending);
        update_sender.send(Duration::from_millis(999)).unwrap();
        assert_eq!(interval.poll_next_unpin(&mut cx), Poll::Pending);
        update_sender.send(Duration::from_millis(1)).unwrap();
        assert_eq!(interval.poll_next_unpin(&mut cx), Poll::Ready(Some(1)));
        update_sender.send(Duration::from_secs(1)).unwrap();
        assert_eq!(interval.poll_next_unpin(&mut cx), Poll::Ready(Some(2)));
        update_sender.send(Duration::from_secs(3)).unwrap();
        assert_eq!(interval.poll_next_unpin(&mut cx), Poll::Ready(Some(3)));
    }

    #[test]
    fn slot_interval() {
        let waker = futures::task::noop_waker();
        let mut cx = Context::from_waker(&waker);

        let (update_sender, update_receiver) = channel::unbounded();
        let mut slot = Slot::new(3, Duration::from_secs(1), update_receiver);

        update_sender.send(Duration::from_secs(0)).unwrap();
        assert_eq!(slot.poll_next_unpin(&mut cx), Poll::Ready(Some(0)));
        update_sender.send(Duration::from_secs(0)).unwrap();
        assert_eq!(slot.poll_next_unpin(&mut cx), Poll::Pending);
        update_sender.send(Duration::from_millis(999)).unwrap();
        assert_eq!(slot.poll_next_unpin(&mut cx), Poll::Pending);
        update_sender.send(Duration::from_millis(1)).unwrap();
        assert_eq!(slot.poll_next_unpin(&mut cx), Poll::Ready(Some(1)));
        update_sender.send(Duration::from_secs(1)).unwrap();
        assert_eq!(slot.poll_next_unpin(&mut cx), Poll::Ready(Some(2)));
        update_sender.send(Duration::from_secs(3)).unwrap();
        assert_eq!(slot.poll_next_unpin(&mut cx), Poll::Ready(Some(0)));
        update_sender.send(Duration::from_secs(1)).unwrap();
        assert_eq!(slot.poll_next_unpin(&mut cx), Poll::Ready(Some(1)));
    }
}
