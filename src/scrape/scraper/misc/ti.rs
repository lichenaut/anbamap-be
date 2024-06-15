use crate::db::util::url_exists;
use crate::prelude::*;
use crate::scrape::util::{
    get_regions, look_between, notify_parse_fail, strip_html, truncate_string,
};
use crate::service::var_service::is_source_enabled;
use chrono::Local;
use sqlx::SqlitePool;

pub async fn scrape_ti(
    pool: &SqlitePool,
    media: &mut Vec<(String, String, String, Vec<String>)>,
) -> Result<()> {
    let ti_enabled: bool = is_source_enabled("TI_B").await?;
    if !ti_enabled {
        return Ok(());
    }

    let today = Local::now().format("%m%d%Y").to_string();
    media.extend(
        scrape_ti_investigations(
            pool,
            &format!(
                "https://www.typeinvestigations.org/all/?post_date={}+{}/",
                today, today
            ),
        )
        .await?,
    );

    Ok(())
}

pub async fn scrape_ti_investigations(
    pool: &SqlitePool,
    url: &str,
) -> Result<Vec<(String, String, String, Vec<String>)>> {
    let mut investigations: Vec<(String, String, String, Vec<String>)> = Vec::new();
    let response = reqwest::get(url).await?;
    if !response.status().is_success() {
        tracing::debug!(
            "Non-success response from Type Investigations: {}",
            response.status()
        );
        return Ok(investigations);
    }

    let mut response: String = response.text().await?;
    response = match look_between(
        &response,
        "<div class=\"posts-grid\">".to_string(),
        "<aside class=\"col-12 col-lg-3 archive-sidebar\">".to_string(),
    )? {
        Some(response) => response,
        None => {
            notify_parse_fail("Type Investigations investigations", &response);
            return Ok(investigations);
        }
    };

    let items: Vec<&str> = response
        .split("<article role=\"article\"")
        .skip(1)
        .collect::<Vec<&str>>();
    for item in items {
        let url: String = match look_between(item, "href=\"".to_string(), "\"".to_string())? {
            Some(url) => url,
            None => {
                notify_parse_fail("Type Investigations url", item);
                break;
            }
        };

        if url_exists(pool, &url).await? {
            break;
        }

        let title: String =
            match look_between(item, "<header>".to_string(), "</header>".to_string())? {
                Some(title) => strip_html(title.trim())?,
                None => {
                    notify_parse_fail("Type Investigations title", item);
                    break;
                }
            };

        let body: String = match look_between(
            item,
            "<div class=\"post-excerpt mb-2\">".to_string(),
            "</div>".to_string(),
        )? {
            Some(body) => strip_html(body.trim())?,
            None => {
                notify_parse_fail("Type Investigations body", item);
                break;
            }
        };

        let regions = get_regions(&[&title, &body]).await?;
        investigations.push((url, title, truncate_string(body)?, regions));
    }

    Ok(investigations)
}
