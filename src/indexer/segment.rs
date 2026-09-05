use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use memmap2::Mmap;
use serde::{Deserialize, Serialize};

use crate::core::{CrabbitError, DocMeta};

pub type Result<T> = std::result::Result<T, CrabbitError>;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TermEntry {
    postings_offset: u64,
    postings_len: u32,
    doc_freq: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SegmentMeta {
    format_version: u32,
    doc_count: u32,
    avg_doc_len: f32,
    created_at: i64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Posting {
    pub local_doc_id: u32,
    pub term_freq: u32,
}

const FORMAT_VERSION: u32 = 1;

/// One document handed to the segment writer: its metadata plus its tokenized terms.
pub struct IndexedDoc {
    pub meta: DocMeta,
    pub terms: Vec<String>,
}

/// Writes an immutable on-disk segment from a batch of already tokenized docs.
pub fn write_segment(dir: &Path, docs: &[IndexedDoc], created_at: i64) -> Result<()> {
    fs::create_dir_all(dir)?;

    let mut postings_by_term: BTreeMap<&str, Vec<Posting>> = BTreeMap::new();
    for (local_doc_id, doc) in docs.iter().enumerate() {
        let mut term_freq: BTreeMap<&str, u32> = BTreeMap::new();
        for term in &doc.terms {
            *term_freq.entry(term.as_str()).or_insert(0) += 1;
        }
        for (term, freq) in term_freq {
            postings_by_term.entry(term).or_default().push(Posting {
                local_doc_id: local_doc_id as u32,
                term_freq: freq,
            });
        }
    }

    let mut postings_bytes = Vec::new();
    let mut terms: BTreeMap<String, TermEntry> = BTreeMap::new();
    for (term, postings) in &postings_by_term {
        let offset = postings_bytes.len() as u64;
        let encoded = bincode::serialize(postings)?;
        postings_bytes.extend_from_slice(&encoded);
        terms.insert(
            term.to_string(),
            TermEntry {
                postings_offset: offset,
                postings_len: encoded.len() as u32,
                doc_freq: postings.len() as u32,
            },
        );
    }
    fs::write(dir.join("postings.bin"), &postings_bytes)?;
    fs::write(dir.join("terms.dict"), bincode::serialize(&terms)?)?;

    let doc_metas: Vec<&DocMeta> = docs.iter().map(|d| &d.meta).collect();
    fs::write(dir.join("docs.store"), bincode::serialize(&doc_metas)?)?;

    let doc_count = docs.len() as u32;
    let avg_doc_len = if doc_count == 0 {
        0.0
    } else {
        docs.iter().map(|d| d.meta.doc_len as f64).sum::<f64>() as f32 / doc_count as f32
    };
    let meta = SegmentMeta {
        format_version: FORMAT_VERSION,
        doc_count,
        avg_doc_len,
        created_at,
    };
    fs::write(dir.join("segment.meta"), bincode::serialize(&meta)?)?;

    Ok(())
}

/// A read-only handle onto one on-disk segment; postings are mmap'd and lazy.
pub struct SegmentReader {
    pub segment_id: u32,
    terms: BTreeMap<String, TermEntry>,
    postings_mmap: Mmap,
    docs: Vec<DocMeta>,
    meta: SegmentMeta,
}

impl SegmentReader {
    pub fn open(dir: &Path, segment_id: u32) -> Result<Self> {
        let terms: BTreeMap<String, TermEntry> =
            bincode::deserialize(&fs::read(dir.join("terms.dict"))?)?;
        let docs: Vec<DocMeta> = bincode::deserialize(&fs::read(dir.join("docs.store"))?)?;
        let meta: SegmentMeta = bincode::deserialize(&fs::read(dir.join("segment.meta"))?)?;

        let postings_file = fs::File::open(dir.join("postings.bin"))?;
        let postings_mmap = unsafe { Mmap::map(&postings_file)? };

        Ok(Self { segment_id, terms, postings_mmap, docs, meta })
    }

    pub fn doc_count(&self) -> u32 {
        self.meta.doc_count
    }

    pub fn avg_doc_len(&self) -> f32 {
        self.meta.avg_doc_len
    }

    pub fn doc_freq(&self, term: &str) -> u32 {
        self.terms.get(term).map(|e| e.doc_freq).unwrap_or(0)
    }

    pub fn postings(&self, term: &str) -> Result<Vec<Posting>> {
        let Some(entry) = self.terms.get(term) else {
            return Ok(Vec::new());
        };
        let start = entry.postings_offset as usize;
        let end = start + entry.postings_len as usize;
        let slice = self
            .postings_mmap
            .get(start..end)
            .ok_or_else(|| CrabbitError::CorruptSegment(format!("term {term} out of bounds")))?;
        Ok(bincode::deserialize(slice)?)
    }

    pub fn doc_meta(&self, local_doc_id: u32) -> Option<&DocMeta> {
        self.docs.get(local_doc_id as usize)
    }

    pub fn terms(&self) -> impl Iterator<Item = &str> {
        self.terms.keys().map(|s| s.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::DocMeta;

    fn sample_doc(url: &str, terms: &[&str]) -> IndexedDoc {
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
    fn writes_and_reads_back_a_segment() {
        let tmp = tempfile::tempdir().unwrap();
        let docs = vec![
            sample_doc("a", &["rust", "web", "framework"]),
            sample_doc("b", &["rust", "search", "engine"]),
        ];
        write_segment(tmp.path(), &docs, 1_000).unwrap();

        let reader = SegmentReader::open(tmp.path(), 0).unwrap();
        assert_eq!(reader.doc_count(), 2);
        assert_eq!(reader.doc_freq("rust"), 2);
        assert_eq!(reader.doc_freq("web"), 1);
        assert_eq!(reader.doc_freq("missing"), 0);

        let postings = reader.postings("rust").unwrap();
        assert_eq!(postings.len(), 2);
        assert_eq!(postings[0].local_doc_id, 0);
        assert_eq!(postings[1].local_doc_id, 1);

        assert_eq!(reader.doc_meta(0).unwrap().url, "a");
        assert_eq!(reader.doc_meta(1).unwrap().url, "b");
    }

    #[test]
    fn term_frequency_is_counted_per_doc() {
        let tmp = tempfile::tempdir().unwrap();
        let docs = vec![sample_doc("a", &["rust", "rust", "rust", "web"])];
        write_segment(tmp.path(), &docs, 0).unwrap();

        let reader = SegmentReader::open(tmp.path(), 0).unwrap();
        let postings = reader.postings("rust").unwrap();
        assert_eq!(postings[0].term_freq, 3);
    }
}
