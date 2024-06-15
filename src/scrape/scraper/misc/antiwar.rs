use crate::db::util::url_exists;
use crate::prelude::*;
use crate::scrape::util::{
    get_base_url, get_regions, look_between, notify_parse_fail, strip_html, truncate_string,
};
use crate::service::var_service::is_source_enabled;
use chrono::Local;
use sqlx::SqlitePool;
use std::process::Command;
use std::str::from_utf8;
use std::thread;
use std::time::Duration;

pub async fn scrape_antiwar(
    pool: &SqlitePool,
    docker_volume: &str,
    media: &mut Vec<(String, String, String, Vec<String>)>,
) -> Result<()> {
    let antiwar_enabled: bool = is_source_enabled("ANTIWAR_B").await?;
    if !antiwar_enabled {
        return Ok(());
    }

    media.extend(
        scrape_antiwar_features(pool, docker_volume, "https://www.antiwar.com/latest.php").await?,
    );

    Ok(())
}

#[allow(unused_assignments)]
pub async fn scrape_antiwar_features(
    pool: &SqlitePool,
    docker_volume: &str,
    url: &str,
) -> Result<Vec<(String, String, String, Vec<String>)>> {
    let mut features: Vec<(String, String, String, Vec<String>)> = Vec::new();
    let response = reqwest::get(url).await?;
    if !response.status().is_success() {
        tracing::debug!("Non-success response from Antiwar: {}", response.status());
        return Ok(features);
    }

    let mut response: String = response.text().await?;
    let today: String = Local::now().format("%B %d, %Y").to_string();
    let date = match look_between(
        &response,
        "<div align=\"right\">Updated ".to_string(),
        " -".to_string(),
    )? {
        Some(date) => date,
        None => {
            notify_parse_fail("Antiwar date", &response);
            return Ok(features);
        }
    };

    if date != today {
        return Ok(features);
    }

    response = match look_between(
        &response,
        "<tr><td colspan=\"2\"><h1>".to_string(),
        "<tr><td colspan=\"2\"><h1>".to_string(),
    )? {
        Some(response) => response,
        None => {
            notify_parse_fail("Antiwar features", &response);
            return Ok(features);
        }
    };

    let mut url_cache: Vec<String> = Vec::new();
    let delay = Duration::from_secs(10);
    url_cache.push(get_base_url(url)?);
    let items: Vec<&str> = response
        .split("<td width=\"50%\">")
        .skip(1)
        .collect::<Vec<&str>>();
    for item in items {
        let url: String = match look_between(item, "href=\"".to_string(), "\"".to_string())? {
            Some(url) => url,
            None => {
                notify_parse_fail("Antiwar url", item);
                break;
            }
        };

        if url_exists(pool, &url).await? {
            break;
        }

        let title: String = match look_between(item, ">".to_string(), "</a>".to_string())? {
            Some(title) => strip_html(title)?,
            None => {
                notify_parse_fail("Antiwar title", item);
                break;
            }
        };

        let base_url = get_base_url(&url)?;
        if url_cache.contains(&base_url) {
            url_cache.clear();
            thread::sleep(delay);
        } else {
            url_cache.push(base_url);
        }
        let mut body: Option<String> = None;
        if url.contains("antiwar.com") {
            let response = reqwest::get(&url).await?;
            if !response.status().is_success() {
                tracing::debug!("Non-success response from Antiwar: {}", response.status());
                break;
            }

            let response: String = response.text().await?;
            body = Some(
                match look_between(
                    &response,
                    "description\" content=\"".to_string(),
                    "\"".to_string(),
                )? {
                    Some(body) => body,
                    None => {
                        notify_parse_fail("Antiwar body", &response);
                        break;
                    }
                },
            );
        } else {
            let output = Command::new(format!("{}/p3venv/bin/python", docker_volume))
            .arg("-c")
            .arg(format!(
                "import sys; sys.path.append('{}'); from url_to_body import get_body; print(get_body('{}'))",
                docker_volume, url
            ))
            .output()?;
            if !output.status.success() {
                tracing::debug!(
                    "newspaper3k failed to get body from Antiwar: {}",
                    from_utf8(&output.stderr)?
                );
                continue;
            }

            let stdout = from_utf8(&output.stdout)?;
            if stdout.is_empty() {
                tracing::debug!("newspaper3k returned empty body from Antiwar");
                continue;
            }

            body = Some(stdout.to_string());
        }
        if let Some(body) = body {
            let body = strip_html(&body)?;
            let regions = get_regions(&[&title, &body]).await?;
            features.push((url, title, truncate_string(body)?, regions));
        }
    }

    Ok(features)
}
