use super::Ward::Max;
use crate::warding::{SimulationState, SimulationWard};
use serde::{Deserialize, Serialize};

/// Time to finality ward. It monitors the amount of rounds of the simulations, triggers when surpassing
/// the set threshold.
#[derive(Debug, Serialize, Deserialize, Copy, Clone)]
#[serde(transparent)]
pub struct MaxWard {
    pub max_count: usize,
}

impl<S, T> SimulationWard<S, T> for MaxWard {
    type SimulationState = SimulationState<S, T>;
    fn analyze(&mut self, state: &Self::SimulationState) -> bool {
        state.nodes.read().iter().all(|n| n.analyze(Max(*self)))
    }
}

#[cfg(test)]
mod test {
    use crate::warding::ttf::MaxWard;
    use crate::warding::{SimulationState, SimulationWard};
    use parking_lot::RwLock;
    use std::sync::Arc;

    #[test]
    fn rebase_threshold() {
        let mut ttf = MaxWard { max_count: 10 };

        let node = 11;
        let state = SimulationState {
            nodes: Arc::new(RwLock::new(vec![Box::new(node)])),
        };
        assert!(ttf.analyze(&state));

        state.nodes.write().push(Box::new(9));
        assert!(!ttf.analyze(&state));
    }
}
