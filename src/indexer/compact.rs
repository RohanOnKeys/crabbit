use std::path::Path;

use crate::core::CrabbitError;
use crate::indexer::manifest::{segment_dir, Manifest};
use crate::indexer::merge::merge_segments;
use crate::indexer::segment::SegmentReader;

/// Simple synchronous merge threshold for pass 1, not a background compaction policy.
const MERGE_THRESHOLD: usize = 4;

pub fn maybe_merge(data_dir: &Path, manifest: &mut Manifest, now: i64) -> Result<(), CrabbitError> {
    if manifest.segments.len() < MERGE_THRESHOLD {
        return Ok(());
    }

    let readers: Vec<SegmentReader> = manifest
        .segments
        .iter()
        .map(|id| SegmentReader::open(&segment_dir(data_dir, *id), *id))
        .collect::<Result<_, _>>()?;

    let new_segment_id = manifest.next_segment_id;
    manifest.next_segment_id += 1;
    let new_dir = segment_dir(data_dir, new_segment_id);
    merge_segments(&readers, &new_dir, now)?;

    let old_segments = std::mem::replace(&mut manifest.segments, vec![new_segment_id]);
    drop(readers);
    for id in old_segments {
        let _ = std::fs::remove_dir_all(segment_dir(data_dir, id));
    }

    Ok(())
}
