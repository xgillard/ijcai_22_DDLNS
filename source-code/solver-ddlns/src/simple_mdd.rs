//! Ici je vais implémenter une stucture de MDD

use crate::{
    Decision, NodeSelectionHeuristic, NodeSource, Problem, SelectableNode, Solution,
    VariableOrdering, Mdd, Var,
};
use derive_builder::Builder;
use rand::{Rng, SeedableRng};
use rand_xoshiro::Xoshiro256Plus;
use rustc_hash::FxHashMap;
use std::{
    collections::hash_map::Entry,
    hash::Hash,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Ord, PartialOrd)]
struct NodeId(usize);

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Ord, PartialOrd)]
struct Edge {
    from: NodeId,
    to: NodeId,
    label: Decision,
    weight: isize,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Ord, PartialOrd)]
struct Node {
    my_id: NodeId,
    value: isize,
    best_parent: Option<Edge>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct MiniNode<S> {
    node_id: NodeId,
    state: S,
    value: isize,
    estimate: isize,
}

/// used to pass info related to the initial state and value
struct Initial<P: Problem> {
    state: P::State,
    value: isize,
}
/// Pass information related to the incumbent best solution
struct Incumbent<'a> {
    best_val: isize,
    best_sol: &'a Option<Solution>,
}
/// Pass configuration information
struct Config<'a, P, V, N>
where
    P: Problem,
    V: VariableOrdering,
    N: NodeSelectionHeuristic,
{
    problem: &'a P,
    var_ord: &'a V,
    node_sel: &'a N,
    //
    rng: &'a mut Xoshiro256Plus,
    proba: f64,
    //
    max_width: usize,
    kill_switch: &'a AtomicBool,
    //
    start_depth: usize,
}

#[derive(Builder)]
pub struct SimpleMdd<P, V, N>
where
    P: Problem,
    P::State: PartialEq + Eq + Hash,
    V: VariableOrdering<State = P::State>,
    N: NodeSelectionHeuristic,
{
    problem: P,
    var_ordering: V,
    node_selection: N,
    kill_switch: Arc<AtomicBool>,

    #[builder(setter(skip))]
    diagram: Diagram<P>,
    #[builder(default="Xoshiro256Plus::seed_from_u64(0)")]
    rng: Xoshiro256Plus,
    proba: f64,
}
impl <P, V, N> SimpleMdd<P, V, N> 
where
    P: Problem,
    P::State: PartialEq + Eq + Hash,
    V: VariableOrdering<State = P::State>,
    N: NodeSelectionHeuristic,
{
    pub fn get_proba(&self) -> f64 {
        self.proba
    }
    pub fn set_proba(&mut self, proba: f64) {
        self.proba = proba;
    }
}
impl<P, V, N> Mdd for SimpleMdd<P, V, N>
where
    P: Problem,
    P::State: PartialEq + Eq + Hash,
    V: VariableOrdering<State = P::State>,
    N: NodeSelectionHeuristic,
{
    type State = P::State;

    fn get_best_value(&self) -> Option<isize> {
        self.diagram.get_best_value()
    }
    fn get_best_solution(&self) -> Option<Solution> {
        self.diagram.get_best_solution()
    }
    fn is_exact(&self) -> bool {
        self.diagram.is_exact
    }

    fn exact(&mut self) -> Option<isize> {
        let config = Config {
            problem: &self.problem,
            var_ord: &self.var_ordering,
            node_sel: &self.node_selection,
            //
            rng: &mut self.rng,
            proba: self.proba,
            //
            max_width: usize::MAX,
            kill_switch: self.kill_switch.as_ref(),
            start_depth: 0,
        };

        let initial = Initial {
            state: self.problem.initial_state(),
            value: self.problem.initial_value(),
        };
        //
        let incumbent = Incumbent {
            best_val: isize::MAX,
            best_sol: &None,
        };
        //
        self.diagram.compile(config, initial, incumbent);
        self.diagram.get_best_value()
    }

    fn restricted(
        &mut self,
        max_width: usize,
        best_val: isize,
        best_sol: &Option<Solution>,
        //
        start_depth: usize
    ) -> Option<isize> {
        let config = Config {
            problem: &self.problem,
            var_ord: &self.var_ordering,
            node_sel: &self.node_selection,
            //
            rng: &mut self.rng,
            proba: self.proba,
            //
            max_width,
            kill_switch: self.kill_switch.as_ref(),
            start_depth
        };

        let initial = Initial {
            state: self.problem.initial_state(),
            value: self.problem.initial_value(),
        };
        //
        let incumbent = Incumbent { best_val, best_sol };
        //
        self.diagram.compile(config, initial, incumbent);
        self.diagram.get_best_value()
    }
}

