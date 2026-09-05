use std::collections::HashMap;

use crate::core::{DocId, DocMeta};
use crate::indexer::segment::SegmentReader;
use crate::pagination::Cursor;
use crate::query::bm25;
use crate::query::parser::parse_query;
use crate::query::pipeline::{ScoredDoc, ScoringContext, ScoringPipeline, TermMatch};

pub struct SearchResult {
    pub doc_id: DocId,
    pub score: f32,
    pub meta: DocMeta,
}

pub struct SearchPage {
    pub results: Vec<SearchResult>,
    pub next_cursor: Option<Cursor>,
    pub has_more: bool,
}

/// Scores and ranks a query across every live segment, paginated from `cursor`.
pub fn search(
    segments: &[SegmentReader],
    query: &str,
    limit: u32,
    cursor: Option<Cursor>,
    pipeline: &ScoringPipeline,
    now: i64,
) -> SearchPage {
    let terms = parse_query(query);
    let mut candidates: Vec<ScoredDoc> = Vec::new();
    let mut metas: HashMap<u64, DocMeta> = HashMap::new();

    for segment in segments {
        let doc_count = segment.doc_count();
        let avg_doc_len = segment.avg_doc_len();

        let mut per_doc: HashMap<u32, Vec<TermMatch>> = HashMap::new();
        for term in &terms {
            let doc_freq = segment.doc_freq(term);
            if doc_freq == 0 {
                continue;
            }
            let idf = bm25::idf(doc_freq, doc_count);
            let Ok(postings) = segment.postings(term) else { continue };
            for posting in postings {
                per_doc
                    .entry(posting.local_doc_id)
                    .or_default()
                    .push(TermMatch { idf, term_freq: posting.term_freq });
            }
        }

        for (local_doc_id, matches) in per_doc {
            let Some(meta) = segment.doc_meta(local_doc_id) else { continue };
            let ctx = ScoringContext {
                doc_len: meta.doc_len,
                avg_doc_len,
                term_matches: &matches,
            };
            let mut candidate = ScoredDoc {
                doc_id: DocId::new(segment.segment_id, local_doc_id),
                score: 0.0,
            };
            pipeline.run(&ctx, &mut candidate);
            metas.insert(candidate.doc_id.to_u64(), meta.clone());
            candidates.push(candidate);
        }
    }

    if let Some(cursor) = &cursor {
        candidates.retain(|c| cursor.is_after(c.score, c.doc_id.to_u64()));
    }
    candidates.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap()
            .then_with(|| a.doc_id.cmp(&b.doc_id))
    });

    let has_more = candidates.len() as u32 > limit;
    candidates.truncate(limit as usize);

    let next_cursor = candidates
        .last()
        .map(|c| Cursor::new(c.score, c.doc_id.to_u64(), now));

    let results = candidates
        .into_iter()
        .map(|c| {
            let meta = metas.remove(&c.doc_id.to_u64()).expect("meta collected above");
            SearchResult { doc_id: c.doc_id, score: c.score, meta }
        })
        .collect();

    SearchPage {
        results,
        next_cursor: if has_more { next_cursor } else { None },
        has_more,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::indexer::segment::{write_segment, IndexedDoc};

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
    fn ranks_higher_term_frequency_first() {
        let tmp = tempfile::tempdir().unwrap();
        let docs = vec![
            doc("low", &["rust", "web"]),
            doc("high", &["rust", "rust", "rust", "web"]),
        ];
        write_segment(tmp.path(), &docs, 0).unwrap();
        let reader = SegmentReader::open(tmp.path(), 0).unwrap();

        let pipeline = ScoringPipeline::default();
        let page = search(&[reader], "rust", 10, None, &pipeline, 0);

        assert_eq!(page.results.len(), 2);
        assert_eq!(page.results[0].meta.url, "high");
        assert_eq!(page.results[1].meta.url, "low");
        assert!(!page.has_more);
    }

    #[test]
    fn paginates_with_cursor_no_duplicates() {
        let tmp = tempfile::tempdir().unwrap();
        let docs = vec![doc("a", &["rust"]), doc("b", &["rust"]), doc("c", &["rust"])];
        write_segment(tmp.path(), &docs, 0).unwrap();
        let reader = SegmentReader::open(tmp.path(), 0).unwrap();
        let pipeline = ScoringPipeline::default();

        let page1 = search(&[reader], "rust", 1, None, &pipeline, 0);
        assert_eq!(page1.results.len(), 1);
        assert!(page1.has_more);
        let cursor = page1.next_cursor.unwrap();

        let tmp2 = tempfile::tempdir().unwrap();
        let docs2 = vec![doc("a", &["rust"]), doc("b", &["rust"]), doc("c", &["rust"])];
        write_segment(tmp2.path(), &docs2, 0).unwrap();
        let reader2 = SegmentReader::open(tmp2.path(), 0).unwrap();
        let page2 = search(&[reader2], "rust", 10, Some(cursor), &pipeline, 0);

        let seen: std::collections::HashSet<_> =
            page1.results.iter().chain(page2.results.iter()).map(|r| r.doc_id).collect();
        assert_eq!(seen.len(), 3);
        assert!(!page2.has_more);
    }
}
