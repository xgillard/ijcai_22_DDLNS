use crate::basics::*;
use derive_builder::Builder;
use rustc_hash::FxHashMap;
use std::{
    hash::Hash,
    rc::Rc,
    sync::{atomic::AtomicBool, Arc},
    time::{Duration, Instant},
};

// ----------------------------------------------------------------------------
// Pure DP implem from problem description
// ----------------------------------------------------------------------------
#[derive(Debug, Builder)]
pub struct PureDp<P, V>
where
    P: Problem,
    V: VariableOrdering<State = P::State>,
    P::State: PartialEq + Eq + Hash,
{
    /// The problem that must be solved
    problem: P,
    /// The variable ordering that will be imposed on the problem
    var_ordering: V,

    /// When did we start working on this instance ?
    start_time: Instant,
    /// An atomic boolean acting as a kill switch. Whenever this flag turns true,
    /// the progress must stop and return the best known solution asap.
    kill_switch: Arc<AtomicBool>,
}
/// Convenient type alias for when we are solving the problem and we care about
/// the actual final solution (assignment)
type FatCache<S> = FxHashMap<Rc<S>, (isize, Rc<S>, Option<Decision>, Option<Duration>)>;

impl<P, V> PureDp<P, V>
where
    P: Problem,
    V: VariableOrdering<State = P::State>,
    P::State: PartialEq + Eq + Hash,
{
    /// Implements a generic dynamic programming resolution strategy which aims
    /// at returning the best possible value for the problem and the assignment
    /// to variables which is required to actually reach that value.
    pub fn minimize(&self) -> ResolutionOutcome {
        let mut cache = FatCache::default();
        let initial = Rc::new(self.problem.initial_state());
        self.minimize_rec(Rc::clone(&initial), &mut cache);

        // build solution from cache
        let killed = self.killed();
        let status = if killed {
            ResolutionStatus::Open{improved: true}
        } else {
            ResolutionStatus::Closed{improved: true}
        };

        let mut curr = &initial;
        let cached = cache.get(curr);

        if let Some((opt, _, _, t)) = cached {
            let mut solution = vec![];
            solution.reserve_exact(self.problem.nb_vars());

            while let Some((_, via, dec, _)) = cache.get(curr) {
                if let Some(dec) = dec {
                    solution.push(*dec);
                }
                curr = via;
            }

            let best_value = Some(*opt);
            let best_sol = Some(Solution::from(solution.iter().copied()));
            let time_to_best = *t;
            let time_to_prove = if killed {
                None
            } else {
                Some(self.start_time.elapsed())
            };

            ResolutionOutcome {
                status,
                best_value,
                best_sol,
                time_to_best,
                time_to_prove,
            }
        } else {
            ResolutionOutcome {
                status,
                best_value: None,
                best_sol: None,
                time_to_best: None,
                time_to_prove: None,
            }
        }
    }

    /// This function evaluates the kill switch
    fn killed(&self) -> bool {
        self.kill_switch.load(std::sync::atomic::Ordering::Relaxed)
    }

    /// This is where the heavy lifting of minimize is achieved. This is the
    /// method which encapsulates the resolution and recursion.
    fn minimize_rec(
        &self,
        state: Rc<P::State>,
        cache: &mut FatCache<P::State>,
    ) -> (isize, Rc<P::State>, Option<Decision>, Option<Duration>) {
        if self.killed() {
            return (isize::MAX, state, None, None);
        }
        if let Some((value, endstate, decision, t)) = cache.get(&state) {
            (*value, Rc::clone(endstate), *decision, *t)
        } else {
            let next_var = self.var_ordering.next(&mut std::iter::once(state.as_ref()));
            if let Some(var) = next_var {
                let mut best = (isize::MAX, Rc::clone(&state), None, None);
                self.problem
                    .for_each_in_domain(state.as_ref(), var, |decision| {
                        if self.killed() {
                            return;
                        }
                        let next_state = Rc::new(self.problem.transition(state.as_ref(), decision));
                        let tx_cost = self.problem.transition_cost(state.as_ref(), decision);

                        let (opt, _, _, t) = self.minimize_rec(Rc::clone(&next_state), cache);
                        let tot_cost = tx_cost.saturating_add(opt);
                        if tot_cost < best.0 {
                            best = (tot_cost, next_state, Some(decision), t)
                        }
                    });
                cache.insert(state, best.clone());
                best
            } else {
                (
                    self.problem.initial_value(),
                    state,
                    None,
                    Some(self.start_time.elapsed()),
                )
            }
        }
    }
}
