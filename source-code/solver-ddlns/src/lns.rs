use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Instant,
};

use crate::{
    Mdd, ResolutionOutcome, ResolutionStatus, Solution,
};
use derive_builder::Builder;

#[derive(Builder)]
#[builder(pattern = "owned")]
pub struct MddLns<D: Mdd>
{
    pub start: Instant,
    pub mdd: D,
    pub width: usize,
    #[builder(default = "Some(isize::MAX)")]
    pub initial_val: Option<isize>,
    pub initial_sol: Option<Solution>,
    pub kill_switch: Arc<AtomicBool>,
    pub nb_var     : usize
}

impl<D: Mdd> MddLns<D>
{
    pub fn minimize(&mut self) -> ResolutionOutcome {
        let mut opt = self.initial_val;
        let mut sol = self.initial_sol.clone();
        let mut ttb = None;
        let mut ttp = None;
        let mut status = ResolutionStatus::Open{improved: false};
        
        let mut d  = self.nb_var - 2;
        while !self.killed() {
            let depth = if sol.is_some() { d } else { 0 };
            let curr  = self
                .mdd
                .restricted(self.width, opt.unwrap_or(isize::MAX), &sol, depth);

            if curr.unwrap_or(isize::MAX) < opt.unwrap_or(isize::MAX) {
                opt = curr;
                sol = self.mdd.get_best_solution();
                ttb = Some(self.start.elapsed());
                
                d   = self.nb_var - 2;
            } else if d > 0 {
                d -= 1;
            } else {
                d = self.nb_var - 2; // on boucle
            }
            if self.mdd.is_exact() {
                status = ResolutionStatus::Closed{improved: opt != self.initial_val};
                ttp = Some(self.start.elapsed());
                break;
            }
        }

        let status = match status {
            ResolutionStatus::Open{..} => 
                ResolutionStatus::Open{improved: opt != self.initial_val},
            ResolutionStatus::Closed{..} => 
                ResolutionStatus::Closed{improved: opt != self.initial_val},
        };

        ResolutionOutcome {
            status,
            best_value: opt,
            best_sol: sol,
            time_to_best: ttb,
            time_to_prove: ttp,
        }
    }

    pub fn minimize_with_cond<F>(&mut self, f: F) -> ResolutionOutcome
    where
        F: Fn(isize) -> bool,
    {
        //let mut rng = Xoshiro256Plus::seed_from_u64(self.seed);
        let mut opt = self.initial_val;
        let mut sol = self.initial_sol.clone();
        let mut ttb = None;
        let mut ttp = None;
        let mut status = ResolutionStatus::Open {improved: false};

        let mut d  = self.nb_var - 2;
        while !self.killed() {
            let depth = if sol.is_some() {
                d
            } else { 
                0
            };
            let curr  = self
                .mdd
                .restricted(self.width, opt.unwrap_or(isize::MAX), &sol, depth);
            
            if curr.unwrap_or(isize::MAX) <= opt.unwrap_or(isize::MAX) {
                opt = curr;
                sol = self.mdd.get_best_solution();
                ttb = Some(self.start.elapsed());
                
                d   = self.nb_var - 2;

                if let Some(opt) = opt {
                    if f(opt) {
                        break;
                    }
                }
            } else if d > 0 {
                d -= 1;
            } else {
                d = self.nb_var - 2; // on boucle
            }
            if self.mdd.is_exact() {
                status = ResolutionStatus::Closed {improved: opt != self.initial_val};
                ttp = Some(self.start.elapsed());
                break;
            }
        }

        let status = match status {
            ResolutionStatus::Open{..} => 
                ResolutionStatus::Open{improved: opt != self.initial_val},
            ResolutionStatus::Closed{..} => 
                ResolutionStatus::Closed{improved: opt != self.initial_val},
        };

        ResolutionOutcome {
            status,
            best_value: opt,
            best_sol: sol,
            time_to_best: ttb,
            time_to_prove: ttp,
        }
    }

    fn killed(&self) -> bool {
        self.kill_switch.load(Ordering::Relaxed)
    }
}