/// This structure represents the diagram, and the diagram only. It has
/// absolutely no knowledge of the problem -- except for the type of state it
/// holds. The whole point of having this structure instead of cobbling it all
/// in `Mdd` is to be able to prove the safety of the call to `branch_on` within
/// the `for_each_in_domain` closure.
/// (Previous version worked but contained an undefined behavior because of an
/// aliased pointer. [Checked with MIRI]. This verion is therefore safer to use
/// and does not require tricking the borrow checker)
#[derive(Debug, Clone)]
struct Diagram<P>
where
    P: Problem,
    P::State: PartialEq + Eq + Hash,
{
    nodes: Vec<Node>,
    next_layer_states: FxHashMap<P::State, NodeId>,
    best_terminal_node: Option<NodeId>,
    is_exact: bool,
}

impl<P> Default for Diagram<P>
where
    P: Problem,
    P::State: PartialEq + Eq + Hash,
{
    fn default() -> Self {
        Self {
            nodes: vec![],
            next_layer_states: FxHashMap::default(),
            best_terminal_node: None,
            is_exact: true,
        }
    }
}

impl<P> Diagram<P>
where
    P: Problem,
    P::State: PartialEq + Eq + Hash,
{
    fn clear(&mut self) {
        self.nodes.clear();
        self.next_layer_states.clear();
        self.best_terminal_node = None;
        self.is_exact = true;
    }

    fn get_best_value(&self) -> Option<isize> {
        self.best_terminal_node.map(|n| self.nodes[n.0].value)
    }

    fn get_best_solution(&self) -> Option<Solution> {
        self.best_terminal_node.map(|best_id| {
            let mut decisions = vec![];
            let mut curr = self.nodes[best_id.0].best_parent;
            while let Some(edge) = curr {
                decisions.push(edge.label);
                curr = self.nodes[edge.from.0].best_parent;
            }
            Solution::from(decisions.iter().copied())
        })
    }

    fn compile<'a, V, N>(
        &mut self,
        // meta stuffs
        mut config: Config<'a, P, V, N>,
        // initial
        initial: Initial<P>,
        // incumbent
        incumbent: Incumbent,
    ) where
        V: VariableOrdering<State = P::State>,
        N: NodeSelectionHeuristic,
    {
        self.clear();

        self.nodes.push(Node {
            my_id: NodeId(0),
            best_parent: None,
            value: initial.value,
        });

        let mut mininodes = vec![MiniNode {
            node_id: NodeId(0),
            value: initial.value,
            estimate: config.problem.estimate(&initial.state),
            state: initial.state,
        }];

        // Dive if needed
        self.dive_if_needed(&config, &incumbent, &mut mininodes);

        // actually develop the stuff
        loop {
            // when there is no node left to develop (not even a terminal node),
            // then the compilation must abort. We must not even search the best
            // terminal node of the dd as it simply does not exists.
            if mininodes.is_empty() {
                return;
            }

            // clear is redundant because of the above drain
            // self.next_layer_states.clear();
            let mut mininodes_states = mininodes.iter().map(|n| &n.state);
    
            if let Some(var) = config.var_ord.next(&mut mininodes_states) {
                // develop this layer
                for mininode in mininodes.drain(..) {
                    // kill switch short cut
                    if config.kill_switch.load(Ordering::Relaxed) {
                        self.is_exact = false;
                        return;
                    }

                    // rough lower bound check
                    let est = mininode.estimate;
                    let tot = mininode.value.saturating_add(est);
                    // skip if rlb greater than best bound
                    if tot < incumbent.best_val {
                        config
                            .problem
                            .for_each_in_domain(&mininode.state, var, |decision| {
                                self.branch_on(
                                    config.kill_switch,
                                    true,
                                    config.problem,
                                    &mininode,
                                    decision,
                                );
                            });
                    }
                }
                // The next layer has been fully expanded. Let us now drain the hash
                // map and restrict that next layer if needed (to that end, we first
                // need to populate the mininodes vector)
                for e in self.next_layer_states.drain() {
                    mininodes.push(MiniNode {
                        node_id: e.1,
                        estimate: config.problem.estimate(&e.0),
                        value: self.nodes[e.1 .0].value,
                        state: e.0,
                    });
                }

                // perform the restriction
                self.restrict(var, &mut config, &incumbent, &mut mininodes);
            } else {
                break;
            }
        }
        // we're done, just find the best terminal node
        let best_id = &mut self.best_terminal_node;
        for node in mininodes.iter() {
            if let Some(id) = best_id {
                let best = &self.nodes[id.0];
                if node.value < best.value {
                    *best_id = Some(node.node_id);
                }
            } else {
                *best_id = Some(node.node_id);
            }
        }
    }

    fn restrict<V, N>(&mut self, 
        var: Var,
        config: &mut Config<P, V, N>, 
        incumbent: &Incumbent,
        mininodes: &mut Vec<MiniNode<<P as Problem>::State>>) 
    where
        V: VariableOrdering<State = P::State>,
        N: NodeSelectionHeuristic,
    {
        if mininodes.len() > config.max_width {
            // we are going to truncate the next layer. it is no longer an exact dd
            self.is_exact = false;
            // first, make sure to move all the mandatory nodes at the beginning
            // of the vector
            let mut frontier = 0;
            for i in 0..mininodes.len() {
                let mandatory = config.node_sel.is_mandatory(
                    self,
                    &mininodes[i],
                    var,
                    incumbent.best_sol);

                if mandatory || config.rng.gen_bool(config.proba) {
                    mininodes.swap(i, frontier);
                    frontier += 1;
                }
            }
            // then sort the rest of the vector and keep only the max_width most
            // relevant ones
            let (_keep, sort) = mininodes.split_at_mut(frontier);
            sort.sort_unstable_by(|a, b| config.node_sel.compare(self, a, b));
            let limit = config.max_width.max(frontier);
            mininodes.truncate(limit);
        }
    }

    fn dive_if_needed<V, N>(&mut self, 
        config: &Config<P, V, N>, 
        incumbent: &Incumbent, 
        mininodes: &mut Vec<MiniNode<<P as Problem>::State>>) 
    where
        V: VariableOrdering<State = P::State>,
        N: NodeSelectionHeuristic,
    {
        if let Some(sol) = incumbent.best_sol {
            // cant be exact otherwise
            self.is_exact = config.start_depth == 0;

            for _ in 0..config.start_depth {
                let var      = config.var_ord.next(&mut mininodes.iter().map(|n| n.state())).unwrap();
                let val      = sol[var];
                let decision = Decision { var, val };
                for mininode in mininodes.drain(..) {
                    self.branch_on(
                        config.kill_switch,
                        false,
                        config.problem,
                        &mininode,
                        decision,
                    );
                }
                // collect the single one node into a mininode (keep it uniform
                // with the non dive case
                for e in self.next_layer_states.drain() {
                    mininodes.push(MiniNode {
                        node_id: e.1,
                        estimate: config.problem.estimate(&e.0),
                        value: self.nodes[e.1 .0].value,
                        state: e.0,
                    });
                }    
            }
        }
    }

    fn branch_on(
        &mut self,
        kill_switch: &AtomicBool,
        failible: bool,
        problem: &P,
        from: &MiniNode<P::State>,
        decision: Decision,
    ) {
        if failible && kill_switch.load(Ordering::Relaxed) {
            self.is_exact = false;
            return;
        }
        //
        let state = problem.transition(&from.state, decision);
        let cost = problem.transition_cost(&from.state, decision);

        let total = from.value.saturating_add(cost);

        // do I need to create a new node ?
        match self.next_layer_states.entry(state) {
            // yes, i do
            Entry::Vacant(e) => {
                let new_node_id = NodeId(self.nodes.len());

                let edge = Edge {
                    from: from.node_id,
                    to: new_node_id,
                    label: decision,
                    weight: cost,
                };

                let node = Node {
                    my_id: new_node_id,
                    value: total,
                    best_parent: Some(edge),
                };

                self.nodes.push(node);
                e.insert(new_node_id);
            }
            // No i don't but i still need to add an edge (if it improves the path)
            Entry::Occupied(e) => {
                let reused_node_id = *e.get();
                let reused_node = &mut self.nodes[reused_node_id.0];

                if reused_node.value > total {
                    // we do improve the best path, hence we must adapt the best parent
                    let edge = Edge {
                        from: from.node_id,
                        to: reused_node_id,
                        label: decision,
                        weight: cost,
                    };
                    reused_node.value = total;
                    reused_node.best_parent = Some(edge);
                }
            }
        }
    }
}

