pub const K1: f32 = 1.2;
pub const B: f32 = 0.75;

/// Inverse document frequency for a term with `doc_freq` matches out of `doc_count` docs.
pub fn idf(doc_freq: u32, doc_count: u32) -> f32 {
    let n = doc_count as f32;
    let df = doc_freq as f32;
    (1.0 + (n - df + 0.5) / (df + 0.5)).ln()
}

/// BM25 contribution of a single term match for one document.
pub fn term_score(tf: u32, doc_len: u32, avg_doc_len: f32, idf: f32) -> f32 {
    let tf = tf as f32;
    let norm = 1.0 - B + B * (doc_len as f32 / avg_doc_len);
    idf * (tf * (K1 + 1.0)) / (tf + K1 * norm)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn idf_decreases_as_doc_frequency_rises() {
        let rare = idf(1, 100);
        let common = idf(50, 100);
        assert!(rare > common);
    }

    #[test]
    fn term_appearing_in_every_doc_scores_low() {
        let low_idf = idf(100, 100);
        assert!(low_idf < 0.1);
    }

    #[test]
    fn score_is_monotonic_in_term_frequency() {
        let idf = idf(5, 100);
        let low_tf = term_score(1, 10, 10.0, idf);
        let high_tf = term_score(5, 10, 10.0, idf);
        assert!(high_tf > low_tf);
    }

    #[test]
    fn matches_hand_computed_fixture() {
        // N=2, df=1, tf=3, doc_len=4, avg_doc_len=4
        let idf = idf(1, 2);
        assert!((idf - std::f32::consts::LN_2).abs() < 1e-4);

        let score = term_score(3, 4, 4.0, idf);
        assert!((score - 1.089_231).abs() < 1e-4);
    }

    #[test]
    fn longer_docs_score_lower_for_same_term_frequency() {
        let idf = idf(5, 100);
        let short_doc = term_score(2, 5, 10.0, idf);
        let long_doc = term_score(2, 40, 10.0, idf);
        assert!(short_doc > long_doc);
    }
}
