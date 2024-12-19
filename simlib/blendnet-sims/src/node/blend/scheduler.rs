use crossbeam::channel;
use futures::Stream;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;

pub struct Interval {
    duration: Duration,
    current_elapsed: Duration,
    update_time: channel::Receiver<Duration>,
}

impl Interval {
    pub fn new(duration: Duration, update_time: channel::Receiver<Duration>) -> Self {
        Self {
            duration,
            current_elapsed: duration, // to immediately release at the interval 0
            update_time,
        }
    }

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

pub struct TemporalRelease {
    random_sleeps: Box<dyn Iterator<Item = Duration> + Send + Sync + 'static>,
    elapsed: Duration,
    current_sleep: Duration,
    update_time: channel::Receiver<Duration>,
}

impl TemporalRelease {
    pub fn new<Rng: RngCore + Send + Sync + 'static>(
        mut rng: Rng,
        update_time: channel::Receiver<Duration>,
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

    fn update(&mut self, elapsed: Duration) -> Option<Delay> {
        self.elapsed += elapsed;
        if self.elapsed >= self.current_sleep {
            let temporal_delay = Delay {
                expected: self.current_sleep,
                actual: self.elapsed,
                as_expected: self.current_sleep == self.elapsed,
            };
            self.elapsed = Duration::from_secs(0);
            self.current_sleep = self.random_sleeps.next().unwrap();
            Some(temporal_delay)
        } else {
            None
        }
    }

    fn log_delay(delay: &Delay) {
        tracing::info!("TemporalDelay: {}", serde_json::to_string(delay).unwrap());
    }
}

impl Stream for TemporalRelease {
    type Item = ();

    fn poll_next(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        if let Ok(elapsed) = self.update_time.recv() {
            if let Some(delay) = self.update(elapsed) {
                Self::log_delay(&delay);
                return Poll::Ready(Some(()));
            }
        }
        Poll::Pending
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct Delay {
    expected: Duration,
    actual: Duration,
    as_expected: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::StreamExt;
    use rand_chacha::rand_core::SeedableRng;

    #[test]
    fn interval_update() {
        let (_tx, rx) = channel::unbounded();
        let mut interval = Interval::new(Duration::from_secs(2), rx);

        assert!(interval.update(Duration::from_secs(0)));
        assert!(!interval.update(Duration::from_secs(1)));
        assert!(interval.update(Duration::from_secs(1)));
        assert!(interval.update(Duration::from_secs(3)));
    }

    #[test]
    fn interval_polling() {
        let waker = futures::task::noop_waker();
        let mut cx = Context::from_waker(&waker);

        let (tx, rx) = channel::unbounded();
        let mut interval = Interval::new(Duration::from_secs(2), rx);

        tx.send(Duration::from_secs(0)).unwrap();
        assert_eq!(interval.poll_next_unpin(&mut cx), Poll::Ready(Some(())));
        tx.send(Duration::from_secs(0)).unwrap();
        assert_eq!(interval.poll_next_unpin(&mut cx), Poll::Pending);
        tx.send(Duration::from_secs(1)).unwrap();
        assert_eq!(interval.poll_next_unpin(&mut cx), Poll::Pending);
        tx.send(Duration::from_secs(1)).unwrap();
        assert_eq!(interval.poll_next_unpin(&mut cx), Poll::Ready(Some(())));
        tx.send(Duration::from_secs(3)).unwrap();
        assert_eq!(interval.poll_next_unpin(&mut cx), Poll::Ready(Some(())));
    }

    #[test]
    fn temporal_release_update() {
        let (_tx, rx) = channel::unbounded();
        let mut temporal_release =
            TemporalRelease::new(rand_chacha::ChaCha8Rng::from_entropy(), rx, (1, 1));

        assert_eq!(temporal_release.update(Duration::from_secs(0)), None);
        assert_eq!(temporal_release.update(Duration::from_millis(999)), None);
        assert_eq!(
            temporal_release.update(Duration::from_secs(1)),
            Some(Delay {
                expected: Duration::from_secs(1),
                actual: Duration::from_millis(1999),
                as_expected: false,
            })
        );
        assert_eq!(
            temporal_release.update(Duration::from_secs(3)),
            Some(Delay {
                expected: Duration::from_secs(1),
                actual: Duration::from_secs(3),
                as_expected: false,
            })
        );
    }

    #[test]
    fn temporal_release_polling() {
        let waker = futures::task::noop_waker();
        let mut cx = Context::from_waker(&waker);

        let (tx, rx) = channel::unbounded();
        let mut temporal_release =
            TemporalRelease::new(rand_chacha::ChaCha8Rng::from_entropy(), rx, (1, 1));

        tx.send(Duration::from_secs(0)).unwrap();
        assert_eq!(temporal_release.poll_next_unpin(&mut cx), Poll::Pending);
        tx.send(Duration::from_millis(999)).unwrap();
        assert_eq!(temporal_release.poll_next_unpin(&mut cx), Poll::Pending);
        tx.send(Duration::from_secs(1)).unwrap();
        assert_eq!(
            temporal_release.poll_next_unpin(&mut cx),
            Poll::Ready(Some(()))
        );
        tx.send(Duration::from_secs(3)).unwrap();
        assert_eq!(
            temporal_release.poll_next_unpin(&mut cx),
            Poll::Ready(Some(()))
        );
    }
}
