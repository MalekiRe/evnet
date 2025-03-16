//! Wrapper around a min-heap taken from Lightyear
use std::{cmp::Ordering, collections::BinaryHeap};

/// A buffer that contains items associated with a key (a Tick, Instant, etc.)
///
/// Elements in the buffer are popped only when they are 'ready', i.e.
/// when the key associated with the item is less than or equal to the current key
///
/// The most recent item (by associated key) is returned first
#[derive(Clone, Debug)]
pub struct ReadyBuffer<K, T> {
    // TODO: compare performance with a SequenceBuffer of fixed size
    // TODO: add a maximum size to the buffer. The elements that are farther away from being ready dont' get added?
    /// min heap: we pop the items with smallest key first
    pub heap: BinaryHeap<ItemWithReadyKey<K, T>>,
}

impl<K: Ord, T> Default for ReadyBuffer<K, T> {
    fn default() -> Self {
        Self {
            heap: BinaryHeap::default(),
        }
    }
}

impl<K: Ord, T> ReadyBuffer<K, T> {
    pub fn new() -> Self {
        Self {
            heap: BinaryHeap::default(),
        }
    }
}

impl<K: Ord + Clone, T> ReadyBuffer<K, T> {
    /// Adds an item to the heap marked by time
    pub fn push(&mut self, key: K, item: T) {
        self.heap.push(ItemWithReadyKey { key, item });
    }

    /// Returns a reference to the item with the highest `K` value or None if the queue is empty.
    /// Does not remove the item from the queue.
    pub fn peek_max_item(&self) -> Option<(K, &T)> {
        self.heap.peek().map(|item| (item.key.clone(), &item.item))
    }

    /// Returns whether or not there is an item with a key more recent or equal to `current_key`
    /// that is ready to be returned (i.e. we are beyond the instant associated with the item)
    pub fn has_item(&self, current_key: &K) -> bool {
        // if self.heap.is_empty() {
        //     return false;
        // }
        if let Some(item) = self.heap.peek() {
            // if the current_key is bigger than the item key, we are ready to return
            let cmp = item.key.cmp(current_key);
            return matches!(cmp, Ordering::Less | Ordering::Equal);
        }
        false
    }

    /// Same as `pop_item` but does not remove the item from the queue
    pub fn peek_item(&self, current_key: &K) -> Option<(K, &T)> {
        if self.has_item(current_key) {
            if let Some(container) = self.heap.peek() {
                return Some((container.key.clone(), &container.item));
            }
        }
        None
    }

    /// Pops the top item (with smallest key) from the queue if the key is above the provided `current_key`
    /// (i.e. we are beyond the instant associated with the item)
    pub fn pop_item(&mut self, current_key: &K) -> Option<(K, T)> {
        if self.has_item(current_key) {
            if let Some(container) = self.heap.pop() {
                return Some((container.key, container.item));
            }
        }
        None
    }

    /// Pop all items that are older or equal than the provided key, then return the value for the most recent item
    /// with a key older or equal to the provided key
    /// (i.e. if we have keys 1, 4, 6, pop_until(5) will pop 1, 4 and return the value for key 4)
    /// /// (i.e. if we have keys 1, 4, 6, pop_until(4) will pop 1, 4 and return the value for key 4)
    pub(crate) fn pop_until(&mut self, key: &K) -> Option<(K, T)> {
        if self.heap.is_empty() {
            return None;
        }
        let mut val = None;
        while let Some(item_with_key) = self.heap.peek() {
            // we have a new update that is older than what we want, stop
            if item_with_key.key > *key {
                // put back the update in the heap
                // self.heap.push(item_with_key);
                break;
            }
            // safety: we know that the heap is not empty and that the key is <= the provided key
            val = self.heap.pop().map(|item| (item.key, item.item));
        }
        val
    }

    /// Pop all items that are older or equal than the provided key, then return all the values that were popped
    pub(crate) fn drain_until(&mut self, key: &K) -> Vec<(K, T)> {
        if self.heap.is_empty() {
            return vec![];
        }
        let mut val = Vec::new();
        while let Some(item_with_key) = self.heap.peek() {
            // we have a new update that is older than what we want, stop
            if item_with_key.key > *key {
                // put back the update in the heap
                // self.heap.push(item_with_key);
                break;
            }
            // safety: we know that the heap is not empty and that the key is <= the provided key
            if let Some(v) = self.heap.pop().map(|item| (item.key, item.item)) {
                val.push(v);
            }
        }
        val
    }

    /// Pop all items that are more recent or equal than the provided key, then return all the values that were popped
    pub(crate) fn drain_after(&mut self, key: &K) -> Vec<(K, T)> {
        if self.heap.is_empty() {
            return vec![];
        }
        let mut older = vec![];
        let mut newer = vec![];
        while let Some(item_with_key) = self.heap.pop() {
            // keep older keys in the buffer
            if item_with_key.key < *key {
                older.push(item_with_key);
                continue;
            }
            newer.push((item_with_key.key, item_with_key.item));
            break;
        }
        // all the remaining values in the heap are the ones that are newer than the provided key
        newer.extend(
            self.heap
                .drain()
                .map(|item| (item.key, item.item))
                .collect::<Vec<_>>(),
        );
        // put back the older values
        older.into_iter().for_each(|item| self.heap.push(item));
        newer
    }

    /// Returns the length of the underlying queue
    pub fn len(&self) -> usize {
        self.heap.len()
    }

    /// Checks if the underlying queue is empty
    pub fn is_empty(&self) -> bool {
        self.heap.is_empty()
    }
}

#[derive(Clone, Debug)]
pub struct ItemWithReadyKey<K, T> {
    pub key: K,
    pub item: T,
}

impl<K: Ord, T> Eq for ItemWithReadyKey<K, T> {}

impl<K: Ord, T> PartialEq<Self> for ItemWithReadyKey<K, T> {
    fn eq(&self, other: &Self) -> bool {
        self.key == other.key
    }
}

impl<K: Ord, T> PartialOrd<Self> for ItemWithReadyKey<K, T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// BinaryHeap is a max-heap, so we must reverse the ordering of the Instants
/// to get a min-heap
impl<K: Ord, T> Ord for ItemWithReadyKey<K, T> {
    fn cmp(&self, other: &ItemWithReadyKey<K, T>) -> Ordering {
        other.key.cmp(&self.key)
    }
}