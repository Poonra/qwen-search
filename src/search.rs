use anyhow::{Context, Result};
use scraper::{Html, Selector};
use std::time::Duration;

const USER_AGENT: &str = "Mozilla/5.0 (X11; Linux x86_64; rv:128.0) Gecko/20100101 Firefox/128.0";
#[derive(Debug, Clone)]
pub struct SearchResult {
    pub title: String,
    pub url: String,
    pub snippet: String,
}

pub struct Websearch {
    http: reqwest::Client,
}

impl Websearch {
    pub fn new() -> Result<Self> {
        let http = reqwest::Client::builder().user_agent(USER_AGENT).timeout(Duration::from_secs(10)).build()?;

        Ok(Self { http })
    }

    pub async fn search(&self, query: &str, max: usize) -> Result<Vec<SearchResult>> {
        let html = self.http.post("https://html.duckduckgo.com/html/")
            .form(&[("q", query)])
            .send()
            .await
            .context("Error sending search request")?
            .error_for_status()?.text().await?;
        Ok(parse_ddg(&html, max))
    }
}

fn parse_ddg(html: &str, max: usize) -> Vec<SearchResult> {
    let doc = Html::parse_document(html);
    let result_sel = Selector::parse("div.result:not(.result--ad)").unwrap();
    let title_sel = Selector::parse("a.result__a").unwrap();
    let snippet_sel = Selector::parse(".result__snippet").unwrap();

    doc.select(&result_sel)
        .filter_map(|r| {
            let a = r.select(&title_sel).next()?;
            let title = a.text().collect::<String>().trim().to_string();
            let url = clean_ddg_url(a.value().attr("href")?);
            let snippet = r
                .select(&snippet_sel)
                .next()
                .map(|s| s.text().collect::<String>().trim().to_string())
                .unwrap_or_default();
            Some(SearchResult { title, url, snippet })
        })
        .take(max)
        .collect()
}

fn clean_ddg_url(href: &str) -> String {
    let full = if href.starts_with("//") { format!("https:{href}") } else { href.to_string() };
    if let Ok(u) = url::Url::parse(&full) {
        if let Some((_, real)) = u.query_pairs().find(|(k, _)| k == "uddg") {
            return real.to_string();
        }
    }
    full
}

pub fn format_results(results: &[SearchResult]) -> String {
    if results.is_empty() {
        return "no results found".into();
    }
    results.iter().enumerate().map(|(i, r)| {
        let snippet: String = r.snippet.chars().take(300).collect();
        format!("[{}] {}\n{}\n{}", i + 1, r.title, r.url, snippet)
    })
        .collect::<Vec<_>>()
        .join("\n\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_results_and_skips_ads() {
        let html = r#"
            <div class="result result--ad">
              <a class="result__a" href="https://ad.example">Buy stuff</a>
            </div>
            <div class="result">
              <a class="result__a" href="//duckduckgo.com/l/?uddg=https%3A%2F%2Fwww.rust-lang.org%2F&rut=abc">Rust</a>
              <a class="result__snippet">A language empowering everyone.</a>
            </div>"#;
        let results = parse_ddg(html, 5);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title, "Rust");
        assert_eq!(results[0].url, "https://www.rust-lang.org/");
        assert_eq!(results[0].snippet, "A language empowering everyone.");
    }
}
