use crate::indexer::stemmer::EnglishStemmer;
use crate::indexer::tokenizer;

/// Tokenizes and stems a raw query string into index-matching terms.
pub fn parse_query(query: &str) -> Vec<String> {
    let stemmer = EnglishStemmer::new();
    tokenizer::tokenize(query)
        .into_iter()
        .map(|t| stemmer.stem(&t))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stems_and_tokenizes_query_terms() {
        assert_eq!(parse_query("Running Frameworks"), vec!["run", "framework"]);
    }
}
