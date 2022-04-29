use std::{vec, fmt::Debug};

use array2d::Array2D;
use rand::{Rng, prelude::SliceRandom};

use crate::{BitSet, Random};

/// A time window
#[derive(Debug, Clone, Copy)]
pub struct TimeWindow {
    /// Earliest time when the salesman can visit the client
    pub earliest: usize,
    /// Latest time when the salesman can visit the client
    pub latest  : usize,
}

/// An instance of the problem
#[derive(Debug, Clone)]
pub struct Tsptw {
    /// The number of clients that must be served
    pub n_client: usize,
    /// The distance between any two clients
    pub dist: Array2D<usize>,
    /// The time window associated with each client
    pub tw: Vec<TimeWindow>
}

impl Tsptw {
    // Ok(cost)
    // Err(pos where it failed)
    pub fn evaluate(&self, perm: &[usize], verbose: bool) -> Evaluation {
        let mut time       = self.tw[0].earliest;
        let mut cost       = 0;
        let mut prev       = 0;

        let mut violation  = 0;
        let mut feasible   = BitSet::empty();
        let mut infeasible = BitSet::empty(); 

        for curr in perm.iter().copied() {
            let travel = self.dist[(prev, curr)];
            time = (time + travel).max(self.tw[curr].earliest);
            cost+= travel;

            let end = self.tw[curr].latest;
            if verbose {
                println!("{:>2} -> {:>2} ||time {:>10.4} -- latest {:>10.4} || violation {}", 
                    prev, curr,
                    (time as f32) / 10000.0, (end as f32) / 10000.0,
                    time > end
                );
            }

            if time > end {
                violation += time - end;
                infeasible.add(curr);
            } else {
                feasible.add(curr);
            }

            prev = curr;
        }

        // go back to depot
        let travel = self.dist[(prev, 0)];
        time = (time + travel).max(self.tw[0].earliest);
        cost+= travel;

        if time > self.tw[0].latest {
            violation += time - self.tw[0].latest;
        }

        Evaluation { cost, violation, feasible, infeasible } 
    }

    pub fn find_feasible(&self, rng: &mut Random, before: &Before, level_max: usize) -> Vec<usize> {
        let mut perm = (1..self.n_client).collect::<Vec<_>>();
        self.random_perm(rng, &mut perm);
        let mut evaluation = self.evaluate(&perm, false);

        let (improved, eval) = self.or_opt1_ls(before, evaluation, &mut perm);
        if improved {
            evaluation = eval;
            if evaluation.violation == 0 {
                return perm;
            }
        }

        let mut copy = perm.clone();
        // until  a solution is found
        loop { 
            let mut level= 0;
            while level <= level_max {
                // copy back the original permutation
                copy.iter_mut().zip(perm.iter()).for_each(|(x, y)| *x = *y);

                let (improved, eval) = self.or_opt1_ls(before, evaluation, &mut copy);
                if improved {
                    evaluation = eval;
                    perm.iter_mut().zip(copy.iter()).for_each(|(x, y)| *x = *y);
                    level  = 0;
                } else {
                    level += 1;
                    self.shake(rng, &mut copy, level);
                }
            }

            if evaluation.violation == 0 {
                return perm;
            }
            
            // level has been exhausted: start from another random perm
            // println!(">>>>>>>>>>>>>> SHUFFLE >>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>");
            self.random_perm(rng, &mut perm);
            evaluation = self.evaluate(&perm, false);
        }
    }

    fn random_perm(&self, rng: &mut Random, perm: &mut [usize]) {
        perm.shuffle(rng)
    }

    fn shake(&self, rng: &mut Random, perm: &mut [usize], level: usize) {
        for _ in 0..level {
            let x = rng.gen_range(0..perm.len());
            let y = rng.gen_range(0..perm.len());
            perm.swap(x, y);
        }
    }

    fn or_opt1_ls(&self, before: &Before, evaluation: Evaluation, perm: &mut Vec<usize>) -> (bool, Evaluation) {
        //if evaluation.violation == 0 {
        //    (true, evaluation)
        //} else {
            let moves    = Moves::new(evaluation);
            let mut copy = perm.clone();

            let mut improved = false;
            let mut best_eval= evaluation;

            for (move_type, item) in moves.iter() {
                copy.iter_mut().zip(perm.iter()).for_each(|(x, y)| *x = *y);
                let (applied, eval) = self.try_apply(before, evaluation, &mut copy, move_type, item);
                if applied && (eval.violation < best_eval.violation || (eval.violation == best_eval.violation && eval.cost <= best_eval.cost)) {
                    improved  = true;
                    best_eval = eval;
                    perm.iter_mut().zip(copy.iter()).for_each(|(x, y)| *x = *y);
                }
            }

            (improved, best_eval)
        //}
    }

