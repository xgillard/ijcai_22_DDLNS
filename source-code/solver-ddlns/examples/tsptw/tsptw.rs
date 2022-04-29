use std::{
    fs::File,
    io::{BufRead, BufReader, Lines, Read},
    num::{ParseFloatError, ParseIntError}, intrinsics::transmute,
};

use papier_lns::{
    Decision, Matrix, NodeSelectionHeuristic, Problem, SelectableNode, Var, VariableOrdering,
};

use crate::{BitSet256, before::Before};

#[derive(Debug, thiserror::Error)]
pub enum TsptwError {
    #[error("io error {0}")]
    Io(#[from] std::io::Error),
    #[error("n_cities is not a valid number {0}")]
    NbCities(ParseIntError),
    #[error("matrix coefficient ({0},{1}) = {2}")]
    MatrixCoeff(usize, usize, ParseFloatError),
    #[error("start of time window {0}")]
    TwStart(ParseFloatError),
    #[error("stop of time window {0}")]
    TwStop(ParseFloatError),
    #[error("no tw start (line: {0})")]
    NoTwStart(usize),
    #[error("no tw stop (line: {0})")]
    NoTwStop(usize),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TimeWindow {
    pub start: usize,
    pub stop: usize,
}

#[derive(Debug, Clone)]
pub struct Tsptw {
    pub n_cities: usize,
    pub distance: Matrix<usize>,
    pub time_window: Vec<TimeWindow>,
    //
    pub before: Before
}

/*******************************************************************************/
/**** Implement Problem ********************************************************/
/*******************************************************************************/
static DEPOT: usize = 0;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct State {
    pub time: usize,
    pub current: usize,
    pub visit: BitSet256,
}

impl Problem for Tsptw {
    type State = State;

    fn nb_vars(&self) -> usize {
        self.n_cities
    }

    fn initial_state(&self) -> Self::State {
        let mut visit = BitSet256::empty();
        for x in 0..self.n_cities {
            // if there are 20 cities, 20 is not one to visit
            visit.add(x);
        }
        State {
            time: 0,
            current: DEPOT,
            visit,
        }
    }

    fn initial_value(&self) -> isize {
        0
    }

    fn for_each_in_domain(&self, state: &Self::State, var: Var, mut f: impl FnMut(Decision)) {
        if state.visit == BitSet256::singleton(DEPOT) {
            f(Decision { var, val: 0 })
        } else {
            let dom = state
                .visit
                .iter()
                .skip(1) // skip depot
                .filter(|next| self.can_visit(state, *next));
            for val in dom {
                let val = val as isize;
                f(Decision { var, val });
            }
        }
    }

    fn transition(&self, state: &Self::State, decision: Decision) -> Self::State {
        let destination   = decision.val as usize;
        let early_arrival = state.time + self.distance[(state.current, destination)];
        let arrival       = early_arrival.max(self.time_window[destination].start);

        let mut visit = state.visit; // copy
        visit.remove(destination);

        State {
            time: arrival,
            current: destination,
            visit,
        }
    }

    fn transition_cost(&self, state: &Self::State, decision: Decision) -> isize {
        let destination = decision.val as usize;
        self.distance[(state.current, destination)] as isize
    }

    fn estimate(&self, state: &Self::State) -> isize {
        if state.visit == BitSet256::singleton(DEPOT) {
            self.distance[(state.current, DEPOT)] as isize
        } else {
            // 3 steps in this estimate: 
            // find the distance to closest neighbor.
            // cost of an mst comprising only those nodes remaining to visit
            // find the shortest feasible path between any of the remaining 
            // nodes to the depot
            // 
            // at each time, we can detect infeasibility and hence return a
            // prohibitive (= + inf, = isize::MAX) estimate for this state; 
            // thereby meaning the state should not be explored

            let mut cities = state.visit;
            cities.remove(DEPOT);

            // ---  STEP 1: find closest to current node -----------------------
            let mut min_dist_1 = usize::MAX;
            let mut min_arr_1  = usize::MAX;

            for city in cities.iter() {
                let d  = self.distance[(state.current, city)];
                let a  = self.time_window[city].start.max(state.time + d);

                min_dist_1 = min_dist_1.min(d);
                min_arr_1  = min_arr_1.min(a);
            }

            // ---  STEP 2: cost of an mst -------------------------------------
            let mut mst  = 0_usize;
            let mut done = BitSet256::empty();
            for x in cities.iter() {
                let mut dist = usize::MAX;
                let mut neigh= x;
                if !done.contains(x) {
                    for y in cities.iter() {
                        if x != y {
                            let twx = self.time_window[x];
                            let twy = self.time_window[y];
                            
                            let dxy = self.distance[(x, y)];
                            let dyx = self.distance[(y, x)];

                            // consider a distance iff it is possibly feasible
                            if dxy < dist && min_arr_1.max(twx.start).saturating_add(dxy) < twy.stop {
                                dist = dxy;
                                neigh= y;
                            }
                            if dyx < dist && min_arr_1.max(twy.start).saturating_add(dyx) < twx.stop {
                                dist = dyx;
                                neigh= y;
                            }
                        }
                    }
                    mst = mst.saturating_add(dist);
                    done.add(x);
                    done.add(neigh);
                }
            }

            // ---  STEP 3: cost of an mst -------------------------------------
            let early_time = min_arr_1.saturating_add(mst);
            let mut min_dist_3 = usize::MAX;
            for x in cities.iter() {
                let twx  = self.time_window[x];
                let twd  = self.time_window[DEPOT];
                let dist = self.distance[(x, DEPOT)];
                let arr  = early_time.max(twx.start);

                if arr < twx.stop && arr.saturating_add(dist) < twd.stop {
                    min_dist_3 = min_dist_3.min(dist);
                }
            }

            min_dist_1.saturating_add(mst).saturating_add(min_dist_3) as isize
        }
    }

    fn on_violation(&self, state: &Self::State, decision: Decision) {
        let src  = state.current;
        let dst  = decision.val as usize;
        let time = state.time;
        let tw   = self.time_window[dst];
        let dist = self.distance[(src, dst)];

        println!("violation x{:<4} <-- {:>3} || dept time = {:>9.4} ; dist = {:>9.4} ; arrival = {:>9.4} ; tw = [{:>9.4} - {:>9.4}]", 
            decision.var.id(), decision.val,
            time as f64 / 10000.0,
            dist as f64 / 10000.0,
            tw.start.max(time + dist) as f64 / 10000.0,
            tw.start as f64 / 10000.0, tw.stop as f64 / 10000.0,
        );
    }

    fn decision_details(&self, state: &Self::State, decision: Decision) {
        let src  = state.current;
        let dst  = decision.val as usize;
        let time = state.time;
        let tw   = self.time_window[dst];
        let dist = self.distance[(src, dst)];
        let arr  = tw.start.max(time + dist);

        print!("arrival = {:>9.4} ; tw = [{:>9.4} - {:>9.4}]; depart = {:>9.4} ; dist = {:>9.4} ", 
            arr as f64 / 10000.0,
            tw.start as f64 / 10000.0, tw.stop as f64 / 10000.0,
            time as f64 / 10000.0,
            dist as f64 / 10000.0,
        );
        if arr > tw.stop {
            println!(" !! violation");
        } else {
            println!();
        }
    }
}

impl Tsptw {
    fn can_visit(&self, state: &State, next: usize) -> bool {
        let mut cities = state.visit;
        cities.remove(DEPOT);
        cities.remove(next);
        if self.before.any_before(cities, next) {
            false
        } else {
            let arrival = state.time + self.distance[(state.current, next)];
            arrival <= self.time_window[next].stop
        }
    }
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub struct LeftToRight(pub usize);
impl VariableOrdering for LeftToRight {
    type State = State;

    fn next(&self, states: &mut dyn Iterator<Item = &Self::State>) -> Option<Var> {
        let to_visit = states.next().unwrap().visit.len();
        let varid = self.0 - to_visit;
        if varid < self.0 {
            Some(Var::new(varid))
        } else {
            None
        }
    }
}

#[derive(Clone, Copy)]
pub struct RandomizedMinLP<'a> {
    inst: &'a Tsptw
}
impl <'a> RandomizedMinLP<'a> {
    pub fn new(inst: &'a Tsptw) -> Self {
        Self { inst }
    }

    pub fn total_openness(&self, state: &State) -> isize {
        let now = state.time;
        let mut tot = 0;
        for x in state.visit.iter() {
            let tw = self.inst.time_window[x];
            let op = tw.stop as isize - tw.start.max(now) as isize;
            if op < 0 {
                return isize::MIN;
            } else {
                tot += op;
            }
        }
        tot 
    }
}
impl NodeSelectionHeuristic for RandomizedMinLP<'_> {
    fn compare<S: papier_lns::NodeSource>(
        &self,
        _dd: &S,
        na: &S::Node,
        nb: &S::Node,
    ) -> std::cmp::Ordering {
        /*
        let va = na.value();
        let vb = nb.value();
        let ea = na.estimate();
        let eb = nb.estimate();
        let ta = va.saturating_add(ea);
        let tb = vb.saturating_add(eb);
        
        ta.cmp(&tb).then(va.cmp(&vb))
        */
        /**/
        unsafe {
            let sa : &State = transmute(na.state());
            let sb : &State = transmute(nb.state());

            let ca = sa.current;
            let cb = sb.current;
            let twa= self.inst.time_window[ca];
            let twb= self.inst.time_window[cb];

            let oa     = self.total_openness(sa);
            let ob     = self.total_openness(sb);

            let va     = na.value();
            let ea     = na.estimate();
            let vb     = nb.value();
            let eb     = nb.estimate();
            let ta     = va.saturating_add(ea);
            let tb     = vb.saturating_add(eb);

            oa.cmp(&ob).reverse()
            .then(ta.cmp(&tb))
            .then(twa.stop.cmp(&twb.stop))
            .then(va.cmp(&vb))
            .then(twa.start.cmp(&twb.start))
            .then(ea.cmp(&eb))
        }
        /**/
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

/*******************************************************************************/
/**** PARSE INSTANCE ***********************************************************/
/*******************************************************************************/

impl TryFrom<File> for Tsptw {
    type Error = TsptwError;

    fn try_from(file: File) -> Result<Self, Self::Error> {
        Tsptw::try_from(BufReader::new(file))
    }
}
impl<S: Read> TryFrom<BufReader<S>> for Tsptw {
    type Error = TsptwError;

    fn try_from(reader: BufReader<S>) -> Result<Self, Self::Error> {
        Tsptw::try_from(reader.lines())
    }
}
impl<B: BufRead> TryFrom<Lines<B>> for Tsptw {
    type Error = TsptwError;

    fn try_from(lines: Lines<B>) -> Result<Self, Self::Error> {
        let mut lc = 0;
        let mut nb_nodes = 0;
        let mut distances = Matrix::new_default(nb_nodes, nb_nodes, 0);
        let mut timewindows = vec![];

        for line in lines {
            let line = line?;
            let line = line.trim();

            // skip comment lines
            if line.starts_with('#') || line.is_empty() {
                continue;
            }

            // First line is the number of nodes
            if lc == 0 {
                nb_nodes = line.parse::<usize>().map_err(TsptwError::NbCities)?;
                distances = Matrix::new_default(nb_nodes, nb_nodes, 0);
            }
            // The next 'nb_nodes' lines represent the distances matrix
            else if (1..=nb_nodes).contains(&lc) {
                let i = (lc - 1) as usize;
                for (j, distance) in line.split_whitespace().enumerate() {
                    let distance = distance
                        .to_string()
                        .parse::<f32>()
                        .map_err(|e| TsptwError::MatrixCoeff(i, j, e))?;
                    let distance = (distance * 10000.0) as usize;
                    distances[(i, j)] = distance;
                }
            }
            // Finally, the last 'nb_nodes' lines impose the time windows constraints
            else {
                let mut tokens = line.split_whitespace();
                let earliest = if let Some(earliest) = tokens.next() {
                    earliest.parse::<f32>().map_err(TsptwError::TwStart)?
                } else {
                    return Err(TsptwError::NoTwStart(lc));
                };

                let latest = if let Some(latest) = tokens.next() {
                    latest.parse::<f32>().map_err(TsptwError::TwStop)?
                } else {
                    return Err(TsptwError::NoTwStop(lc));
                };

                let earliest = (earliest * 10000.0) as usize;
                let latest = (latest * 10000.0) as usize;

                let timewind = TimeWindow {
                    start: earliest,
                    stop: latest,
                };
                timewindows.push(timewind);
            }
            lc += 1;
        }

        let before = Before::new(nb_nodes, &distances, &timewindows);

        Ok(Tsptw {
            n_cities: nb_nodes,
            distance: distances,
            time_window: timewindows,
            //
            before,
        })
    }
}
