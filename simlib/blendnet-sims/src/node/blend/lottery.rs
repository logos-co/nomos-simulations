use rand::Rng;

pub struct StakeLottery<R> {
    rng: R,
    stake_proportion: f64,
}

impl<R> StakeLottery<R>
where
    R: Rng,
{
    pub fn new(rng: R, stake_proportion: f64) -> Self {
        Self {
            rng,
            stake_proportion,
        }
    }

    pub fn run(&mut self) -> bool {
        self.rng.gen_range(0.0..1.0) < self.stake_proportion
    }
}
