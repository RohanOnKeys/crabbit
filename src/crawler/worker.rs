use std::sync::Arc;

use futures::stream::{self, StreamExt};

use crate::crawler::dedup::Dedup;
use crate::crawler::fetch::{build_client, fetch_one};
use crate::crawler::robots::is_allowed;
use crate::indexer::ingest::RawDocument;
use crate::state::EngineState;

pub async fn run(state: Arc<EngineState>) -> anyhow::Result<()> {
    let crawler_cfg = state.config.crawler.clone();
    if crawler_cfg.seed_urls.is_empty() {
        tracing::info!("no seed_urls configured, crawler idle");
        return Ok(());
    }

    let client = build_client(&crawler_cfg.user_agent)?;
    let dedup = Arc::new(Dedup::new());

    stream::iter(crawler_cfg.seed_urls.clone())
        .for_each_concurrent(crawler_cfg.concurrency, |url| {
            let client = client.clone();
            let dedup = dedup.clone();
            let state = state.clone();
            async move { crawl_one(&state, &client, &dedup, url).await }
        })
        .await;

    Ok(())
}

async fn crawl_one(state: &Arc<EngineState>, client: &reqwest::Client, dedup: &Dedup, url: String) {
    if !is_allowed(&url) {
        return;
    }

    let page = match fetch_one(client, &url).await {
        Ok(page) => page,
        Err(e) => {
            tracing::warn!(%url, error = %e, "failed to fetch");
            return;
        }
    };

    if !dedup.insert(Dedup::hash(&page.body)) {
        tracing::info!(%url, "skipping duplicate content");
        return;
    }

    let doc = RawDocument { url: page.url.clone(), title: page.title, body: page.body };
    match state.ingest_document(doc, crate::api::unix_now()) {
        Ok(_) => tracing::info!(%url, "crawled and indexed"),
        Err(e) => tracing::warn!(%url, error = %e, "failed to index crawled page"),
    }
}