impl<S> SelectableNode for MiniNode<S> {
    type State = S;

    fn state(&self) -> &Self::State {
        &self.state
    }

    fn value(&self) -> isize {
        self.value
    }

    fn estimate(&self) -> isize {
        self.estimate
    }
}

impl<P> NodeSource for Diagram<P>
where
    P: Problem,
{
    type State = P::State;
    type Node = MiniNode<P::State>;
    type Path = PathIter<P>;

    fn path(&self, node: &Self::Node) -> Self::Path {
        let me = self as *const Diagram<P>;
        let cur = self.nodes[node.node_id.0].best_parent;
        PathIter {
            diagram: me,
            current: cur,
        }
    }
}

/// This iterator is unsafe as it borrows the diagram without explicitly
/// mentioning its lifetime. This could be fixed if generic associated types
/// became generic
#[derive(Clone, Copy)]
struct PathIter<P>
where
    P: Problem,
    P::State: Eq + PartialEq + Hash,
{
    diagram: *const Diagram<P>,
    current: Option<Edge>,
}
impl<P> Iterator for PathIter<P>
where
    P: Problem,
    P::State: Eq + PartialEq + Hash,
{
    type Item = Decision;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(edge) = self.current {
            unsafe {
                let diagram = &*self.diagram;
                self.current = diagram.nodes[edge.from.0].best_parent;
            }
            Some(edge.label)
        } else {
            None
        }
    }
}
