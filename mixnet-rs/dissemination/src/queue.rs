use std::collections::VecDeque;

use rand::{rngs::StdRng, seq::SliceRandom, Rng, SeedableRng};
use strum_macros::EnumIter;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, EnumIter)]
pub enum QueueType {
    NonMix,
    PureCoinFlipping,
    PureRandomSampling,
    PermutedCoinFlipping,
    NoisyCoinFlipping,
}

impl std::str::FromStr for QueueType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "NonMix" => Ok(QueueType::NonMix),
            "PureCoinFlipping" => Ok(QueueType::PureCoinFlipping),
            "PureRandomSampling" => Ok(QueueType::PureRandomSampling),
            "PermutedCoinFlipping" => Ok(QueueType::PermutedCoinFlipping),
            "NoisyCoinFlipping" => Ok(QueueType::NoisyCoinFlipping),
            _ => Err(format!("Unknown queue type: {}", s)),
        }
    }
}

pub trait Queue<T: Copy> {
    fn push(&mut self, msg: T);
    fn pop(&mut self) -> Option<T>;
    fn len(&self) -> usize;
}

pub struct QueueConfig {
    pub queue_type: QueueType,
    pub seed: u64,
    pub min_queue_size: u16,
}

pub fn new_queue<T: 'static + Copy>(cfg: &QueueConfig) -> Box<dyn Queue<T>> {
    match cfg.queue_type {
        QueueType::NonMix => Box::new(NonMixQueue::new()),
        QueueType::PureCoinFlipping => Box::new(PureCoinFlippingQueue::new(
            cfg.min_queue_size,
            StdRng::seed_from_u64(cfg.seed),
        )),
        QueueType::PureRandomSampling => Box::new(PureRandomSamplingQueue::new(
            cfg.min_queue_size,
            StdRng::seed_from_u64(cfg.seed),
        )),
        QueueType::PermutedCoinFlipping => Box::new(PermutedCoinFlippingQueue::new(
            cfg.min_queue_size,
            StdRng::seed_from_u64(cfg.seed),
        )),
        QueueType::NoisyCoinFlipping => {
            Box::new(NoisyCoinFlippingQueue::new(StdRng::seed_from_u64(cfg.seed)))
        }
    }
}

struct NonMixQueue<T: Copy> {
    queue: VecDeque<T>,
}

impl<T: Copy> NonMixQueue<T> {
    fn new() -> Self {
        Self {
            queue: VecDeque::new(),
        }
    }
}

impl<T: Copy> Queue<T> for NonMixQueue<T> {
    fn push(&mut self, msg: T) {
        self.queue.push_back(msg)
    }

    fn pop(&mut self) -> Option<T> {
        self.queue.pop_front()
    }

    fn len(&self) -> usize {
        self.queue.len()
    }
}

struct MixQueue<T: Copy> {
    queue: Vec<Option<T>>,
    rng: StdRng,
}

impl<T: Copy> MixQueue<T> {
    fn new(rng: StdRng) -> Self {
        Self {
            queue: Vec::new(),
            rng,
        }
    }

    fn push(&mut self, data: T) {
        self.queue.push(Some(data))
    }

    fn pop(&mut self, idx: usize) -> Option<T> {
        if idx < self.queue.len() {
            self.queue.remove(idx)
        } else {
            None
        }
    }

    fn len(&self) -> usize {
        self.queue.len()
    }
}

struct MinSizeMixQueue<T: Copy> {
    queue: MixQueue<T>,
    min_pool_size: u16,
}

impl<T: Copy> MinSizeMixQueue<T> {
    fn new(min_pool_size: u16, rng: StdRng) -> Self {
        let mut queue = MixQueue::new(rng);
        queue.queue = vec![None; min_pool_size as usize];
        Self {
            queue,
            min_pool_size,
        }
    }

    fn push(&mut self, msg: T) {
        self.queue.push(msg)
    }

    fn pop(&mut self, idx: usize) -> Option<T> {
        self.queue.pop(idx)
    }

    fn ensure_min_size(&mut self) {
        if self.queue.len() < self.min_pool_size as usize {
            self.queue.queue.extend(
                std::iter::repeat(None).take(self.min_pool_size as usize - self.queue.queue.len()),
            );
        }
    }

    fn len(&self) -> usize {
        self.queue.len()
    }
}

struct PureCoinFlippingQueue<T: Copy> {
    queue: MinSizeMixQueue<T>,
}

impl<T: Copy> PureCoinFlippingQueue<T> {
    fn new(min_pool_size: u16, rng: StdRng) -> Self {
        Self {
            queue: MinSizeMixQueue::new(min_pool_size, rng),
        }
    }
}

