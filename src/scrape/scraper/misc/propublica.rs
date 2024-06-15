use crate::db::util::url_exists;
use crate::prelude::*;
use crate::scrape::util::{
    get_regions, look_between, notify_parse_fail, strip_html, truncate_string,
};
use crate::service::var_service::is_source_enabled;
use chrono::{DateTime, FixedOffset, Local, NaiveDateTime};
use sqlx::SqlitePool;

pub async fn scrape_propublica(
    pool: &SqlitePool,
    media: &mut Vec<(String, String, String, Vec<String>)>,
) -> Result<()> {
    let propublica_enabled: bool = is_source_enabled("PROPUBLICA_B").await?;
    if !propublica_enabled {
        return Ok(());
    }

    media.extend(
        scrape_propublica_news(
            pool,
            &format!(
                "https://www.propublica.org/archive/{}/",
                Local::now().format("%Y/%m")
            ),
        )
        .await?,
    );

    Ok(())
}

pub async fn scrape_propublica_news(
    pool: &SqlitePool,
    url: &str,
) -> Result<Vec<(String, String, String, Vec<String>)>> {
    let mut news: Vec<(String, String, String, Vec<String>)> = Vec::new();
    let response = reqwest::get(url).await?;
    if !response.status().is_success() {
        tracing::debug!(
            "Non-success response from ProPublica: {}",
            response.status()
        );
        return Ok(news);
    }

    let mut response: String = response.text().await?;
    response = match look_between(
        &response,
        "<div class=\"stories\">".to_string(),
        "<nav class=\"pagination\">".to_string(),
    )? {
        Some(response) => response,
        None => {
            notify_parse_fail("ProPublica news", &response);
            return Ok(news);
        }
    };

    let today: String = Local::now().format("%Y-%m-%d").to_string();
    let items: Vec<&str> = response
        .split("<div class=\"story-entry")
        .skip(1)
        .collect::<Vec<&str>>();
    for item in items {
        let date_time: String =
            match look_between(item, "datetime=\"".to_string(), "\"".to_string())? {
                Some(date_time) => date_time,
                None => {
                    notify_parse_fail("ProPublica date", item);
                    break;
                }
            };

        let naive_date_time =
            NaiveDateTime::parse_from_str(&date_time.replace("EDT", ""), "%Y-%m-%d%H:%M")?;
        let offset = match FixedOffset::east_opt(14400) {
            Some(offset) => offset,
            None => {
                notify_parse_fail("ProPublica offset", item);
                break;
            }
        };

        let date_time: DateTime<FixedOffset> =
            DateTime::from_naive_utc_and_offset(naive_date_time, offset);
        let date_time = date_time
            .with_timezone(&chrono::Local)
            .format("%Y-%m-%d")
            .to_string();
        if date_time != today {
            break;
        }

        let body: String =
            match look_between(item, "<p class=\"dek\">".to_string(), "</p>".to_string())? {
                Some(body) => strip_html(body)?,
                None => {
                    notify_parse_fail("ProPublica body", item);
                    break;
                }
            };

        if body.is_empty() {
            continue;
        }

        let url: String = match look_between(item, "href=\"".to_string(), "\"".to_string())? {
            Some(url) => url,
            None => {
                notify_parse_fail("ProPublica url", item);
                break;
            }
        };

        if url_exists(pool, &url).await? {
            break;
        }

        let title: String = match look_between(
            item,
            "<div class=\"description\">".to_string(),
            "</a>".to_string(),
        )? {
            Some(title) => strip_html(title.trim())?,
            None => {
                notify_parse_fail("ProPublica title", item);
                break;
            }
        };

        let regions = get_regions(&[&title, &body]).await?;
        news.push((url, title, truncate_string(body)?, regions));
    }

    Ok(news)
}
