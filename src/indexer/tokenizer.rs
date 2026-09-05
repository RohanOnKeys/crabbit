/// Lowercases, strips punctuation, and splits on whitespace.
pub fn tokenize(text: &str) -> Vec<String> {
    text.split_whitespace()
        .map(|word| {
            word.chars()
                .filter(|c| c.is_alphanumeric())
                .collect::<String>()
                .to_lowercase()
        })
        .filter(|w| !w.is_empty())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_lowercases_and_strips_punctuation() {
        assert_eq!(
            tokenize("Rust's Web-Framework, fast!"),
            vec!["rusts", "webframework", "fast"]
        );
    }

    #[test]
    fn empty_string_yields_no_tokens() {
        assert!(tokenize("").is_empty());
    }

    #[test]
    fn whitespace_only_yields_no_tokens() {
        assert!(tokenize("   \t\n  ").is_empty());
    }

    #[test]
    fn handles_unicode_word_chars() {
        assert_eq!(tokenize("café naïve"), vec!["café", "naïve"]);
    }
}
