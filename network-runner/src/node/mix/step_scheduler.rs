use chrono::format::Item;
use futures::Stream;
use rand::RngCore;
use std::pin::Pin;
use std::sync::mpsc;
use std::task::{Context, Poll};
use std::time::Duration;

struct Interval {
    duration: Duration,
    current_elapsed: Duration,
    update_time: mpsc::Receiver<Duration>,
}

impl Interval {
    pub fn update(&mut self, elapsed: Duration) -> bool {
        self.current_elapsed += elapsed;
        if self.current_elapsed >= self.duration {
            self.current_elapsed = Duration::from_secs(0);
            true
        } else {
            false
        }
    }
}

impl Stream for Interval {
    type Item = ();

    fn poll_next(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        if let Ok(elapsed) = self.update_time.recv() {
            if self.update(elapsed) {
                return Poll::Ready(Some(()));
            }
        }
        Poll::Pending
    }
}

struct TemporalRelease {
    random_sleeps: Box<dyn Iterator<Item = Duration>>,
    elapsed: Duration,
    current_sleep: Duration,
    update_time: mpsc::Receiver<Duration>,
}

impl TemporalRelease {
    pub fn new<Rng: RngCore + 'static>(
        mut rng: Rng,
        update_time: mpsc::Receiver<Duration>,
        (min_delay, max_delay): (u64, u64),
    ) -> Self {
        let mut random_sleeps = Box::new(std::iter::repeat_with(move || {
            Duration::from_secs((rng.next_u64() % (max_delay + 1)).max(min_delay))
        }));
        let current_sleep = random_sleeps.next().unwrap();
        Self {
            random_sleeps,
            elapsed: Duration::from_secs(0),
            current_sleep,
            update_time,
        }
    }
    pub fn update(&mut self, elapsed: Duration) -> bool {
        self.elapsed += elapsed;
        if self.elapsed >= self.current_sleep {
            self.elapsed = Duration::from_secs(0);
            self.current_sleep = self.random_sleeps.next().unwrap();
            true
        } else {
            false
        }
    }
}

impl Stream for TemporalRelease {
    type Item = ();

    fn poll_next(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        if let Ok(elapsed) = self.update_time.recv() {
            if self.update(elapsed) {
                return Poll::Ready(Some(()));
            }
        }
        Poll::Pending
    }
}
