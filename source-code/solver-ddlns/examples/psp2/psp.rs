use std::{
    cell::RefCell,
    fmt::Debug,
    fs::File,
    io::{BufRead, BufReader, Lines, Read},
};

use papier_lns::{
    Decision, Matrix, NodeSelectionHeuristic, Problem, SelectableNode, Solution, Var,
    VariableOrdering,
};

use smallbitset::Set32;

static BOT: i32 = -1;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct State {
    time: usize,
    k: i32,
    // for each item i, req[i] denotes the time when the current order must be
    // delivered.
    u: Vec<i32>,
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub struct LeftToRight;
impl VariableOrdering for LeftToRight {
    type State = State;

    fn next(&self, states: &mut dyn Iterator<Item = &Self::State>) -> Option<Var> {
        let time = states.next().unwrap().time;
        if time > 0 {
            Some(Var::new(time - 1))
        } else {
            None
        }
    }
}

#[derive(Clone, Default)]
pub struct RandomizedMinLP;
impl NodeSelectionHeuristic for RandomizedMinLP {
    fn compare<S: papier_lns::NodeSource>(
        &self,
        _dd: &S,
        na: &S::Node,
        nb: &S::Node,
    ) -> std::cmp::Ordering {
        let a = na.value().saturating_add(na.estimate());
        let b = nb.value().saturating_add(nb.estimate());
        a.cmp(&b)
    }

    fn is_mandatory<S: papier_lns::NodeSource>(
        &self,
        dd: &S,
        node: &S::Node,
        last_var: Var,
        best_sol: &Option<papier_lns::Solution>,
    ) -> bool {
        if let Some(best_sol) = best_sol {
            if let Some(last_decision) = dd.path(node).next() {
                let best_decision = best_sol[last_var];
                if last_decision.val == best_decision {
                    return true;
                }
            }
        }
        false
    }
}

#[derive(Clone, Debug)]
pub struct Psp {
    pub optimum: Option<usize>,
    pub nb_periods: usize,
    pub nb_items: usize,
    pub nb_orders: usize,
    pub changeover_cost: Matrix<usize>,
    pub stocking_cost: Vec<usize>,
    // le précédent/suivant est -1 lorsqu'il n'ya plus de deadline
    pub prev_demand: Matrix<i32>,

    pub mst: Vec<usize>,

    buffer_state: RefCell<Vec<i32>>,
    buffer_time: RefCell<Vec<usize>>,
}

impl Problem for Psp {
    type State = State;

    fn nb_vars(&self) -> usize {
        self.nb_periods
    }

    fn initial_state(&self) -> State {
        let u = Vec::from_iter(self.prev_demand.col(self.nb_periods).copied());
        State {
            time: self.nb_periods,
            k: BOT,
            u,
        }
    }

    fn initial_value(&self) -> isize {
        0
    }

    fn for_each_in_domain(&self, state: &Self::State, var: Var, mut f: impl FnMut(Decision)) {
        let time = var.id();
        let dom = (0..self.nb_items as isize).filter(move |i| state.u[*i as usize] >= time as i32);
        for val in dom {
            f(Decision { var, val })
        }
    }
    fn transition(&self, state: &Self::State, decision: Decision) -> Self::State {
        let item = decision.val as usize;
        let mut next = state.clone();
        next.time -= 1;
        next.k = item as i32;
        next.u[item] = self.prev_demand[(item, state.u[item] as usize)];
        next
    }

    fn transition_cost(&self, state: &Self::State, decision: Decision) -> isize {
        let time = decision.var.id();
        let item = decision.val as usize;
        let changeover = if state.k == BOT {
            0
        } else {
            self.changeover_cost[(item, state.k as usize)]
        };
        let stocking = self.stocking_cost[item] * (state.u[item] as usize - time);
        (changeover + stocking) as isize
    }

    fn estimate(&self, state: &Self::State) -> isize {
        if state.time == 0 {
            0
        } else {
            // This is ugly as a sin: but it works like hell !
            // I simply copy the current state in a buffer which I then pass on to
            // the greedy estimate computation function. Also, I pass on a mutable
            // pointer to the 'mut_time' which is used during the computation of
            // the optimal stocking plan
            let mut mut_time = self.buffer_time.borrow_mut();
            let mut mut_state = self.buffer_state.borrow_mut();
            mut_state
                .iter_mut()
                .zip(state.u.iter())
                .for_each(|(d, s)| *d = *s);
            let greedy = Self::compute_ideal_stocking(
                state.time,
                mut_state.as_mut(),
                mut_time.as_mut(),
                &self.prev_demand,
                &self.stocking_cost,
            );

            let idx: u32 = Self::vertices(state.k, &state.u).into();
            let mst = self.mst[idx as usize];
            let stock = greedy;

            (stock + mst) as isize
        }
    }
}

impl Psp {
    /*** ESTIMATION ON THE STOCKING COSTS ***************************************/
    fn compute_ideal_stocking(
        periods: usize,
        state: &mut [i32],
        buffer_time: &mut [usize],
        prev_dem: &Matrix<i32>,
        stocking: &[usize],
    ) -> usize {
        for (time, storage_cost) in buffer_time.iter_mut().enumerate().take(periods).rev() {
            let mut item = None;
            let mut deadline = None;
            let mut cost = None;

            for (state_item, state_deadline) in state.iter().enumerate() {
                if *state_deadline >= time as i32 {
                    if let Some(c_cost) = cost {
                        if stocking[state_item] >= c_cost {
                            item = Some(state_item);
                            deadline = Some(*state_deadline);
                            cost = Some(stocking[state_item]);
                        }
                    } else {
                        item = Some(state_item);
                        deadline = Some(*state_deadline);
                        cost = Some(stocking[state_item]);
                    }
                }
            }
            let item = item.unwrap();
            let cost = cost.unwrap();
            let deadline = deadline.unwrap() as usize;
            *storage_cost = (deadline - time) * cost;
            state[item] = prev_dem[(item, deadline)];
        }

        // Cumulative sum
        let mut tot : usize = 0;
        for v in buffer_time.iter_mut() {
            tot = tot.saturating_add(*v);
            *v = tot;
        }

        buffer_time[periods - 1]
    }

    /*** ESTIMATION ON THE CHANGEOVER COSTS *************************************/
    fn vertices(prev: i32, requests: &[i32]) -> Set32 {
        let mut vertices = Set32::empty();
        if prev != -1 {
            vertices = vertices.insert(prev as u8);
        }
        for (i, v) in requests.iter().copied().enumerate() {
            if v >= 0 {
                vertices = vertices.insert(i as u8);
            }
        }
        vertices
    }
    fn precompute_all_mst(n_vars: usize, changeover: &Matrix<usize>) -> Vec<usize> {
        let len = 2_usize.pow(n_vars as u32);
        let mut out = vec![0; len];

        let mut heap = vec![];
        for (i, v) in out.iter_mut().enumerate() {
            *v = Self::mst(Set32::from(i as u32), changeover, &mut heap);
        }

        out
    }
    fn mst(
        mut vertices: Set32,
        changeover: &Matrix<usize>,
        heap: &mut Vec<(usize, u8, u8)>,
    ) -> usize {
        for i in vertices {
            for j in vertices {
                if i != j {
                    let a = i as usize;
                    let b = j as usize;
                    let edge = changeover[(a, b)].min(changeover[(b, a)]);
                    heap.push((edge, i, j));
                }
            }
        }
        heap.sort_unstable_by_key(|x| x.0);
        let mut total = 0;
        let mut edge_max  = 0;
        let mut iter_heap = heap.iter();
        while !vertices.is_empty() {
            if let Some(edge) = iter_heap.next() {
                let l = edge.0;
                let i = edge.1;
                let j = edge.2;

                if vertices.contains(i) || vertices.contains(j) {
                    edge_max = edge_max.max(l);
                    total += l;
                    vertices = vertices.remove(i);
                    vertices = vertices.remove(j);
                }
            } else {
                break;
            }
        }

        total - edge_max
    }
}

/*** GREEDY PROBLEM RESOLUTION ***********************************************/
impl Psp {
    /// Solve it as a plain Wagner-Within, scheduling the most expensive task to
    /// store as late as possible.
    pub fn greedy(&self) -> (isize, Option<Solution>) {
        let mut cost = self.initial_value();
        let mut solution = vec![];

        let mut state = self.initial_state();
        let nb_vars = self.nb_vars();

        for time in (0..nb_vars).rev() {
            let var = Var::new(time);
            let mut dec: Option<Decision> = None;
            self.for_each_in_domain(&state, var, |d| {
                if let Some(kept) = dec {
                    if self.stocking_cost[d.val as usize] > self.stocking_cost[kept.val as usize] {
                        dec = Some(d);
                    }
                } else {
                    dec = Some(d);
                }
            });

            if let Some(dec) = dec {
                solution.push(dec);
                cost = cost.saturating_add(self.transition_cost(&state, dec));
                state = self.transition(&state, dec);
            } else {
                return (isize::MAX, None);
            }
        }
        (cost, Some(Solution::from(solution.iter().copied())))
    }
}
/*** BELOW THIS LINE IS THE CODE TO PARSE INSTANCE FILES *********************/
#[derive(Debug, thiserror::Error)]
pub enum PspError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("missing {0}")]
    Missing(&'static str),
    #[error("expected int {0}")]
    ParseInt(#[from] std::num::ParseIntError),
}
impl TryFrom<File> for Psp {
    type Error = PspError;

    fn try_from(file: File) -> Result<Psp, PspError> {
        Psp::try_from(BufReader::new(file))
    }
}
impl<S: Read> TryFrom<BufReader<S>> for Psp {
    type Error = PspError;

    fn try_from(buf: BufReader<S>) -> Result<Psp, PspError> {
        Psp::try_from(buf.lines())
    }
}
impl<B: BufRead> TryFrom<Lines<B>> for Psp {
    type Error = PspError;

    fn try_from(mut lines: Lines<B>) -> Result<Psp, PspError> {
        let nb_periods = lines
            .next()
            .ok_or(PspError::Missing("nb periods"))??
            .parse::<usize>()?;
        let nb_items = lines
            .next()
            .ok_or(PspError::Missing("nb items"))??
            .parse::<usize>()?;
        let nb_orders = lines
            .next()
            .ok_or(PspError::Missing("nb orders"))??
            .parse::<usize>()?;

        let _blank = lines.next();
        let mut changeover_cost = Matrix::new_default(nb_items, nb_items, 0);

        let mut i = 0;
        for line in &mut lines {
            let line = line?;
            let line = line.trim();
            if line.is_empty() {
                break;
            }

            let costs = line.split_whitespace();
            for (other, cost) in costs.enumerate() {
                changeover_cost[(i, other)] = cost.parse::<usize>()?;
            }

            i += 1;
        }

        let stocking_texts = lines.next().ok_or(PspError::Missing("stocking costs"))??;
        let mut stocking_cost = vec![0; nb_items];
        let stock_iter = stocking_cost
            .iter_mut()
            .zip(stocking_texts.split_whitespace());

        for (cost, text) in stock_iter {
            *cost = text.parse::<usize>()?;
        }

        let _blank = lines.next();

        let mut prev_demand = Matrix::new(nb_items, nb_periods + 1);
        i = 0;
        for line in &mut lines {
            let line = line?;
            let line = line.trim();

            if line.is_empty() {
                break;
            }

            let demands_for_item = line.split_whitespace();

            // on construit la relation prev_demand[i]
            let mut last_period = BOT;
            for (period, demand_text) in demands_for_item.enumerate() {
                prev_demand[(i, period)] = last_period;

                let demand = demand_text.parse::<usize>()?;
                if demand > 0 {
                    last_period = period as i32;
                }

                if period == nb_periods - 1 {
                    prev_demand[(i, 1 + period)] = last_period;
                }
            }

            i += 1;
        }

        // This means there mus be TWO blank lines between the end of demands
        // and the known optimum.
        let _skip = lines.next();
        let optimum = if let Some(line) = lines.next() {
            Some(line?.trim().parse::<usize>()?)
        } else {
            None
        };

        let mst = Psp::precompute_all_mst(nb_items, &changeover_cost);

        Ok(Psp {
            optimum,
            nb_periods,
            nb_items,
            nb_orders,
            changeover_cost,
            stocking_cost,
            prev_demand,

            mst,

            buffer_state: RefCell::new(vec![0; nb_items]),
            buffer_time: RefCell::new(vec![0; nb_periods]),
        })
    }
}
