use crate::db::util::url_exists;
use crate::prelude::*;
use crate::scrape::util::{get_regions, look_between, strip_html, truncate_string};
use crate::service::var_service::is_source_enabled;
use chrono::Local;
use sqlx::SqlitePool;

pub async fn scrape_amnesty(
    pool: &SqlitePool,
    media: &mut Vec<(String, String, String, Vec<String>)>,
) -> Result<()> {
    let amnesty_enabled: bool = is_source_enabled("AMNESTY_B").await?;
    if !amnesty_enabled {
        return Ok(());
    }

    media.extend(scrape_amnesty_resources(pool, "https://www.amnestyusa.org/news/").await?);

    Ok(())
}

pub async fn scrape_amnesty_resources(
    pool: &SqlitePool,
    url: &str,
) -> Result<Vec<(String, String, String, Vec<String>)>> {
    let mut resources: Vec<(String, String, String, Vec<String>)> = Vec::new();
    let response = reqwest::get(url).await?;
    if !response.status().is_success() {
        tracing::error!(
            "Non-success response from Amnesty USA: {}",
            response.status()
        );
        return Ok(resources);
    }

    let mut response: String = response.text().await?;
    response = match look_between(
        &response,
        "<div class=\"ts-grid-col-3-outline\">".to_string(),
        "<div class=\"p-site xl:container\">".to_string(),
    )? {
        Some(response) => response,
        None => return Ok(resources),
    };

    let today: String = Local::now().format("%B %d, %Y").to_string();
    let items: Vec<&str> = response
        .split("class=\"hocus-headline\"")
        .skip(1)
        .collect::<Vec<&str>>();
    for item in items {
        let date_time: String = match look_between(
            item,
            "<p class=\"card-md--tag--hocus utility-2xs mt-xs text-gray-300\">".to_string(),
            "<".to_string(),
        )? {
            Some(date_time) => date_time,
            None => continue,
        };

        if date_time.trim() != today {
            break;
        }

        let url: String = match look_between(item, "href=\"".to_string(), "\"".to_string())? {
            Some(url) => url,
            None => continue,
        };

        if url_exists(pool, &url).await? {
            break;
        }

        let title: String = match look_between(
            item,
            "<h3 class=\"utility-md\">".to_string(),
            "<".to_string(),
        )? {
            Some(title) => strip_html(title.trim())?,
            None => continue,
        };

        let body: String = match look_between(
            item,
            "<p class=\"body-xs mt-xs\">".to_string(),
            "<".to_string(),
        )? {
            Some(body) => truncate_string(strip_html(body)?)?,
            None => continue,
        };

        let regions = get_regions(&[&title, &body]).await?;
        resources.push((url, title, body, regions));
    }

    Ok(resources)
}
