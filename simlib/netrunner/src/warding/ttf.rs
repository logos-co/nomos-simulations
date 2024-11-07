use std::cell::RefCell;

use super::WardCondition::{self, Max};
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
        state.nodes.read().iter().all(|n| n.analyze(&mut Max(self)))
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(transparent)]
pub struct SumWard {
    max_count: usize,
}

pub struct SumWardCondition {
    pub step_result: RefCell<usize>,
}

impl<S, T> SimulationWard<S, T> for SumWard {
    type SimulationState = SimulationState<S, T>;
    fn analyze(&mut self, state: &Self::SimulationState) -> bool {
        let nodes = state.nodes.read();
        let condition = SumWardCondition {
            step_result: RefCell::new(0),
        };
        for node in nodes.iter() {
            node.analyze(&mut WardCondition::Sum(&condition));
        }
        let result = condition.step_result.borrow();
        *result > self.max_count
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
