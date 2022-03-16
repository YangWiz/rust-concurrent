//! Thread-safe key/value cache.

use std::collections::hash_map::{Entry, HashMap, DefaultHasher};
use std::hash::{Hash, Hasher};
use std::sync::{Arc, Mutex, RwLock};
use std::rc::Rc;

const BUCKET_NUM: u64 = 16;
/// Cache that remembers the result for each key.
#[derive(Debug)]
pub struct Cache<K, V> {
    // todo! This is an example cache type. Build your own cache type that satisfies the
    // specification for `get_or_insert_with`.
    // lock for specific key.
    inner: Arc<RwLock<HashMap<K, V>>>,
    locks: Arc<Vec<Rc<Mutex<()>>>>,
}

impl<K, V> Default for Cache<K, V> {
    fn default() -> Self {
        Cache {
            inner: Arc::new(RwLock::new(HashMap::new())),
            locks: Arc::new((0..BUCKET_NUM).map(|_| { Rc::new(Mutex::new(()))}).collect()),
        }
    }
}

fn calculate_hash<T: Hash>(t: &T) -> u64 {
    let mut s = DefaultHasher::new();
    t.hash(&mut s);
    s.finish()
}

impl<K: Eq + Hash + Clone, V: Clone> Cache<K, V> {
    /// Retrieve the value or insert a new one created by `f`.
    /// An invocation to this function should not block another invocation with a different key.
    /// For example, if a thread calls `get_or_insert_with(key1, f1)` and another thread calls
    /// `get_or_insert_with(key2, f2)` (`key1≠key2`, `key1,key2∉cache`) concurrently, `f1` and `f2`
    /// should run concurrently.
    ///
    /// On the other hand, since `f` may consume a lot of resource (= money), it's desirable not to
    /// duplicate the work. That is, `f` should be run only once for each key. Specifically, even
    /// for the concurrent invocations of `get_or_insert_with(key, f)`, `f` is called only once.
    pub fn get_or_insert_with<F: FnOnce(K) -> V>(&self, key: K, f: F) -> V {
        // interleaving of reader lock and writer lock => deadlock.
        // read() => lock r, increment a state, lock g | write() => wait g unlock => deadlock.
        let cache = self.inner.clone();
        let locks = self.locks.clone();
        let mut key_lock;

        {
            let map = cache.read().unwrap();

            match map.get(&key) {
                Some(val) => {
                    return val.clone();
                },
                None => {
                    let index = (calculate_hash(&key) % BUCKET_NUM) as usize;
                    key_lock = locks[index].clone();
                },
            }; 
            // reader lock drops here.
        }
        match key_lock.try_lock() {
            Ok(_) => {

            },
            Err(_) => {
                // todo: CondVar.
                return cache.read().unwrap().get(&key).unwrap().clone();
            }
        }
        
        // bad!
        let val = f(key.clone());
        let mut map = self.inner.write().unwrap();
        map.insert(key, val.clone());
        val
    }
}
