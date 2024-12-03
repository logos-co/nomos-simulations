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
    NoisyCoinFlippingRandomRelease,
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
            "NoisyCoinFlippingRandomRelease" => Ok(QueueType::NoisyCoinFlippingRandomRelease),
            _ => Err(format!("Unknown queue type: {}", s)),
        }
    }
}

pub trait Queue<T: Copy> {
    fn push(&mut self, data: T);
    fn pop(&mut self) -> Message<T>;
    fn data_count(&self) -> usize;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Message<T: Copy> {
    Data(T),
    Noise,
}

pub struct QueueConfig {
    pub queue_type: QueueType,
    pub seed: u64,
    pub min_queue_size: u16,
}

pub fn new_queue<T: 'static + Copy>(cfg: &QueueConfig) -> Box<dyn Queue<T>> {
    match cfg.queue_type {
        QueueType::NonMix => Box::new(NonMixQueue::new()),
        QueueType::PureCoinFlipping => {
            Box::new(PureCoinFlippingQueue::new(cfg.min_queue_size, cfg.seed))
        }
        QueueType::PureRandomSampling => {
            Box::new(PureRandomSamplingQueue::new(cfg.min_queue_size, cfg.seed))
        }
        QueueType::PermutedCoinFlipping => {
            Box::new(PermutedCoinFlippingQueue::new(cfg.min_queue_size, cfg.seed))
        }
        QueueType::NoisyCoinFlipping => Box::new(NoisyCoinFlippingQueue::new(cfg.seed)),
        QueueType::NoisyCoinFlippingRandomRelease => {
            Box::new(NoisyCoinFlippingRandomReleaseQueue::new(cfg.seed))
        }
    }
}

struct NonMixQueue<T: Copy> {
    queue: VecDeque<T>, // don't need to contain Noise
}

impl<T: Copy> NonMixQueue<T> {
    fn new() -> Self {
        Self {
            queue: VecDeque::new(),
        }
    }
}

impl<T: Copy> Queue<T> for NonMixQueue<T> {
    fn push(&mut self, data: T) {
        self.queue.push_back(data)
    }

    fn pop(&mut self) -> Message<T> {
        match self.queue.pop_front() {
            Some(data) => Message::Data(data),
            None => Message::Noise,
        }
    }

    fn data_count(&self) -> usize {
        self.queue.len()
    }
}

struct MixQueue<T: Copy> {
    queue: Vec<Message<T>>,
    data_count: usize,
    rng: StdRng,
}

impl<T: Copy> MixQueue<T> {
    fn new(num_initial_noises: usize, seed: u64) -> Self {
        Self {
            queue: vec![Message::Noise; num_initial_noises],
            data_count: 0,
            rng: StdRng::seed_from_u64(seed),
        }
    }

    fn push(&mut self, data: T) {
        self.queue.push(Message::Data(data));
        self.data_count += 1;
    }

    fn fill_noises(&mut self, k: usize) {
        self.queue.extend(std::iter::repeat(Message::Noise).take(k))
    }

    fn pop(&mut self, idx: usize) -> Option<Message<T>> {
        if idx < self.queue.len() {
            let msg = self.queue.remove(idx);
            if let Message::Data(_) = msg {
                self.data_count -= 1;
            }
            Some(msg)
        } else {
            None
        }
    }

    fn data_count(&self) -> usize {
        self.data_count
    }

    fn len(&self) -> usize {
        self.queue.len()
    }

    fn flip_coin(&mut self) -> bool {
        self.rng.gen_bool(0.5)
    }

    fn sample_index(&mut self) -> usize {
        self.rng.gen_range(0..self.queue.len())
    }

    fn shuffle(&mut self) {
        self.queue.as_mut_slice().shuffle(&mut self.rng);
    }
}

struct MinSizeMixQueue<T: Copy> {
    queue: MixQueue<T>,
    min_pool_size: u16,
}

impl<T: Copy> MinSizeMixQueue<T> {
    fn new(min_pool_size: u16, seed: u64) -> Self {
        Self {
            queue: MixQueue::new(min_pool_size as usize, seed),
            min_pool_size,
        }
    }

    fn push(&mut self, msg: T) {
        self.queue.push(msg)
    }

    fn pop(&mut self, idx: usize) -> Option<Message<T>> {
        self.queue.pop(idx)
    }

    fn data_count(&self) -> usize {
        self.queue.data_count()
    }

    fn ensure_min_size(&mut self) {
        if self.queue.len() < self.min_pool_size as usize {
            self.queue
                .fill_noises(self.min_pool_size as usize - self.queue.len());
        }
    }

    fn len(&self) -> usize {
        self.queue.len()
    }

    fn flip_coin(&mut self) -> bool {
        self.queue.flip_coin()
    }

    fn sample_index(&mut self) -> usize {
        self.queue.sample_index()
    }

    fn shuffle(&mut self) {
        self.queue.shuffle()
    }
}

struct PureCoinFlippingQueue<T: Copy> {
    queue: MinSizeMixQueue<T>,
}

impl<T: Copy> PureCoinFlippingQueue<T> {
    fn new(min_pool_size: u16, seed: u64) -> Self {
        Self {
            queue: MinSizeMixQueue::new(min_pool_size, seed),
        }
    }
}

impl<T: Copy> Queue<T> for PureCoinFlippingQueue<T> {
    fn push(&mut self, msg: T) {
        self.queue.push(msg)
    }

    fn pop(&mut self) -> Message<T> {
        self.queue.ensure_min_size();

        loop {
            for i in 0..self.queue.len() {
                if self.queue.flip_coin() {
                    return self.queue.pop(i).unwrap();
                }
            }
        }
    }

