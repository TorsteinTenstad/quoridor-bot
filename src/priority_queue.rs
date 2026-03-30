use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashSet};
use std::hash::Hash;

#[derive(Debug, Clone)]
pub struct PriorityQueue<K, T> {
    heap: BinaryHeap<Reverse<(K, T)>>,
    set: HashSet<T>,
}

impl<K: Ord + Clone, T: Ord + Hash + Clone> Default for PriorityQueue<K, T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<K: Ord + Clone, T: Ord + Hash + Clone> PriorityQueue<K, T> {
    pub fn new() -> Self {
        Self {
            heap: BinaryHeap::new(),
            set: HashSet::new(),
        }
    }

    #[allow(dead_code)]
    pub fn peek(&self) -> Option<(K, T)> {
        let Reverse((k, t)) = self.heap.peek()?;
        Some((k.clone(), t.clone()))
    }

    pub fn pop(&mut self) -> Option<(K, T)> {
        let Reverse((k, t)) = self.heap.pop()?;
        self.set.remove(&t);
        Some((k, t))
    }

    pub fn insert(&mut self, k: K, t: T) -> bool {
        self.heap.push(Reverse((k, t.clone())));
        self.set.insert(t)
    }

    pub fn contains(&self, t: &T) -> bool {
        self.set.contains(t)
    }

    pub fn remove(&mut self, t: &T) {
        self.heap.retain(|Reverse((_k, t_))| t != t_);
        self.set.remove(t);
    }
}
