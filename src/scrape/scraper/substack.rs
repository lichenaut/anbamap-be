use crate::prelude::*;
use crate::scrape::util::get_regions;
use crate::service::var_service::get_substack_urls;

pub async fn scrape_substack(media: &mut Vec<(String, String, String, Vec<String>)>) -> Result<()> {
    let substack_urls = match get_substack_urls().await? {
        Some(urls) => urls,
        None => return Ok(()),
    };

    let substack_urls = substack_urls
        .split(',')
        .filter(|&s| !s.is_empty())
        .collect::<Vec<&str>>();

    for substack_url in substack_urls {
        media.extend(scrape_substack_archive(substack_url).await?);
    }

    Ok(())
}

pub async fn scrape_substack_archive(
    url: &str,
) -> Result<Vec<(String, String, String, Vec<String>)>> {
    let mut letters: Vec<(String, String, String, Vec<String>)> = Vec::new();
    let response = reqwest::get(url).await?;
    if !response.status().is_success() {
        tracing::error!("Non-success response from Substack: {}", response.status());
        return Ok(letters);
    }

    let mut response: String = response.text().await?;
    response = match look_between(
        response,
        "<div class=\"portable-archive-list\">",
        "<div class=\"footer-wrap publication-footer\">",
    ) {
        Some(response) => response,
        None => return Ok(letters),
    };

    let now: String = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let items: Vec<&str> = response
        .split("<div class=\"pencraft pc-display-flex pc-flexDirection-column pc-reset")
        .skip(1)
        .collect::<Vec<&str>>();
    for chunk in items.chunks(3) {
        let second = match chunk.get(1) {
            Some(second) => second,
            None => continue,
        };

        let date_time: String = match look_between(second, "dateTime=\"", "\"") {
            Some(date_time) => date_time.chars().take(10).collect::<String>(),
            None => continue,
        };

        if date_time != now {
            continue;
        }

        let mut intermediate = match second.splitn(2, '>').last() {
            Some(intermediate) => intermediate,
            None => continue,
        };

        let body: String = match look_between(intermediate, ">", "<") {
            Some(body) => body,
            None => continue,
        };

        let first = match chunk.first() {
            Some(first) => first,
            None => continue,
        };

        let url = match look_between(first, "href=\"", "\"") {
            Some(url) => url,
            None => continue,
        };

        intermediate = match first.splitn(2, '>').last() {
            Some(intermediate) => intermediate,
            None => continue,
        };

        let title = match look_between(intermediate, ">", "<") {
            Some(title) => title,
            None => continue,
        };

        let regions = get_regions(&[&title, &body]).await?;
        letters.push((
            url,
            title.replace("&#39;", r"'").replace("&amp;", "&"),
            body.replace("&#39;", r"'").replace("&amp;", "&"),
            regions,
        ));
    }

    Ok(letters)
}

fn look_between<T: ToString>(text: T, this: &str, that: &str) -> Option<String> {
    let text = text.to_string();
    match text.splitn(2, this).last() {
        Some(text) => text.split(that).next().map(|text| text.to_string()),
        None => None,
    }
}
