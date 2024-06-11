use crate::prelude::*;
use crate::scrape::util::{get_regions, look_between, strip_content, truncate_string};
use crate::service::var_service::get_substack_urls;
use chrono::Local;

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
        &response,
        "<div class=\"portable-archive-list\">".to_string(),
        "<div class=\"footer-wrap publication-footer\">".to_string(),
    )
    .await?
    {
        Some(response) => response,
        None => return Ok(letters),
    };

    let today: String = Local::now().format("%Y-%m-%d").to_string();
    let items: Vec<&str> = response
        .split("<div class=\"pencraft pc-display-flex pc-flexDirection-column pc-reset")
        .skip(1)
        .collect::<Vec<&str>>();
    for chunk in items.chunks(3) {
        let second: String = match chunk.get(1) {
            Some(second) => second.to_string(),
            None => continue,
        };

        let date_time: String =
            match look_between(&second, "dateTime=\"".to_string(), "T".to_string()).await? {
                Some(date_time) => date_time,
                None => continue,
            };

        if date_time != today {
            break;
        }

        let mut intermediate: String = match second.splitn(2, '>').last() {
            Some(intermediate) => intermediate.to_string(),
            None => continue,
        };

        let body: String =
            match look_between(&intermediate, ">".to_string(), "<".to_string()).await? {
                Some(body) => truncate_string(strip_content(body).await?).await?,
                None => continue,
            };

        let first: String = match chunk.first() {
            Some(first) => first.to_string(),
            None => continue,
        };

        let url: String =
            match look_between(&first, "href=\"".to_string(), "\"".to_string()).await? {
                Some(url) => url,
                None => continue,
            };

        intermediate = match first.splitn(2, '>').last() {
            Some(intermediate) => intermediate.to_string(),
            None => continue,
        };

        let title = match look_between(&intermediate, ">".to_string(), "<".to_string()).await? {
            Some(title) => strip_content(title).await?,
            None => continue,
        };

        let regions = get_regions(&[&title, &body]).await?;
        letters.push((url, title, body, regions));
    }

    Ok(letters)
}
