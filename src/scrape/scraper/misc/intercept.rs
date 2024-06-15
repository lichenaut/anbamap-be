use crate::db::util::url_exists;
use crate::prelude::*;
use crate::scrape::util::{
    get_regions, look_between, notify_parse_fail, strip_html, truncate_string,
};
use crate::service::var_service::is_source_enabled;
use chrono::Local;
use sqlx::SqlitePool;

pub async fn scrape_intercept(
    pool: &SqlitePool,
    media: &mut Vec<(String, String, String, Vec<String>)>,
) -> Result<()> {
    let intercept_enabled: bool = is_source_enabled("INTERCEPT_B").await?;
    if !intercept_enabled {
        return Ok(());
    }

    media.extend(
        scrape_intercept_stories(
            pool,
            &format!(
                "https://theintercept.com/{}/",
                Local::now().format("%Y/%m/%d")
            ),
        )
        .await?,
    );

    Ok(())
}

pub async fn scrape_intercept_stories(
    pool: &SqlitePool,
    url: &str,
) -> Result<Vec<(String, String, String, Vec<String>)>> {
    let mut stories: Vec<(String, String, String, Vec<String>)> = Vec::new();
    let response = reqwest::get(url).await?;
    if !response.status().is_success() {
        tracing::debug!(
            "Non-success response from The Intercept: {}",
            response.status()
        );
        return Ok(stories);
    }

    let mut response: String = response.text().await?;
    response = match look_between(
        &response,
        "<div class=\"max-w-[47rem] mx-auto my-8\">".to_string(),
        "<footer".to_string(),
    )? {
        Some(response) => response,
        None => {
            notify_parse_fail("The Intercept stories", &response);
            return Ok(stories);
        }
    };

    let items: Vec<&str> = response
        .split("<article class=\"content-card content-card--standard\"")
        .skip(1)
        .collect::<Vec<&str>>();
    for item in items {
        let url: String = match look_between(item, "href=\"".to_string(), "\"".to_string())? {
            Some(url) => url,
            None => {
                notify_parse_fail("The Intercept url", item);
                break;
            }
        };

        if url_exists(pool, &url).await? {
            break;
        }

        let title: String = match look_between(
            item,
            "<h3 class=\"content-card__title\">".to_string(),
            "</h3>".to_string(),
        )? {
            Some(title) => strip_html(title.trim())?,
            None => {
                notify_parse_fail("The Intercept title", item);
                break;
            }
        };

        let body: String = match look_between(
            item,
            "<div class=\"content-card__excerpt\">".to_string(),
            "</div>".to_string(),
        )? {
            Some(body) => strip_html(body.trim())?,
            None => {
                notify_parse_fail("The Intercept body", item);
                break;
            }
        };

        let regions = get_regions(&[&title, &body]).await?;
        stories.push((url, title, truncate_string(body)?, regions));
    }

    Ok(stories)
}
