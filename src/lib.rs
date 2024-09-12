use std::{
    marker::PhantomData,
    ops::{Index, IndexMut},
    sync::OnceLock,
    thread::{self, JoinHandle},
};

pub mod rng;
use rng::{Rng, RngProvider};

/// statically declared sqrt(2) default exploration constant
fn default_exploration_constant() -> f64 {
    static DEFAULT_EXPLORATION_CONSTANT: OnceLock<f64> = OnceLock::new();

    *DEFAULT_EXPLORATION_CONSTANT.get_or_init(|| 2.0_f64.sqrt())
}

pub trait GameState: Clone {
    type Move: Clone + Copy + Eq;
    type UserData: Eq;

    /// Returns all moves that can be performed from this state.
    fn all_moves(&self) -> Vec<Self::Move>;

    /// A default implementation for a random move from this state, used in random playout.
    fn random_move<R: Rng>(&self, rng: &mut R) -> Option<Self::Move> {
        let children = self.all_moves();
        if children.is_empty() {
            None
        } else {
            let idx = rng.gen_range(0..children.len());
            Some(children[idx])
        }
    }

    /// Modify this state by applying this move.
    fn apply_move(&self, action: Self::Move) -> Self;

    /// Determine if this is a terminal state. If so then return metadata about the state.
    fn is_terminal_state(&self) -> Option<Self::UserData>;

    /// Given metadata from a terminal state, is this beneficial for this state?
    fn terminal_is_win(&self, condition: &Self::UserData) -> bool;
}

pub struct Node<T>
where
    T: GameState,
{
    n: u32,
    w: u32,
    pub state: T,
    children: Vec<usize>,
    parent: Option<usize>,
}

impl<T> Node<T>
where
    T: GameState,
{
    pub fn new(t: T, parent: Option<usize>) -> Self {
        Self {
            n: 1,
            w: 0,
            state: t,
            children: Vec::new(),
            parent,
        }
    }
}

pub struct Tree<T: GameState> {
    nodes: Vec<Node<T>>,
    exploration_factor: f64,
}

impl<T: GameState> Tree<T> {
    pub fn new(exploration_factor: f64) -> Self {
        Self {
            nodes: Vec::new(),
            exploration_factor,
        }
    }

    pub fn add_node_with_parent(&mut self, n: Node<T>) -> usize {
        let parent = n.parent;
        let len = self.nodes.len();
        self.nodes.push(n);
        if let Some(parent) = parent {
            self.nodes.get_mut(parent).unwrap().children.push(len);
        }
        len
    }

    /// upper confidence bound calculation
    fn uct(&self, node_idx: usize, parent_idx: usize) -> f64 {
        let node = &self.nodes[node_idx];
        let parent = &self.nodes[parent_idx];

        let win_prob = node.w as f64 / node.n as f64;
        let exploration = self.exploration_factor * ((parent.n as f64).ln() / node.n as f64).sqrt();

        win_prob + exploration
    }

    /// Traverse children and find node with bets UCT.
    pub fn select(&self) -> usize {
        let mut nidx = 0;
        loop {
            let p = &self[nidx];
            if p.state.is_terminal_state().is_some() {
                return nidx;
            }
            if p.children.is_empty() {
                break;
            } else {
                let best_uct_opt = p
                    .children
                    .iter()
                    .map(|&c| (self.uct(c, nidx), c))
                    .max_by(|v1, v2| v1.0.total_cmp(&v2.0));
                if let Some(best_uct) = best_uct_opt {
                    nidx = best_uct.1;
                } else {
                    unreachable!()
                }
            }
        }

        nidx
    }

    /// Creates all children for a given node index and returns their indexes.
    pub fn expand(&mut self, idx: usize) -> Vec<usize> {
        let state = self[idx].state.clone();

        state
            .all_moves()
            .into_iter()
            .map(|m| state.apply_move(m))
            .map(|s| Node::new(s, Some(idx)))
            .map(|n| self.add_node_with_parent(n))
            .collect()
    }

    pub fn random_playout<R: Rng>(&self, n: usize, rng: &mut R) -> <T as GameState>::UserData {
        let mut state = self[n].state.clone();
        loop {
            let reward = state.is_terminal_state();
            if let Some(r) = reward {
                return r;
            } else {
                let m = state.random_move(rng).unwrap();
                state = state.apply_move(m);
            }
        }
    }

    pub fn backpropagate(&mut self, idx: usize, result: <T as GameState>::UserData) {
        let mut node = &mut self[idx];
        loop {
            node.n += 1;
            if node.state.terminal_is_win(&result) {
                node.w += 1;
            }
            match node.parent {
                Some(parent) => node = &mut self[parent],
                None => break,
            }
        }
    }
}

impl<T: GameState> Index<usize> for Tree<T> {
    type Output = Node<T>;

