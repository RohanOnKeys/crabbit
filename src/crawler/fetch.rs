use std::time::Duration;

use reqwest::Client;

const MAX_BODY_BYTES: usize = 5 * 1024 * 1024;

pub struct FetchedPage {
    pub url: String,
    pub title: String,
    pub body: String,
}

pub fn build_client(user_agent: &str) -> anyhow::Result<Client> {
    Ok(Client::builder()
        .user_agent(user_agent)
        .timeout(Duration::from_secs(15))
        .build()?)
}

pub async fn fetch_one(client: &Client, url: &str) -> anyhow::Result<FetchedPage> {
    let resp = client.get(url).send().await?.error_for_status()?;
    let bytes = resp.bytes().await?;
    let capped = &bytes[..bytes.len().min(MAX_BODY_BYTES)];
    let html = String::from_utf8_lossy(capped);

    let title = extract_title(&html).unwrap_or_else(|| url.to_string());
    let body = strip_tags(&html);

    Ok(FetchedPage { url: url.to_string(), title, body })
}

/// Crude `<title>` scrape; full HTML parsing is a later pass.
fn extract_title(html: &str) -> Option<String> {
    let lower = html.to_lowercase();
    let start = lower.find("<title>")? + "<title>".len();
    let end = lower[start..].find("</title>")? + start;
    Some(html[start..end].trim().to_string())
}

/// Crude tag stripping; full HTML/readability extraction is a later pass.
fn strip_tags(html: &str) -> String {
    let mut out = String::with_capacity(html.len());
    let mut in_tag = false;
    for c in html.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(c),
            _ => {}
        }
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_title() {
        let html = "<html><head><TITLE>Hello World</TITLE></head></html>";
        assert_eq!(extract_title(html), Some("Hello World".to_string()));
    }

    #[test]
    fn missing_title_is_none() {
        assert_eq!(extract_title("<html></html>"), None);
    }

    #[test]
    fn strips_tags_and_collapses_whitespace() {
        let html = "<p>Hello   <b>World</b></p>\n<p>Again</p>";
        assert_eq!(strip_tags(html), "Hello World Again");
    }
}
