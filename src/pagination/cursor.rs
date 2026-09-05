use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum CursorError {
    #[error("cursor is not valid base64")]
    InvalidBase64,
    #[error("cursor payload is malformed")]
    InvalidPayload,
    #[error("cursor has expired")]
    Expired,
}

/// Opaque keyset pagination cursor: last-seen (score, doc_id), plus issue time for TTL.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Cursor {
    pub score: f32,
    pub doc_id: u64,
    pub issued_at: i64,
}

impl Cursor {
    pub fn new(score: f32, doc_id: u64, issued_at: i64) -> Self {
        Self { score, doc_id, issued_at }
    }

    pub fn encode(&self) -> String {
        let json = serde_json::to_vec(self).expect("Cursor serializes");
        URL_SAFE_NO_PAD.encode(json)
    }

    pub fn decode(s: &str, now: i64, ttl_seconds: i64) -> Result<Self, CursorError> {
        let bytes = URL_SAFE_NO_PAD
            .decode(s)
            .map_err(|_| CursorError::InvalidBase64)?;
        let cursor: Cursor =
            serde_json::from_slice(&bytes).map_err(|_| CursorError::InvalidPayload)?;
        if now - cursor.issued_at > ttl_seconds {
            return Err(CursorError::Expired);
        }
        Ok(cursor)
    }

    /// True if (score, doc_id) sorts strictly after this cursor under (score DESC, doc_id ASC).
    pub fn is_after(&self, score: f32, doc_id: u64) -> bool {
        score < self.score || (score == self.score && doc_id > self.doc_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip() {
        let c = Cursor::new(0.82, 4421, 1_000);
        let encoded = c.encode();
        let decoded = Cursor::decode(&encoded, 1_500, 3600).unwrap();
        assert_eq!(c, decoded);
    }

    #[test]
    fn expired_cursor_rejected() {
        let c = Cursor::new(0.5, 1, 1_000);
        let encoded = c.encode();
        let err = Cursor::decode(&encoded, 1_000 + 3601, 3600).unwrap_err();
        assert_eq!(err, CursorError::Expired);
    }

    #[test]
    fn not_yet_expired_accepted() {
        let c = Cursor::new(0.5, 1, 1_000);
        let encoded = c.encode();
        assert!(Cursor::decode(&encoded, 1_000 + 3600, 3600).is_ok());
    }

    #[test]
    fn tampered_cursor_rejected() {
        let err = Cursor::decode("not-valid-base64!!", 0, 3600).unwrap_err();
        assert_eq!(err, CursorError::InvalidBase64);
    }

    #[test]
    fn truncated_payload_rejected() {
        let encoded = URL_SAFE_NO_PAD.encode(b"{\"score\":");
        let err = Cursor::decode(&encoded, 0, 3600).unwrap_err();
        assert_eq!(err, CursorError::InvalidPayload);
    }

    #[test]
    fn is_after_orders_by_score_desc_then_doc_id_asc() {
        let c = Cursor::new(0.5, 10, 0);
        assert!(c.is_after(0.4, 999));
        assert!(!c.is_after(0.6, 0));
        assert!(c.is_after(0.5, 11));
        assert!(!c.is_after(0.5, 9));
        assert!(!c.is_after(0.5, 10));
    }
}