    fn index(&self, index: usize) -> &Self::Output {
        &self.nodes[index]
    }
}

impl<T: GameState> IndexMut<usize> for Tree<T> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.nodes[index]
    }
}

pub struct BestResultHandle<T: GameState> {
    threads: Vec<JoinHandle<(u32, Vec<u32>)>>,
    initial_move_set: Vec<T::Move>,
}

pub struct BestResult<T: GameState> {
    pub iterations: u32,
    pub best_move: <T as GameState>::Move,
}

impl<T: GameState> BestResultHandle<T> {
    pub fn is_finished(&mut self) -> bool {
        !self.threads.iter().any(|thread| !thread.is_finished())
    }

    pub fn join(self) -> BestResult<T> {
        let results = self
            .threads
            .into_iter()
            .map(|t| t.join().unwrap())
            .reduce(|acc, val| {
                let iters = acc.0 + val.0;
                let vals = acc.1.into_iter().zip(val.1).map(|(a, b)| a + b).collect();
                (iters, vals)
            })
            .unwrap();

        let iterations = results.0;

        let best_move_idx = results
            .1
            .into_iter()
            .enumerate()
            .max_by_key(|t| t.1)
            .unwrap()
            .0;

        let best_move = self.initial_move_set[best_move_idx];

        BestResult {
            iterations,
            best_move,
        }
    }
}

pub struct MCTS<R>
where
    R: RngProvider,
{
    num_threads: usize,
    exploration_factor: f64,
    rng_type: PhantomData<R>,
}

pub fn run_with_end_condition<T, R>(
    exploration_factor: f64,
    state: T,
    end_condition: impl Fn(usize, u32) -> bool + Send + Copy + 'static,
    nthreads: usize,
) -> BestResultHandle<T>
where
    T: GameState + Send + Sync + 'static,
    R: RngProvider,
{
    let initial_move_set = state.all_moves();

    let threads = (0..nthreads)
        .map(|_| {
            let state = state.clone();
            let mut rng = R::init();
            thread::spawn(move || {
                let mut iterations = 0;
                let mut tree = Tree::new(exploration_factor);
                let n = Node::new(state, None);
                tree.add_node_with_parent(n);

                loop {
                    let selection_idx = tree.select();
                    let terminal = tree[selection_idx].state.is_terminal_state();

                    // if terminal state, backprogagate it otherwise expand
                    if let Some(reward) = terminal {
                        tree.backpropagate(selection_idx, reward);
                    } else {
                        let new_children = tree.expand(selection_idx);

                        let random_child_idx = rng.gen_range(0..new_children.len());
                        let child_selection = new_children[random_child_idx];

                        let result = tree.random_playout(child_selection, &mut rng);

                        tree.backpropagate(child_selection, result);
                    }

                    if end_condition(nthreads, iterations) {
                        break;
                    }

                    iterations += 1;
                }
                (
                    iterations,
                    tree[0]
                        .children
                        .iter()
                        .map(|&idx| tree[idx].n)
                        .collect::<Vec<u32>>(),
                )
            })
        })
        .collect::<Vec<_>>();

    BestResultHandle {
        threads,
        initial_move_set,
    }
}

impl<R> MCTS<R>
where
    R: RngProvider,
{
    pub fn num_threads(mut self, num_threads: usize) -> Self {
        self.num_threads = num_threads;
        self
    }

    pub fn exploration_factor(mut self, exploration_factor: f64) -> Self {
        self.exploration_factor = exploration_factor;
        self
    }

    #[cfg(feature = "chrono")]
    pub fn run_with_duration<T>(&self, state: T, duration: chrono::TimeDelta) -> BestResultHandle<T>
    where
        T: GameState + Send + Sync + 'static,
    {
        let end_time = chrono::Utc::now() + duration;

        run_with_end_condition::<T, R>(
            self.exploration_factor,
            state,
            move |_, _| chrono::Utc::now() >= end_time,
            self.num_threads,
        )
    }

    pub fn run_with_iterations<T>(&self, state: T, num_iterations: u32) -> BestResultHandle<T>
    where
        T: GameState + Send + Sync + 'static,
    {
        run_with_end_condition::<T, R>(
            self.exploration_factor,
            state,
            move |nthreads, iters| iters >= num_iterations / nthreads as u32,
            self.num_threads,
        )
    }
}

impl<R: RngProvider> Default for MCTS<R> {
    fn default() -> Self {
        #[cfg(feature = "multi-threaded")]
        let num_threads = num_cpus::get();
        #[cfg(not(feature = "multi-threaded"))]
        let num_threads = 1;

        let exploration_factor = default_exploration_constant();

        Self {
            num_threads,
            exploration_factor,
            rng_type: PhantomData,
        }
    }
}