    fn data_count(&self) -> usize {
        self.queue.data_count()
    }
}

struct PureRandomSamplingQueue<T: Copy> {
    queue: MinSizeMixQueue<T>,
}

impl<T: Copy> PureRandomSamplingQueue<T> {
    fn new(min_pool_size: u16, seed: u64) -> Self {
        Self {
            queue: MinSizeMixQueue::new(min_pool_size, seed),
        }
    }
}

impl<T: Copy> Queue<T> for PureRandomSamplingQueue<T> {
    fn push(&mut self, msg: T) {
        self.queue.push(msg)
    }

    fn pop(&mut self) -> Message<T> {
        self.queue.ensure_min_size();

        let i = self.queue.sample_index();
        self.queue.pop(i).unwrap()
    }

    fn data_count(&self) -> usize {
        self.queue.data_count()
    }
}

struct PermutedCoinFlippingQueue<T: Copy> {
    queue: MinSizeMixQueue<T>,
}

impl<T: Copy> PermutedCoinFlippingQueue<T> {
    fn new(min_pool_size: u16, seed: u64) -> Self {
        Self {
            queue: MinSizeMixQueue::new(min_pool_size, seed),
        }
    }
}

impl<T: Copy> Queue<T> for PermutedCoinFlippingQueue<T> {
    fn push(&mut self, msg: T) {
        self.queue.push(msg)
    }

    fn pop(&mut self) -> Message<T> {
        self.queue.ensure_min_size();

        self.queue.shuffle();

        loop {
            for i in 0..self.queue.len() {
                if self.queue.flip_coin() {
                    return self.queue.pop(i).unwrap();
                }
            }
        }
    }

    fn data_count(&self) -> usize {
        self.queue.data_count()
    }
}

struct NoisyCoinFlippingQueue<T: Copy> {
    queue: MixQueue<T>,
    idx: usize,
}

impl<T: Copy> NoisyCoinFlippingQueue<T> {
    pub fn new(seed: u64) -> Self {
        Self {
            queue: MixQueue::new(0, seed),
            idx: 0,
        }
    }
}

impl<T: Copy> Queue<T> for NoisyCoinFlippingQueue<T> {
    fn push(&mut self, msg: T) {
        self.queue.push(msg)
    }

    fn pop(&mut self) -> Message<T> {
        if self.queue.len() == 0 {
            return Message::Noise;
        }

        loop {
            if self.idx >= self.queue.len() {
                self.idx = 0;
            }

            if self.queue.flip_coin() {
                return self.queue.pop(self.idx).unwrap();
            } else if self.idx == 0 {
                return Message::Noise;
            } else {
                self.idx += 1;
            }
        }
    }

    fn data_count(&self) -> usize {
        self.queue.data_count()
    }
}

struct NoisyCoinFlippingRandomReleaseQueue<T: Copy> {
    queue: MixQueue<T>,
}

impl<T: Copy> NoisyCoinFlippingRandomReleaseQueue<T> {
    pub fn new(seed: u64) -> Self {
        Self {
            queue: MixQueue::new(0, seed),
        }
    }
}

impl<T: Copy> Queue<T> for NoisyCoinFlippingRandomReleaseQueue<T> {
    fn push(&mut self, msg: T) {
        self.queue.push(msg)
    }

    fn pop(&mut self) -> Message<T> {
        if self.queue.len() == 0 {
            return Message::Noise;
        }

        if self.queue.flip_coin() {
            let i = self.queue.sample_index();
            self.queue.pop(i).unwrap()
        } else {
            Message::Noise
        }
    }

    fn data_count(&self) -> usize {
        self.queue.data_count()
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    #[test]
    fn test_non_mix_queue() {
        let mut queue = new_queue(&QueueConfig {
            queue_type: QueueType::NonMix,
            seed: 0,
            min_queue_size: 0,
        });

        // Check if noise is returned when queue is empty
        assert_eq!(queue.pop(), Message::Noise);

        // Check if queue is FIFO
        queue.push(0);
        queue.push(1);
        assert_eq!(queue.pop(), Message::Data(0));
        assert_eq!(queue.pop(), Message::Data(1));

        // Check if noise is returned when queue is empty
        assert_eq!(queue.pop(), Message::Noise);

        // Check if queue is FIFO again
        queue.push(2);
        queue.push(3);
        assert_eq!(queue.pop(), Message::Data(2));
        assert_eq!(queue.pop(), Message::Data(3));
    }

    #[test]
    fn test_mix_queues() {
        for queue_type in [
            QueueType::PureCoinFlipping,
            QueueType::PureRandomSampling,
            QueueType::PermutedCoinFlipping,
            QueueType::NoisyCoinFlipping,
            QueueType::NoisyCoinFlippingRandomRelease,
        ] {
            test_mix_queue(queue_type);
        }
    }

    fn test_mix_queue(queue_type: QueueType) {
        let mut queue = new_queue(&QueueConfig {
            queue_type,
            seed: 0,
            min_queue_size: 4,
        });

        // Check if noise is returned when queue is empty
        assert_eq!(queue.pop(), Message::Noise);

        // Put only 2 messages even though the min queue size is 4
        queue.push(0);
        queue.push(1);

        // Wait until 2 messages are returned from the queue
        let mut set: HashSet<_> = vec![0, 1].into_iter().collect();
        while !set.is_empty() {
            if let Message::Data(msg) = queue.pop() {
                assert!(set.remove(&msg));
            }
        }

        // Check if noise is returned when there is no real message remains
        assert_eq!(queue.pop(), Message::Noise);
    }
}
