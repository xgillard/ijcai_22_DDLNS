use std::{
    cmp::Ordering,
    fmt::Display,
    hash::Hash,
    ops::{Deref, Index},
    time::Duration, num::ParseIntError, str::FromStr,
};

// ----------------------------------------------------------------------------
/// Variable
// ----------------------------------------------------------------------------
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Var(usize);
impl Var {
    pub fn new(x: usize) -> Self {
        Self(x)
    }
    pub fn id(self) -> usize {
        self.0
    }
}
// ----------------------------------------------------------------------------
/// Decision
// ----------------------------------------------------------------------------
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Decision {
    pub var: Var,
    pub val: isize,
}
impl Decision {
    pub fn new(var: Var, val: isize) -> Self {
        Self { var, val }
    }
}
// ----------------------------------------------------------------------------
/// Solution
// ----------------------------------------------------------------------------
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Solution {
    data: Vec<isize>,
}
impl Solution {
    pub fn iter(&self) -> impl ExactSizeIterator<Item = Decision> + '_ {
        self.data
            .iter()
            .copied()
            .enumerate()
            .map(|(i, val)| Decision { var: Var(i), val })
    }
}
impl Index<Var> for Solution {
    type Output = isize;
    fn index(&self, index: Var) -> &Self::Output {
        &self.data[index.0]
    }
}
impl<T> From<T> for Solution
where
    T: Iterator<Item = Decision>,
{
    fn from(it: T) -> Self {
        let decisions = it.collect::<Vec<Decision>>();
        let mut data  = vec![0; decisions.len()];
        for d in decisions {
            data[d.var.0] = d.val;
        }
        Self { data }
    }
}

impl FromStr for Solution {
    type Err = ParseIntError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let mut data = vec![];
        for token in value.split_ascii_whitespace() {
            let decision  = token.parse::<isize>()?;
            data.push(decision);
        }
        Ok(Self {data})
    }
}

impl Display for Solution {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for i in self.data.iter() {
            write!(f, "{} ", i)?;
        }
        Ok(())
    }
}
// ----------------------------------------------------------------------------
/// Resolution status
// ----------------------------------------------------------------------------
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResolutionStatus {
    /// I haven't proved optimality yet
    Open{improved: bool},
    /// I was able to prove optimality
    Closed{improved: bool},
}

impl ResolutionStatus {
    pub fn to_str(self) -> &'static str {
        match self {
            ResolutionStatus::Open{improved} => 
                if improved { "open(improved)"   } else { "open(initial)" },
            ResolutionStatus::Closed{improved} => 
                if improved { "closed(improved)" } else { "closed(initial)" },
        }
    }
}
// ----------------------------------------------------------------------------
/// Resolution outcome: the result of attempting to solve the problem with a
/// given method
// ----------------------------------------------------------------------------
#[derive(Clone)]
pub struct ResolutionOutcome {
    pub status: ResolutionStatus,
    pub best_value: Option<isize>,
    pub best_sol: Option<Solution>,
    pub time_to_best: Option<Duration>,
    pub time_to_prove: Option<Duration>,
}
// ----------------------------------------------------------------------------
/// Variable Ordering
// ----------------------------------------------------------------------------
pub trait VariableOrdering {
    type State;

    fn next(&self, state: &mut dyn Iterator<Item = &Self::State>) -> Option<Var>;
}
// ----------------------------------------------------------------------------
/// Problem definition
// ----------------------------------------------------------------------------
pub trait Problem {
    type State: PartialEq + Eq + Hash;

    fn nb_vars(&self) -> usize;
    fn initial_state(&self) -> Self::State;
    fn initial_value(&self) -> isize;
    //
    fn for_each_in_domain(&self, state: &Self::State, var: Var, f: impl FnMut(Decision));

    fn transition(&self, state: &Self::State, decision: Decision) -> Self::State;
    fn transition_cost(&self, state: &Self::State, decision: Decision) -> isize;

    // rough lower bound
    fn estimate(&self, _state: &Self::State) -> isize {
        isize::MIN
    }

    fn evaluate(&self, var_ord: &dyn VariableOrdering<State=Self::State>, sol: &Solution) -> isize {
        let mut state = self.initial_state();
        let mut cost = self.initial_value();
        while let Some(var) = var_ord.next(&mut std::iter::once(&state)) {
            let val = sol[var];
            let decision = Decision::new(var, val);
            cost += self.transition_cost(&state, decision);
            state = self.transition(&state, decision);
        }
        cost
    }

    fn details(&self, var_ord: &dyn VariableOrdering<State=Self::State>, sol: &Solution) {
        let mut state = self.initial_state();
        while let Some(var) = var_ord.next(&mut std::iter::once(&state)) {
            let val = sol[var];
            let decision = Decision::new(var, val);
            self.decision_details(&state, decision);
            state = self.transition(&state, decision);
        }
    }

