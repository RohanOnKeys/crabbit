use crate::query::bm25;

/// One query term's match info against a candidate document.
pub struct TermMatch {
    pub idf: f32,
    pub term_freq: u32,
}

pub struct ScoringContext<'a> {
    pub doc_len: u32,
    pub avg_doc_len: f32,
    pub term_matches: &'a [TermMatch],
}

#[derive(Debug, Clone, Copy)]
pub struct ScoredDoc {
    pub doc_id: crate::core::DocId,
    pub score: f32,
}

pub trait ScoringStage: Send + Sync {
    fn score(&self, ctx: &ScoringContext, candidate: &mut ScoredDoc);
}

pub struct Bm25Stage;

impl ScoringStage for Bm25Stage {
    fn score(&self, ctx: &ScoringContext, candidate: &mut ScoredDoc) {
        let sum: f32 = ctx
            .term_matches
            .iter()
            .map(|m| bm25::term_score(m.term_freq, ctx.doc_len, ctx.avg_doc_len, m.idf))
            .sum();
        candidate.score += sum;
    }
}

pub struct ScoringPipeline {
    stages: Vec<Box<dyn ScoringStage>>,
}

impl ScoringPipeline {
    pub fn new(stages: Vec<Box<dyn ScoringStage>>) -> Self {
        Self { stages }
    }

    pub fn run(&self, ctx: &ScoringContext, candidate: &mut ScoredDoc) {
        for stage in &self.stages {
            stage.score(ctx, candidate);
        }
    }
}

impl Default for ScoringPipeline {
    fn default() -> Self {
        Self::new(vec![Box::new(Bm25Stage)])
    }
}
