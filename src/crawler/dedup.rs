use std::collections::HashSet;
use std::sync::Mutex;

/// In-run duplicate detection by content hash; cross-run dedup is a later pass.
pub struct Dedup {
    seen: Mutex<HashSet<u64>>,
}

impl Dedup {
    pub fn new() -> Self {
        Self { seen: Mutex::new(HashSet::new()) }
    }

    pub fn hash(body: &str) -> u64 {
        let digest = blake3::hash(body.as_bytes());
        u64::from_le_bytes(digest.as_bytes()[..8].try_into().unwrap())
    }

    /// Returns true if this is the first time we've seen this hash.
    pub fn insert(&self, hash: u64) -> bool {
        self.seen.lock().unwrap().insert(hash)
    }
}

impl Default for Dedup {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_body_hashes_identically() {
        assert_eq!(Dedup::hash("hello"), Dedup::hash("hello"));
    }

    #[test]
    fn different_body_hashes_differently() {
        assert_ne!(Dedup::hash("hello"), Dedup::hash("world"));
    }

    #[test]
    fn insert_only_true_on_first_sight() {
        let dedup = Dedup::new();
        let h = Dedup::hash("hello");
        assert!(dedup.insert(h));
        assert!(!dedup.insert(h));
    }
}