    fn try_apply(&self, before: &Before, eval: Evaluation, perm: &mut Vec<usize>, move_type: MoveType, item: usize) -> (bool, Evaluation) {
        match move_type {
            MoveType::FeasibleBackward   => self.try_move_backwards(before, eval, perm, item),
            MoveType::InfeasibleBackward => self.try_move_backwards(before, eval, perm, item),
            MoveType::FeasibleForward    => self.try_move_forwards (before, eval, perm, item),
            MoveType::InfeasibleForward  => self.try_move_forwards (before, eval, perm, item),
        }
    }

    fn try_move_backwards(&self, before: &Before, eval: Evaluation, perm: &mut Vec<usize>, item: usize) -> (bool, Evaluation) {
        let pos = perm.iter().position(|x| *x==item).unwrap();
        
        let mut improved = false;
        let mut best_p   = perm.clone();
        let mut best_e   = eval;
        
        for i in (1..=pos).rev() {
            if before.is_before(perm[i-1], item) {
                break;
            }
            perm.swap(i, i-1);
            
            
            let e_eval = self.evaluate(perm, false);
            if e_eval.violation < best_e.violation {
                improved = true;
                best_e   = e_eval;
                best_p.iter_mut().zip(perm.iter()).for_each(|(x, y)| *x = *y);
            }
        }

        perm.iter_mut().zip(best_p.iter()).for_each(|(x, y)| *x = *y);
        (improved, best_e)
    }

    fn try_move_forwards(&self, before: &Before, eval: Evaluation, perm: &mut Vec<usize>, item: usize) -> (bool, Evaluation) {
        let pos = perm.iter().position(|x| *x==item).unwrap();

        let mut improved = false;
        let mut best_p   = perm.clone();
        let mut best_e   = eval;
        
        for i in pos..perm.len()-1 {
            if before.is_before(item, perm[i+1]) {
                break;
            }
            perm.swap(i, i+1);
            
            let e_eval = self.evaluate(perm, false);
            if e_eval.violation < best_e.violation {
                improved = true;
                best_e   = e_eval;
                best_p.iter_mut().zip(perm.iter()).for_each(|(x, y)| *x = *y);
            }
        }

        perm.iter_mut().zip(best_p.iter()).for_each(|(x, y)| *x = *y);
        (improved, best_e)
    }

}

#[derive(Debug, Clone, Copy)]
pub struct Evaluation {
    pub cost       : usize,
    pub violation  : usize,
    pub feasible   : BitSet,
    pub infeasible : BitSet
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MoveType {
    InfeasibleBackward = 1,
    FeasibleForward    = 2,
    FeasibleBackward   = 3,
    InfeasibleForward  = 4,
}

pub struct Moves {
    evaluation: Evaluation,
}

impl Moves {
    pub fn new(evaluation: Evaluation) -> Self {
        Self{evaluation}
    }
    pub fn iter(&self) -> impl Iterator<Item = (MoveType, usize)> + '_ {
        let infeasible_bw = self.evaluation.infeasible.ones().map(|i| (MoveType::InfeasibleBackward, i));
        let feasible_fw   = self.evaluation.infeasible.ones().map(|i| (MoveType::FeasibleForward,    i));
        let feasible_bw   = self.evaluation.infeasible.ones().map(|i| (MoveType::FeasibleBackward,   i));
        let infeasible_fw = self.evaluation.infeasible.ones().map(|i| (MoveType::InfeasibleForward,  i));
        infeasible_bw.chain(feasible_fw).chain(feasible_bw).chain(infeasible_fw)
    }
}

#[derive(Debug, Clone)]
pub struct Before {
    pred: Vec<BitSet>
}
impl Before {
    pub fn new(tsptw: &Tsptw) -> Self {
        let mut pred = vec![BitSet::empty(); tsptw.n_client];
        for (i, pred_i) in pred.iter_mut().enumerate() {
            for j in 0..tsptw.n_client {
                if i == j {
                    continue;
                } else {
                    let dist = tsptw.dist[(i, j)];
                    let arr  = tsptw.tw[i].earliest.saturating_add(dist);
                    let end  = tsptw.tw[j].latest;
                    if arr > end {
                        pred_i.add(j);
                    }
                }
            }
        }
        Self{pred}
    }
    /// must x be before y ?
    pub fn is_before(&self, x: usize, y: usize) -> bool {
        self.pred[y].contains(x)
    }
}