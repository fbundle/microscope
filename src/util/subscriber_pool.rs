use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

pub struct Pool<T> {
    map: Mutex<HashMap<u64, T>>,
    last_key: AtomicU64,
}

impl<T> Pool<T> {
    pub fn new() -> Self {
        Pool {
            map: Mutex::new(HashMap::new()),
            last_key: AtomicU64::new(0),
        }
    }

    pub fn subscribe(&self, handler: T) -> u64 {
        let key = self.last_key.fetch_add(1, Ordering::Relaxed) + 1;
        self.map.lock().unwrap().insert(key, handler);
        key
    }

    pub fn unsubscribe(&self, key: u64) {
        self.map.lock().unwrap().remove(&key);
    }

    /// Call f with each handler. Holds the lock while iterating.
    pub fn for_each<F>(&self, mut f: F)
    where
        F: FnMut(&T),
    {
        let map = self.map.lock().unwrap();
        for v in map.values() {
            f(v);
        }
    }
}

impl<T> Default for Pool<T> {
    fn default() -> Self {
        Self::new()
    }
}