    fn check(&self, var_ord: &dyn VariableOrdering<State=Self::State>, sol: &Solution) {
        let mut state = self.initial_state();
        while let Some(var) = var_ord.next(&mut std::iter::once(&state)) {
            let val = sol[var];
            let decision = Decision::new(var, val);
            let mut in_domain = false;
            self.for_each_in_domain(&state, decision.var, |d| in_domain |= d == decision);
            
            if !in_domain {
                self.on_violation(&state, decision);
            }

            state = self.transition(&state, decision);
        }
    }

    fn on_violation(&self, _state: &Self::State, decision: Decision) {
        println!("violation x{} <-- {}", decision.var.id(), decision.val);
    }

    fn decision_details(&self, _state: &Self::State, decision: Decision) {
        println!("x{} <-- {}", decision.var.id(), decision.val);
    }
}

// ----------------------------------------------------------------------------
/// Node Selection Heurisic (as a comparator)
// ----------------------------------------------------------------------------
pub trait Mdd {
    type State: Eq + PartialEq + Hash;

    fn get_best_value(&self) -> Option<isize>;
    fn get_best_solution(&self) -> Option<Solution>;
    fn is_exact(&self) -> bool;
    fn exact(&mut self) -> Option<isize>;

    fn restricted(
        &mut self,
        max_width: usize,
        best_val: isize,
        best_sol: &Option<Solution>,
        start_depth: usize,
    ) -> Option<isize>;
}

// ----------------------------------------------------------------------------
/// Node Selection Heurisic (as a comparator)
// ----------------------------------------------------------------------------
pub trait NodeSource {
    type State;
    type Node: SelectableNode<State = Self::State>;
    type Path: Iterator<Item = Decision>;

    /// Returns the path from root to this node within the given source structure
    /// `within` is typically the mdd from which the selectable node originates
    fn path(&self, node: &Self::Node) -> Self::Path;
}
pub trait SelectableNode {
    type State;

    /// Returns the state of the node
    fn state(&self) -> &Self::State;
    /// Tells the best objective value of the problem when considered at this
    /// specific node
    fn value(&self) -> isize;
    /// Estimates the best objective on the remaining sub problem
    fn estimate(&self) -> isize;
}
pub trait NodeSelectionHeuristic {
    /// An optional method that tells whether or not we want to force the
    /// node compilation proceedure to keep the given node while expanding further
    /// layers
    fn is_mandatory<S: NodeSource>(
        &self,
        _dd: &S,
        _node: &S::Node,
        _last_var: Var,
        _best_sol: &Option<Solution>,
    ) -> bool {
        false
    }
    /// This method helps defines a custom ordering on the selectable nodes.
    /// That order specifies the likelihood for a node to remain in the decision
    /// diagram after a restriction is performed. An earlier rank
    /// (Ordering::Less) means that the node is more likely to stay in the dd
    /// after restriction occured.
    fn compare<S: NodeSource>(&self, dd: &S, na: &S::Node, nb: &S::Node) -> Ordering;
}

#[derive(Default, Debug, Clone, Copy)]
pub struct MinLP;
impl NodeSelectionHeuristic for MinLP {
    fn compare<S: NodeSource>(&self, _dd: &S, na: &S::Node, nb: &S::Node) -> Ordering {
        na.value().cmp(&nb.value())
    }
}

#[derive(Debug, Clone, Copy)]
pub struct KeepThemAll;
impl NodeSelectionHeuristic for KeepThemAll {
    fn compare<S: NodeSource>(&self, _dd: &S, na: &S::Node, nb: &S::Node) -> Ordering {
        // never ever occurs
        na.value().cmp(&nb.value())
    }
    fn is_mandatory<S: NodeSource>(
        &self,
        _dd: &S,
        _node: &S::Node,
        _last_var: Var,
        _best_sol: &Option<Solution>,
    ) -> bool {
        true
    }
}

// ----------------------------------------------------------------------------
// Boilerplate to make any reference to a problem into a problem itself
// ----------------------------------------------------------------------------
impl<P, D> Problem for D
where
    P: Problem,
    D: Deref<Target = P>,
{
    type State = P::State;

    fn nb_vars(&self) -> usize {
        self.deref().nb_vars()
    }
    fn initial_state(&self) -> Self::State {
        self.deref().initial_state()
    }
    fn initial_value(&self) -> isize {
        self.deref().initial_value()
    }
    fn for_each_in_domain(&self, state: &Self::State, var: Var, f: impl FnMut(Decision)) {
        self.deref().for_each_in_domain(state, var, f)
    }
    fn transition(&self, state: &Self::State, decision: Decision) -> Self::State {
        self.deref().transition(state, decision)
    }
    fn transition_cost(&self, state: &Self::State, decision: Decision) -> isize {
        self.deref().transition_cost(state, decision)
    }
    fn estimate(&self, state: &Self::State) -> isize {
        self.deref().estimate(state)
    }
    fn evaluate(&self, var_ord: &dyn VariableOrdering<State=Self::State>, sol: &Solution) -> isize {
        self.deref().evaluate(var_ord, sol)
    }
}
