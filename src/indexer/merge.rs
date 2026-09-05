use std::collections::HashMap;
use std::path::Path;

use crate::core::CrabbitError;
use crate::indexer::segment::{write_segment, IndexedDoc, SegmentReader};

/// Merges several segments into one by reconstructing term multisets from postings.
pub fn merge_segments(
    readers: &[SegmentReader],
    out_dir: &Path,
    created_at: i64,
) -> Result<(), CrabbitError> {
    let mut merged_docs = Vec::new();

    for reader in readers {
        let mut doc_terms: HashMap<u32, Vec<String>> = HashMap::new();
        for term in reader.terms() {
            for posting in reader.postings(term)? {
                doc_terms
                    .entry(posting.local_doc_id)
                    .or_default()
                    .extend(std::iter::repeat_n(term.to_string(), posting.term_freq as usize));
            }
        }

        for local_doc_id in 0..reader.doc_count() {
            let Some(meta) = reader.doc_meta(local_doc_id) else { continue };
            let terms = doc_terms.remove(&local_doc_id).unwrap_or_default();
            merged_docs.push(IndexedDoc { meta: meta.clone(), terms });
        }
    }

    write_segment(out_dir, &merged_docs, created_at)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::DocMeta;

    fn doc(url: &str, terms: &[&str]) -> IndexedDoc {
        let terms: Vec<String> = terms.iter().map(|t| t.to_string()).collect();
        IndexedDoc {
            meta: DocMeta {
                url: url.to_string(),
                title: url.to_string(),
                snippet: String::new(),
                content_hash: 0,
                doc_len: terms.len() as u32,
                indexed_at: 0,
            },
            terms,
        }
    }

    #[test]
    fn merges_two_segments_without_losing_docs() {
        let dir_a = tempfile::tempdir().unwrap();
        write_segment(dir_a.path(), &[doc("a", &["rust", "web"])], 0).unwrap();
        let reader_a = SegmentReader::open(dir_a.path(), 0).unwrap();

        let dir_b = tempfile::tempdir().unwrap();
        write_segment(dir_b.path(), &[doc("b", &["rust", "search"])], 0).unwrap();
        let reader_b = SegmentReader::open(dir_b.path(), 1).unwrap();

        let out_dir = tempfile::tempdir().unwrap();
        merge_segments(&[reader_a, reader_b], out_dir.path(), 0).unwrap();

        let merged = SegmentReader::open(out_dir.path(), 2).unwrap();
        assert_eq!(merged.doc_count(), 2);
        assert_eq!(merged.doc_freq("rust"), 2);
        assert_eq!(merged.doc_freq("web"), 1);
        assert_eq!(merged.doc_freq("search"), 1);

        let urls: std::collections::HashSet<_> =
            (0..merged.doc_count()).map(|id| merged.doc_meta(id).unwrap().url.clone()).collect();
        assert_eq!(urls, ["a".to_string(), "b".to_string()].into_iter().collect());
    }
}
