use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

use crate::config::Config;
use crate::core::{CrabbitError, DocId};
use crate::indexer::compact::maybe_merge;
use crate::indexer::ingest::{ingest, RawDocument};
use crate::indexer::manifest::{manifest_path, Manifest};
use crate::query::pipeline::ScoringPipeline;

pub struct EngineState {
    pub config: Config,
    pub data_dir: PathBuf,
    pub manifest: Mutex<Manifest>,
    pub pipeline: ScoringPipeline,
    query_count: AtomicU64,
    total_latency_micros: AtomicU64,
}

impl EngineState {
    pub fn new(config: Config) -> anyhow::Result<Self> {
        let data_dir = PathBuf::from(&config.index.data_dir);
        let manifest = Manifest::load(&manifest_path(&data_dir))?;

        Ok(Self {
            config,
            data_dir,
            manifest: Mutex::new(manifest),
            pipeline: ScoringPipeline::default(),
            query_count: AtomicU64::new(0),
            total_latency_micros: AtomicU64::new(0),
        })
    }

    /// Shared by `/index` and the crawler: ingest one doc, merge if due, persist.
    pub fn ingest_document(&self, doc: RawDocument, now: i64) -> Result<DocId, CrabbitError> {
        let mut manifest = self.manifest.lock().unwrap();
        let doc_id = ingest(&self.data_dir, &mut manifest, doc, now)?;
        maybe_merge(&self.data_dir, &mut manifest, now)?;
        manifest.save(&manifest_path(&self.data_dir))?;
        Ok(doc_id)
    }

    pub fn record_query(&self, latency_micros: u64) {
        self.query_count.fetch_add(1, Ordering::Relaxed);
        self.total_latency_micros
            .fetch_add(latency_micros, Ordering::Relaxed);
    }

    pub fn query_count(&self) -> u64 {
        self.query_count.load(Ordering::Relaxed)
    }

    pub fn avg_query_latency_micros(&self) -> u64 {
        let count = self.query_count();
        if count == 0 {
            0
        } else {
            self.total_latency_micros.load(Ordering::Relaxed) / count
        }
    }
}
