use crate::db::util::url_exists;
use crate::prelude::*;
use crate::scrape::util::{
    get_regions, look_between, notify_parse_fail, strip_html, truncate_string,
};
use crate::service::var_service::is_source_enabled;
use chrono::Local;
use sqlx::SqlitePool;

pub async fn scrape_truthout(
    pool: &SqlitePool,
    media: &mut Vec<(String, String, String, Vec<String>)>,
) -> Result<()> {
    let truthout_enabled: bool = is_source_enabled("TRUTHOUT_B").await?;
    if !truthout_enabled {
        return Ok(());
    }

    media.extend(scrape_truthout_news(pool, "https://truthout.org/latest/").await?);

    Ok(())
}

pub async fn scrape_truthout_news(
    pool: &SqlitePool,
    url: &str,
) -> Result<Vec<(String, String, String, Vec<String>)>> {
    let mut news: Vec<(String, String, String, Vec<String>)> = Vec::new();
    let response = reqwest::get(url).await?;
    if !response.status().is_success() {
        tracing::debug!("Non-success response from Truthout: {}", response.status());
        return Ok(news);
    }

    let mut response: String = response.text().await?;
    response = match look_between(
        &response,
        "<h1 class=\"articles__ti\">Latest</h1>".to_string(),
        "<nav aria-label=\"Pagination\"".to_string(),
    )? {
        Some(response) => response,
        None => {
            notify_parse_fail("Truthout news", &response);
            return Ok(news);
        }
    };

    let today: String = Local::now().format("%Y-%m-%d").to_string();
    let items: Vec<&str> = response
        .split("<div class=\"categories d-inline\">")
        .skip(1)
        .collect::<Vec<&str>>();
    for item in items {
        let date_time: String =
            match look_between(item, "datetime=\"".to_string(), "T".to_string())? {
                Some(date_time) => date_time,
                None => {
                    notify_parse_fail("Truthout date", item);
                    break;
                }
            };

        if date_time != today {
            break;
        }

        let url: String =
            match look_between(item, "itemprop=\"headline\">".to_string(), ">".to_string())? {
                Some(url) => {
                    match look_between(url.trim(), "<a href=\"".to_string(), "\"".to_string())? {
                        Some(url) => url,
                        None => {
                            notify_parse_fail("Truthout url", item);
                            break;
                        }
                    }
                }
                None => {
                    notify_parse_fail("Truthout url", item);
                    break;
                }
            };

        if url_exists(pool, &url).await? {
            break;
        }

        let title: String = match look_between(
            item,
            "itemprop=\"headline\">".to_string(),
            "</a>".to_string(),
        )? {
            Some(title) => strip_html(title.trim())?,
            None => {
                notify_parse_fail("Truthout title", item);
                break;
            }
        };

        let body: String = match look_between(
            item,
            "itemprop=\"description\">".to_string(),
            "</div>".to_string(),
        )? {
            Some(body) => strip_html(body.trim())?,
            None => {
                notify_parse_fail("Truthout body", item);
                break;
            }
        };

        let regions = get_regions(&[&title, &body]).await?;
        news.push((url, title, truncate_string(body)?, regions));
    }

    Ok(news)
}
