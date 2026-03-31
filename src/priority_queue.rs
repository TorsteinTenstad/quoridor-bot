use std::cmp::Reverse;
use std::hash::Hash;

#[derive(Debug, Clone)]
pub struct PriorityQueue<K, T> {
    pq: priority_queue::PriorityQueue<T, Reverse<K>>,
}

impl<K: Ord + Clone, T: Ord + Hash + Clone> Default for PriorityQueue<K, T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<K: Ord + Clone, T: Ord + Hash + Clone> PriorityQueue<K, T> {
    pub fn new() -> Self {
        Self {
            pq: priority_queue::PriorityQueue::<T, Reverse<K>>::new(),
        }
    }

    pub fn peek(&self) -> Option<(K, T)> {
        let (t, Reverse(k)) = self.pq.peek()?;
        Some((k.clone(), t.clone()))
    }

    pub fn pop(&mut self) -> Option<(K, T)> {
        let (t, Reverse(k)) = self.pq.pop()?;
        Some((k, t))
    }

    pub fn insert(&mut self, k: K, t: T) -> bool {
        self.pq.push(t, Reverse(k)).is_some()
    }

    pub fn contains(&self, t: &T) -> bool {
        self.pq.contains(t)
    }

    pub fn remove(&mut self, t: &T) {
        self.pq.remove(t);
    }
}
