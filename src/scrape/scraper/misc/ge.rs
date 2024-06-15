use crate::db::util::url_exists;
use crate::prelude::*;
use crate::scrape::util::{
    get_regions, look_between, notify_parse_fail, strip_html, truncate_string,
};
use crate::service::var_service::is_source_enabled;
use chrono::Local;
use sqlx::SqlitePool;
use std::thread;
use std::time::Duration;

pub async fn scrape_ge(
    pool: &SqlitePool,
    media: &mut Vec<(String, String, String, Vec<String>)>,
) -> Result<()> {
    let ge_enabled: bool = is_source_enabled("GE_B").await?;
    if !ge_enabled {
        return Ok(());
    }

    media.extend(
        scrape_ge_reports(
            pool,
            &format!(
                "https://geopoliticaleconomy.com/{}/",
                Local::now().format("%Y/%m/%d")
            ),
        )
        .await?,
    );

    Ok(())
}

pub async fn scrape_ge_reports(
    pool: &SqlitePool,
    url: &str,
) -> Result<Vec<(String, String, String, Vec<String>)>> {
    let mut reports: Vec<(String, String, String, Vec<String>)> = Vec::new();
    let response = reqwest::get(url).await?;
    if !response.status().is_success() {
        tracing::debug!(
            "Non-success response from Geopolitical Economy Report: {}",
            response.status()
        );
        return Ok(reports);
    }

    let mut response: String = response.text().await?;
    response = match look_between(
        &response,
        "<div class=\"zox-main-blog zoxrel left zox100\">".to_string(),
        "<div class=\"zox-inf-more-wrap left zoxrel\">".to_string(),
    )? {
        Some(response) => response,
        None => {
            notify_parse_fail("Geopolitical Economy Report reports", &response);
            return Ok(reports);
        }
    };

    let delay = Duration::from_secs(20);
    let items: Vec<&str> = response
        .split("<div class=\"zox-art-title\">")
        .skip(1)
        .collect::<Vec<&str>>();
    for item in items {
        let url: String = match look_between(item, "href=\"/".to_string(), "\"".to_string())? {
            Some(url) => url,
            None => {
                notify_parse_fail("Geopolitical Economy Report url", item);
                break;
            }
        };

        if url_exists(pool, &url).await? {
            break;
        }

        let title: String = match look_between(
            item,
            "<h2 class=\"zox-s-title2\">".to_string(),
            "</h2>".to_string(),
        )? {
            Some(title) => strip_html(title)?,
            None => {
                notify_parse_fail("Geopolitical Economy Report title", item);
                break;
            }
        };

        thread::sleep(delay);
        let response = reqwest::get(&url).await?;
        if !response.status().is_success() {
            tracing::error!(
                "Non-success response from Geopolitical Economy Report: {}",
                response.status()
            );
            break;
        }

        let response: String = response.text().await?;
        let body: String = match look_between(
            &response,
            "description\" content=\"".to_string(),
            "\"".to_string(),
        )? {
            Some(body) => strip_html(body)?,
            None => {
                notify_parse_fail("Geopolitical Economy Report body", &response);
                break;
            }
        };

        let tags = match look_between(
            &response,
            "</span><span itemprop=\"keywords\">".to_string(),
            "</span>".to_string(),
        )? {
            Some(tags) => strip_html(tags)?,
            None => {
                notify_parse_fail("Geopolitical Economy Report tags", &response);
                break;
            }
        };

        let regions = get_regions(&[&title, &format!("{} {}", body, tags)]).await?;
        reports.push((url, title, truncate_string(body)?, regions));
    }

    Ok(reports)
}
