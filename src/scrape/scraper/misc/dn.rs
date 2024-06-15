use crate::db::util::url_exists;
use crate::prelude::*;
use crate::scrape::util::{
    get_regions, look_between, notify_parse_fail, strip_html, truncate_string,
};
use crate::service::var_service::is_source_enabled;
use chrono::Local;
use sqlx::SqlitePool;

pub async fn scrape_dn(
    pool: &SqlitePool,
    media: &mut Vec<(String, String, String, Vec<String>)>,
) -> Result<()> {
    let dn_enabled: bool = is_source_enabled("DN_B").await?;
    if !dn_enabled {
        return Ok(());
    }

    media.extend(
        scrape_dn_headlines(
            pool,
            &format!(
                "https://www.democracynow.org/{}/headlines",
                Local::now().format("%Y/%-m/%-d")
            ),
        )
        .await?,
    );

    Ok(())
}

pub async fn scrape_dn_headlines(
    pool: &SqlitePool,
    url: &str,
) -> Result<Vec<(String, String, String, Vec<String>)>> {
    let mut headlines: Vec<(String, String, String, Vec<String>)> = Vec::new();
    let response = reqwest::get(url).await?;
    if !response.status().is_success() {
        tracing::debug!(
            "Non-success response from Democracy Now!: {}",
            response.status()
        );
        return Ok(headlines);
    }

    let mut response: String = response.text().await?;
    response = match look_between(
        &response,
        "<div id=\"headlines\">".to_string(),
        "<div class=\"fine_print grey_description\">".to_string(),
    )? {
        Some(response) => response,
        None => {
            notify_parse_fail("Democracy Now! headlines", &response);
            return Ok(headlines);
        }
    };

    let items: Vec<&str> = response
        .split("<div class=\"headline\"")
        .skip(1)
        .collect::<Vec<&str>>();
    for item in items {
        let mut url: String = url.to_string() + "#";
        let url_id: String = match look_between(item, "id=\"".to_string(), "\"".to_string())? {
            Some(url) => url,
            None => {
                notify_parse_fail("Democracy Now! url", item);
                break;
            }
        };

        url.push_str(&url_id);
        if url_exists(pool, &url).await? {
            break;
        }

        let title: String = match look_between(item, "<h2>".to_string(), "</h2>".to_string())? {
            Some(title) => strip_html(title)?,
            None => {
                notify_parse_fail("Democracy Now! title", item);
                break;
            }
        };

        let body: String = match look_between(
            item,
            "<div class=\"headline_summary\"><p>".to_string(),
            "</p>".to_string(),
        )? {
            Some(body) => strip_html(body)?,
            None => {
                notify_parse_fail("Democracy Now! body", item);
                break;
            }
        };

        let tags: String = match look_between(
            item,
            "<a data-ga-action=\"Headlines: Topic\"".to_string(),
            "</li></ul>".to_string(),
        )? {
            Some(tags) => strip_html("<".to_string() + &tags)?,
            None => {
                notify_parse_fail("Democracy Now! tags", item);
                break;
            }
        };

        let regions = get_regions(&[&title, &format!("{} {}", body, tags)]).await?;
        headlines.push((url, title, truncate_string(body)?, regions));
    }

    Ok(headlines)
}