impl<T: Copy> Queue<T> for PureCoinFlippingQueue<T> {
    fn push(&mut self, msg: T) {
        self.queue.push(msg)
    }

    fn pop(&mut self) -> Option<T> {
        self.queue.ensure_min_size();

        loop {
            for i in 0..self.len() {
                if self.queue.queue.rng.gen_bool(0.5) {
                    return self.queue.pop(i);
                }
            }
        }
    }

    fn len(&self) -> usize {
        self.queue.len()
    }
}

struct PureRandomSamplingQueue<T: Copy> {
    queue: MinSizeMixQueue<T>,
}

impl<T: Copy> PureRandomSamplingQueue<T> {
    fn new(min_pool_size: u16, rng: StdRng) -> Self {
        Self {
            queue: MinSizeMixQueue::new(min_pool_size, rng),
        }
    }
}

impl<T: Copy> Queue<T> for PureRandomSamplingQueue<T> {
    fn push(&mut self, msg: T) {
        self.queue.push(msg)
    }

    fn pop(&mut self) -> Option<T> {
        self.queue.ensure_min_size();

        let i = self.queue.queue.rng.gen_range(0..self.queue.len());
        self.queue.pop(i)
    }

    fn len(&self) -> usize {
        self.queue.len()
    }
}

struct PermutedCoinFlippingQueue<T: Copy> {
    queue: MinSizeMixQueue<T>,
}

impl<T: Copy> PermutedCoinFlippingQueue<T> {
    fn new(min_pool_size: u16, rng: StdRng) -> Self {
        Self {
            queue: MinSizeMixQueue::new(min_pool_size, rng),
        }
    }
}

impl<T: Copy> Queue<T> for PermutedCoinFlippingQueue<T> {
    fn push(&mut self, msg: T) {
        self.queue.push(msg)
    }

    fn pop(&mut self) -> Option<T> {
        self.queue.ensure_min_size();

        self.queue
            .queue
            .queue
            .as_mut_slice()
            .shuffle(&mut self.queue.queue.rng);

        loop {
            for i in 0..self.queue.len() {
                if self.queue.queue.rng.gen_bool(0.5) {
                    return self.queue.pop(i);
                }
            }
        }
    }

    fn len(&self) -> usize {
        self.queue.len()
    }
}

struct NoisyCoinFlippingQueue<T: Copy> {
    queue: MixQueue<T>,
}

impl<T: Copy> NoisyCoinFlippingQueue<T> {
    pub fn new(rng: StdRng) -> Self {
        Self {
            queue: MixQueue::new(rng),
        }
    }
}

impl<T: Copy> Queue<T> for NoisyCoinFlippingQueue<T> {
    fn push(&mut self, msg: T) {
        self.queue.push(msg)
    }

    fn pop(&mut self) -> Option<T> {
        if self.queue.len() == 0 {
            return None;
        }

        loop {
            for i in 0..self.queue.len() {
                if self.queue.rng.gen_bool(0.5) {
                    return self.queue.pop(i);
                } else if i == 0 {
                    return None;
                }
            }
        }
    }

    fn len(&self) -> usize {
        self.queue.len()
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    #[test]
    fn test_non_mix_queue() {
        let mut queue = new_queue(QueueType::NonMix, 0, 0);

        // Check if None (noise) is returned when queue is empty
        assert_eq!(queue.pop(), None);

        // Check if queue is FIFO
        queue.push(0);
        queue.push(1);
        assert_eq!(queue.pop(), Some(0));
        assert_eq!(queue.pop(), Some(1));

        // Check if None (noise) is returned when queue is empty
        assert_eq!(queue.pop(), None);

        // Check if queue is FIFO again
        queue.push(2);
        queue.push(3);
        assert_eq!(queue.pop(), Some(2));
        assert_eq!(queue.pop(), Some(3));
    }

    #[test]
    fn test_mix_queues() {
        for queue_type in [
            QueueType::PureCoinFlipping,
            QueueType::PureRandomSampling,
            QueueType::PermutedCoinFlipping,
            QueueType::NoisyCoinFlipping,
        ] {
            test_mix_queue(queue_type);
        }
    }

    fn test_mix_queue(queue_type: QueueType) {
        let mut queue = new_queue(queue_type, 0, 4);

        // Check if None (noise) is returned when queue is empty
        assert_eq!(queue.pop(), None);

        // Put only 2 messages even though the min queue size is 4
        queue.push(0);
        queue.push(1);

        // Wait until 2 messages are returned from the queue
        let mut set: HashSet<_> = vec![0, 1].into_iter().collect();
        while !set.is_empty() {
            if let Some(msg) = queue.pop() {
                assert!(set.remove(&msg));
            }
        }

        // Check if None (noise) is returned when there is no real message remains
        assert_eq!(queue.pop(), None);
    }
}
