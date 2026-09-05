# Crabbit

A fast, memory safe web search engine written in Rust, built around cursor based pagination so results stay stable and quick even deep into a result set.

## Overview

Crabbit is a search engine core made up of four pieces: a crawler, an indexer, a ranking and query layer, and a thin API on top. The name is a nod to Rust's crab mascot and the speed of a rabbit.

## Features

- Async crawling and indexing pipeline built on tokio
- Inverted index with BM25 style relevance scoring
- Cursor based (keyset) pagination instead of fragile offset paging
- REST API with typed request and response models
- Pluggable ranking pipeline (swap or chain scoring stages)
- Sharded index support for horizontal scaling
- Structured logging and metrics out of the box

## Why cursor based pagination

Classic `OFFSET` and `LIMIT` pagination gets slow and inconsistent as the offset grows, and it breaks when new documents are inserted between page loads (items shift, duplicates appear, or results get skipped).

Crabbit instead issues an opaque cursor with every page of results. The cursor encodes the last seen document's sort key (score, then document id as a tiebreaker), base64 encoded so it is safe to pass around in a URL. The next request just says "give me results after this point," so:

- Performance stays flat regardless of how deep you page
- Result order stays stable even if the index changes mid session
- No duplicate or skipped results across pages

### Example

```
GET /search?q=rust+web+framework&limit=20
```

Response:

```json
{
  "results": [ ... ],
  "next_cursor": "eyJzY29yZSI6MC44MiwiZG9jX2lkIjo0NDIxfQ",
  "has_more": true
}
```

Fetching the next page:

```
GET /search?q=rust+web+framework&limit=20&cursor=eyJzY29yZSI6MC44MiwiZG9jX2lkIjo0NDIxfQ
```

## Architecture

```
        ┌───────────┐     ┌───────────┐     ┌───────────┐     ┌───────────┐
        │  Crawler  │ --> │  Indexer  │ --> │  Query    │ --> │  API      │
        │           │     │           │     │  Engine   │     │  Layer    │
        └───────────┘     └───────────┘     └───────────┘     └───────────┘
```

- **Crawler**: fetches and normalizes pages, respects robots.txt, deduplicates via content hashing
- **Indexer**: builds and merges inverted index segments, handles tokenization and stemming
- **Query Engine**: parses queries, scores documents, applies the pagination cursor logic
- **API Layer**: exposes REST endpoints, handles auth and rate limiting

## Getting started

### Prerequisites

- Rust 1.75 or later
- Cargo

### Build

```bash
git clone https://github.com/yourname/crabbit.git
cd crabbit
cargo build --release
```

### Run

```bash
cargo run --release -- --config config.toml
```

## Configuration

```toml
[server]
host = "0.0.0.0"
port = 8080

[index]
shard_count = 4
data_dir = "./data/index"

[pagination]
default_page_size = 20
max_page_size = 100
cursor_ttl_seconds = 3600

[crawler]
concurrency = 50
user_agent = "CrabbitBot/1.0"
```

## Project structure

```
crabbit/
├── src/
│   ├── crawler/
│   ├── indexer/
│   ├── query/
│   ├── pagination/
│   ├── api/
│   └── main.rs
├── tests/
├── benches/
├── config.toml
└── Cargo.toml
```

## API reference

| Endpoint | Method | Description |
|---|---|---|
| `/search` | GET | Run a query, returns paginated results with a cursor |
| `/index` | POST | Submit a document for indexing |
| `/health` | GET | Liveness and readiness check |
| `/stats` | GET | Index size, shard status, query latency metrics |

## Testing and benchmarks

```bash
cargo test
cargo bench
```

## Contributing

Pull requests are welcome. Please open an issue first for larger changes so the approach can be discussed before implementation.

## License

MIT