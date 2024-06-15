use crate::db::util::url_exists;
use crate::prelude::*;
use crate::scrape::util::{
    get_regions, look_between, notify_parse_fail, strip_html, truncate_string,
};
use crate::service::var_service::is_source_enabled;
use chrono::Local;
use sqlx::SqlitePool;

pub async fn scrape_jc(
    pool: &SqlitePool,
    media: &mut Vec<(String, String, String, Vec<String>)>,
) -> Result<()> {
    let jc_enabled: bool = is_source_enabled("JC_B").await?;
    if !jc_enabled {
        return Ok(());
    }

    media.extend(
        scrape_jc_blogs(
            pool,
            &format!(
                "https://www.jonathan-cook.net/blog/{}/",
                Local::now().format("%Y-%m-%d")
            ),
        )
        .await?,
    );

    Ok(())
}

pub async fn scrape_jc_blogs(
    pool: &SqlitePool,
    url: &str,
) -> Result<Vec<(String, String, String, Vec<String>)>> {
    let mut blogs: Vec<(String, String, String, Vec<String>)> = Vec::new();
    let response = reqwest::get(url).await?;
    if !response.status().is_success() {
        tracing::debug!(
            "Non-success response from Jonathan Cook: {}",
            response.status()
        );
        return Ok(blogs);
    }

    let mut response: String = response.text().await?;
    response = match look_between(
        &response,
        "<header class=\"headline_area\">".to_string(),
        "<div class=\"sidebar\">".to_string(),
    )? {
        Some(response) => response,
        None => {
            notify_parse_fail("Jonathan Cook blogs", &response);
            return Ok(blogs);
        }
    };

    let items: Vec<&str> = response
        .split("<article id=")
        .skip(1)
        .collect::<Vec<&str>>();
    for item in items {
        let url: String = match look_between(item, "href=\"".to_string(), "\"".to_string())? {
            Some(url) => url,
            None => {
                notify_parse_fail("Jonathan Cook url", item);
                break;
            }
        };

        if url_exists(pool, &url).await? {
            break;
        }

        let title: String =
            match look_between(item, "rel=\"bookmark\">".to_string(), "</a>".to_string())? {
                Some(title) => strip_html(title.trim())?,
                None => {
                    notify_parse_fail("Jonathan Cook title", item);
                    break;
                }
            };

        // Crawl delay is five minutes: not worth it for body and tags.
        let regions = get_regions(&[&title]).await?;
        blogs.push((url, title.clone(), truncate_string(title)?, regions));
    }

    Ok(blogs)
}
