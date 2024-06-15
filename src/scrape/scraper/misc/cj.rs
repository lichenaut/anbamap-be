use crate::db::util::url_exists;
use crate::prelude::*;
use crate::scrape::util::{
    get_regions, look_between, notify_parse_fail, strip_html, truncate_string,
};
use crate::service::var_service::is_source_enabled;
use chrono::Local;
use sqlx::SqlitePool;

pub async fn scrape_cj(
    pool: &SqlitePool,
    media: &mut Vec<(String, String, String, Vec<String>)>,
) -> Result<()> {
    let cj_enabled: bool = is_source_enabled("CJ_B").await?;
    if !cj_enabled {
        return Ok(());
    }

    media.extend(
        scrape_cj_resources(pool, "https://caitlinjohnstone.com.au/category/article/").await?,
    );

    Ok(())
}

pub async fn scrape_cj_resources(
    pool: &SqlitePool,
    url: &str,
) -> Result<Vec<(String, String, String, Vec<String>)>> {
    let mut resources: Vec<(String, String, String, Vec<String>)> = Vec::new();
    let response = reqwest::get(url).await?;
    if !response.status().is_success() {
        tracing::debug!(
            "Non-success response from Caitlin Johnstone: {}",
            response.status()
        );
        return Ok(resources);
    }

    let mut response: String = response.text().await?;
    response = match look_between(
        &response,
        "Article</h1>".to_string(),
        "class=\"wp-block-group has-global-padding is-layout-constrained wp-container-core-group-is-layout-12 wp-block-group-is-layout-constrained\"".to_string(),
    )
    ?
    {
        Some(response) => response,
        None => {
            notify_parse_fail("Caitlin Johnstone resources", &response);
            return Ok(resources);
        }
    };

    let today: String = Local::now().format("%Y-%m-%d").to_string();
    let items: Vec<&str> = response
        .split("<figure style=\"aspect-ratio:3/2; margin-bottom:var(--wp--preset--spacing--40);\"")
        .skip(1)
        .collect::<Vec<&str>>();
    for item in items {
        let date_time: String =
            match look_between(item, "datetime=\"".to_string(), "T".to_string())? {
                Some(date_time) => date_time,
                None => {
                    notify_parse_fail("Caitlin Johnstone date", item);
                    break;
                }
            };

        if date_time != today {
            break;
        }

        let url: String = match look_between(item, "href=\"".to_string(), "\"".to_string())? {
            Some(url) => url,
            None => {
                notify_parse_fail("Caitlin Johnstone url", item);
                break;
            }
        };

        if url_exists(pool, &url).await? {
            break;
        }

        let title: String =
            match look_between(item, "target=\"_self\" >".to_string(), "</a>".to_string())? {
                Some(title) => strip_html(title)?,
                None => {
                    notify_parse_fail("Caitlin Johnstone title", item);
                    break;
                }
            };

        let response = reqwest::get(&url).await?;
        if !response.status().is_success() {
            tracing::error!(
                "Non-success response from Caitlin Johnstone: {}",
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
            Some(body) => strip_html(&body)?,
            None => {
                notify_parse_fail("Caitlin Johnstone body", &response);
                break;
            }
        };

        let tags: String = match look_between(
            &response,
            "<div class=\"taxonomy-post_tag has-link-color wp-elements-90c16d2487f1707e39afbb7d15aaa168 wp-block-post-terms has-text-color has-base-color has-small-font-size\">".to_string(),
            "</div>".to_string(),
        )? {
            Some(response) => strip_html(response)?,
            None => {
                notify_parse_fail("Caitlin Johnstone tags", &response);
                break;
            }
        };

        let regions = get_regions(&[&title, &format!("{} {}", body, tags)]).await?;
        resources.push((url, title, truncate_string(body)?, regions));
    }

    Ok(resources)
}
