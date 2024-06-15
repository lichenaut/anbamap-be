use crate::db::util::url_exists;
use crate::prelude::*;
use crate::scrape::util::{
    get_regions, look_between, notify_parse_fail, strip_html, truncate_string,
};
use crate::service::var_service::get_substack_urls;
use chrono::Local;
use sqlx::SqlitePool;

pub async fn scrape_substack(
    pool: &SqlitePool,
    media: &mut Vec<(String, String, String, Vec<String>)>,
) -> Result<()> {
    let substack_urls = match get_substack_urls().await? {
        Some(urls) => urls,
        None => return Ok(()),
    };

    let substack_urls = substack_urls
        .split(',')
        .filter(|&s| !s.is_empty())
        .collect::<Vec<&str>>();
    for substack_url in substack_urls {
        media.extend(scrape_substack_archive(pool, substack_url).await?);
    }

    Ok(())
}

pub async fn scrape_substack_archive(
    pool: &SqlitePool,
    url: &str,
) -> Result<Vec<(String, String, String, Vec<String>)>> {
    let mut letters: Vec<(String, String, String, Vec<String>)> = Vec::new();
    let response = reqwest::get(url).await?;
    if !response.status().is_success() {
        tracing::debug!("Non-success response from Substack: {}", response.status());
        return Ok(letters);
    }

    let mut response: String = response.text().await?;
    response = match look_between(
        &response,
        "<div class=\"portable-archive-list\">".to_string(),
        "<div class=\"footer-wrap publication-footer\">".to_string(),
    )? {
        Some(response) => response,
        None => {
            notify_parse_fail("Substack archive", &response);
            return Ok(letters);
        }
    };

    let today: String = Local::now().format("%Y-%m-%d").to_string();
    let items: Vec<&str> = response
        .split("<div class=\"pencraft pc-display-flex pc-flexDirection-column pc-reset")
        .skip(1)
        .collect::<Vec<&str>>();
    for chunk in items.chunks(3) {
        let second: String = match chunk.get(1) {
            Some(second) => second.to_string(),
            None => {
                notify_parse_fail("Substack second chunk", chunk.join(" "));
                break;
            }
        };

        let date_time: String =
            match look_between(&second, "dateTime=\"".to_string(), "T".to_string())? {
                Some(date_time) => date_time,
                None => {
                    notify_parse_fail("Substack date", &second);
                    break;
                }
            };

        if date_time != today {
            break;
        }

        let first: String = match chunk.first() {
            Some(first) => first.to_string(),
            None => {
                notify_parse_fail("Substack first chunk", chunk.join(" "));
                break;
            }
        };

        let url: String = match look_between(&first, "href=\"".to_string(), "\"".to_string())? {
            Some(url) => url,
            None => {
                notify_parse_fail("Substack url", &first);
                break;
            }
        };

        if url_exists(pool, &url).await? {
            break;
        }

        let mut intermediate: String = match first.splitn(2, '>').last() {
            Some(intermediate) => intermediate.to_string(),
            None => {
                notify_parse_fail("Substack intermediate 1", &first);
                break;
            }
        };

        let title = match look_between(&intermediate, ">".to_string(), "</a>".to_string())? {
            Some(title) => strip_html(title)?,
            None => {
                notify_parse_fail("Substack title", &intermediate);
                break;
            }
        };

        intermediate = match second.splitn(2, '>').last() {
            Some(intermediate) => intermediate.to_string(),
            None => {
                notify_parse_fail("Substack intermediate 2", &second);
                break;
            }
        };

        let body: String = match look_between(&intermediate, ">".to_string(), "</a>".to_string())? {
            Some(body) => strip_html(body)?,
            None => {
                notify_parse_fail("Substack body", &intermediate);
                break;
            }
        };

        let regions = get_regions(&[&title, &body]).await?;
        letters.push((url, title, truncate_string(body)?, regions));
    }

    Ok(letters)
}
