use crate::db::util::url_exists;
use crate::prelude::*;
use crate::scrape::util::{
    get_regions, look_between, notify_parse_fail, strip_html, truncate_string,
};
use crate::service::var_service::is_source_enabled;
use chrono::Local;
use sqlx::SqlitePool;

pub async fn scrape_os(
    pool: &SqlitePool,
    media: &mut Vec<(String, String, String, Vec<String>)>,
) -> Result<()> {
    let os_enabled: bool = is_source_enabled("OS_B").await?;
    if !os_enabled {
        return Ok(());
    }

    media.extend(
        scrape_os_news(
            pool,
            &format!(
                "https://www.opensecrets.org/news/{}/",
                Local::now().format("%Y/%m")
            ),
        )
        .await?,
    );
    media.extend(
        scrape_os_reports(
            pool,
            &format!(
                "https://www.opensecrets.org/news/reports?year={}",
                Local::now().format("%Y")
            ),
        )
        .await?,
    );

    Ok(())
}

pub async fn scrape_os_news(
    pool: &SqlitePool,
    url: &str,
) -> Result<Vec<(String, String, String, Vec<String>)>> {
    let mut news: Vec<(String, String, String, Vec<String>)> = Vec::new();
    let response = reqwest::get(url).await?;
    if !response.status().is_success() {
        tracing::debug!(
            "Non-success response from OpenSecrets: {}",
            response.status()
        );
        return Ok(news);
    }

    let mut response: String = response.text().await?;
    response = match look_between(
        &response,
        "<div class=\"l-col-wide\" style=\"margin-bottom: 1rem;\">".to_string(),
        "<div class=\"control-container\">".to_string(),
    )? {
        Some(response) => response,
        None => {
            notify_parse_fail("OpenSecrets news", &response);
            return Ok(news);
        }
    };

    let today = Local::now().format("%B %-d, %Y").to_string();
    let this_month = Local::now().format("%Y/%m").to_string();
    let items: Vec<&str> = response
        .split("<div class=\"Card\">")
        .skip(1)
        .collect::<Vec<&str>>();
    for item in items {
        let date_time: String = match look_between(
            item,
            "</a></span> and </span>".to_string(),
            "</span>".to_string(),
        )? {
            Some(date_time) => strip_html(date_time.trim())?,
            None => {
                notify_parse_fail("OpenSecrets date", item);
                break;
            }
        };

        if date_time != today {
            break;
        }

        let mut url = url.to_string();
        let url_news: String = match look_between(
            item,
            format!("href=\"/news/{this_month}/"),
            "\"".to_string(),
        )? {
            Some(url) => url,
            None => {
                notify_parse_fail("OpenSecrets url", item);
                break;
            }
        };

        url.push_str(&url_news);
        if url_exists(pool, &url).await? {
            break;
        }

        let title: String = match look_between(item, "#555;\">".to_string(), "</a>".to_string())? {
            Some(title) => strip_html(title)?,
            None => {
                notify_parse_fail("OpenSecrets title", item);
                break;
            }
        };

        let body: String = match look_between(
            item,
            "<div class=\"Card-description\">".to_string(),
            "</p>".to_string(),
        )? {
            Some(body) => strip_html(body.trim())?,
            None => {
                notify_parse_fail("OpenSecrets body", item);
                break;
            }
        };

        let regions = get_regions(&[&title, &body]).await?;
        news.push((url, title, truncate_string(body)?, regions));
    }

    Ok(news)
}

pub async fn scrape_os_reports(
    pool: &SqlitePool,
    url: &str,
) -> Result<Vec<(String, String, String, Vec<String>)>> {
    let mut reports: Vec<(String, String, String, Vec<String>)> = Vec::new();
    let response = reqwest::get(url).await?;
    if !response.status().is_success() {
        tracing::debug!(
            "Non-success response from OpenSecrets: {}",
            response.status()
        );
        return Ok(reports);
    }

    let mut response: String = response.text().await?;
    response = match look_between(
        &response,
        "<div class=\"Reports-header\">".to_string(),
        "Feel free to distribute".to_string(),
    )? {
        Some(response) => response,
        None => {
            notify_parse_fail("OpenSecrets reports", &response);
            return Ok(reports);
        }
    };

    let today = Local::now().format("%B %-d, %Y").to_string();
    let items: Vec<&str> = response
        .split("<div class=\"report-card u-richtext u-mb4\">")
        .skip(1)
        .collect::<Vec<&str>>();
    for item in items {
        let date_time: String = match look_between(
            item,
            "<br><em>Published on ".to_string(),
            "</em></p>".to_string(),
        )? {
            Some(date_time) => date_time,
            None => {
                notify_parse_fail("OpenSecrets date", item);
                break;
            }
        };

        if date_time != today {
            break;
        }

        let mut url: String = url.chars().take(url.len() - 10).collect::<String>();
        let url_reports: String = match look_between(
            item,
            "[<a href=\"/news/reports".to_string(),
            "\"".to_string(),
        )? {
            Some(url) => url,
            None => {
                notify_parse_fail("OpenSecrets url", item);
                break;
            }
        };

        url.push_str(&url_reports);
        if url_exists(pool, &url).await? {
            break;
        }

        let title: String = match look_between(
            item,
            "<span class=\"report-card-title\">".to_string(),
            "</span>".to_string(),
        )? {
            Some(title) => strip_html(title)?,
            None => {
                notify_parse_fail("OpenSecrets title", item);
                break;
            }
        };

        let body: String = match look_between(item, "</em></p>".to_string(), "[".to_string())? {
            Some(body) => strip_html(body)?,
            None => {
                notify_parse_fail("OpenSecrets body", item);
                break;
            }
        };

        let regions = get_regions(&[&title, &body]).await?;
        reports.push((url, title, truncate_string(body)?, regions));
    }

    Ok(reports)
}
