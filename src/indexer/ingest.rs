use std::path::Path;

use crate::core::{CrabbitError, DocId, DocMeta};
use crate::indexer::manifest::{segment_dir, Manifest};
use crate::indexer::segment::{write_segment, IndexedDoc};
use crate::indexer::stemmer::EnglishStemmer;
use crate::indexer::tokenizer::tokenize;

pub struct RawDocument {
    pub url: String,
    pub title: String,
    pub body: String,
}

/// Tokenizes, stems, and writes one document as a new single-doc segment.
pub fn ingest(
    data_dir: &Path,
    manifest: &mut Manifest,
    doc: RawDocument,
    indexed_at: i64,
) -> Result<DocId, CrabbitError> {
    let stemmer = EnglishStemmer::new();
    let mut terms: Vec<String> = tokenize(&doc.title);
    terms.extend(tokenize(&doc.body));
    let terms: Vec<String> = terms.iter().map(|t| stemmer.stem(t)).collect();

    let content_hash = blake3::hash(doc.body.as_bytes());
    let content_hash = u64::from_le_bytes(content_hash.as_bytes()[..8].try_into().unwrap());

    let meta = DocMeta {
        url: doc.url,
        title: doc.title,
        snippet: doc.body.chars().take(160).collect(),
        content_hash,
        doc_len: terms.len() as u32,
        indexed_at,
    };

    let segment_id = manifest.next_segment_id;
    manifest.next_segment_id += 1;

    write_segment(&segment_dir(data_dir, segment_id), &[IndexedDoc { meta, terms }], indexed_at)?;
    manifest.segments.push(segment_id);

    Ok(DocId::new(segment_id, 0))
}
