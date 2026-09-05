use serde::{Deserialize, Serialize};

/// Global document id: a segment id plus a doc id local to that segment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct DocId {
    pub segment_id: u32,
    pub local_doc_id: u32,
}

impl DocId {
    pub fn new(segment_id: u32, local_doc_id: u32) -> Self {
        Self { segment_id, local_doc_id }
    }

    pub fn to_u64(self) -> u64 {
        (self.segment_id as u64) << 32 | self.local_doc_id as u64
    }

    pub fn from_u64(v: u64) -> Self {
        Self {
            segment_id: (v >> 32) as u32,
            local_doc_id: v as u32,
        }
    }
}

/// A document as submitted for indexing, before tokenization.
#[derive(Debug, Clone)]
pub struct Document {
    pub url: String,
    pub title: String,
    pub body: String,
}

/// Metadata stored per-doc alongside the index, enough to render a search result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocMeta {
    pub url: String,
    pub title: String,
    pub snippet: String,
    pub content_hash: u64,
    pub doc_len: u32,
    pub indexed_at: i64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn doc_id_round_trips_through_u64() {
        let id = DocId::new(7, 42);
        assert_eq!(DocId::from_u64(id.to_u64()), id);
    }

    #[test]
    fn doc_id_orders_by_segment_then_local_id() {
        assert!(DocId::new(1, 0) < DocId::new(2, 0));
        assert!(DocId::new(1, 0) < DocId::new(1, 1));
    }
}
