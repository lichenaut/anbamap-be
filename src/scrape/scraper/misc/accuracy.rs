use crate::db::util::url_exists;
use crate::prelude::*;
use crate::scrape::util::{get_regions, look_between, strip_content, truncate_string};
use crate::service::var_service::is_source_enabled;
use chrono::Local;
use sqlx::SqlitePool;

pub async fn scrape_accuracy(
    pool: &SqlitePool,
    media: &mut Vec<(String, String, String, Vec<String>)>,
) -> Result<()> {
    let accuracy_enabled: bool = is_source_enabled("ACCURACY_B").await?;
    if !accuracy_enabled {
        return Ok(());
    }

    media.extend(scrape_accuracy_releases(pool, "https://accuracy.org/news-releases/").await?);

    Ok(())
}

pub async fn scrape_accuracy_releases(
    pool: &SqlitePool,
    url: &str,
) -> Result<Vec<(String, String, String, Vec<String>)>> {
    let mut releases: Vec<(String, String, String, Vec<String>)> = Vec::new();
    let response = reqwest::get(url).await?;
    if !response.status().is_success() {
        tracing::error!("Non-success response from Accuracy: {}", response.status());
        return Ok(releases);
    }

    let mut response: String = response.text().await?;
    response = match look_between(
        &response,
        "<div class=\"content-wrap\">".to_string(),
        "<p><a href=\"https://accuracy.org/news-releases/page/2/\" >".to_string(),
    )
    .await?
    {
        Some(response) => response,
        None => return Ok(releases),
    };

    let today: String = Local::now().format("%Y-%m-%d").to_string();
    let items: Vec<&str> = response
        .split("<div class=\"post list_container\">")
        .skip(1)
        .collect::<Vec<&str>>();
    for item in items {
        let date_time: String = match look_between(
            item,
            "<span class=\"date time published\" title=\"".to_string(),
            "T".to_string(),
        )
        .await?
        {
            Some(date_time) => date_time,
            None => continue,
        };

        if date_time != today {
            break;
        }

        let url: String =
            match look_between(item, "<a href=\"".to_string(), "\"".to_string()).await? {
                Some(url) => url,
                None => continue,
            };

        if url_exists(pool, &url).await? {
            continue;
        }

        let title = match look_between(item, "title=\"".to_string(), "\"".to_string()).await? {
            Some(title) => strip_content(title)
                .await?
                .replace("Permanent Link to ", ""),
            None => continue,
        };

        let body: String =
            match look_between(item, "</div></div>".to_string(), "</p>".to_string()).await? {
                Some(body) => truncate_string(strip_content(body).await?).await?,
                None => continue,
            };

        let regions = get_regions(&[&title, &body]).await?;
        releases.push((url, title, body, regions));
    }

    Ok(releases)
}
