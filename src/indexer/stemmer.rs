use rust_stemmers::{Algorithm, Stemmer};

pub struct EnglishStemmer(Stemmer);

impl EnglishStemmer {
    pub fn new() -> Self {
        Self(Stemmer::create(Algorithm::English))
    }

    pub fn stem(&self, word: &str) -> String {
        self.0.stem(word).into_owned()
    }
}

impl Default for EnglishStemmer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stems_known_word_pairs() {
        let stemmer = EnglishStemmer::new();
        assert_eq!(stemmer.stem("running"), "run");
        assert_eq!(stemmer.stem("indexing"), "index");
        assert_eq!(stemmer.stem("frameworks"), "framework");
    }
}
