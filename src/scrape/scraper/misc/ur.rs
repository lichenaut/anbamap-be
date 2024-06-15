use crate::db::util::url_exists;
use crate::prelude::*;
use crate::scrape::util::{
    get_regions, look_between, notify_parse_fail, strip_html, truncate_string,
};
use crate::service::var_service::is_source_enabled;
use chrono::Local;
use sqlx::SqlitePool;

pub async fn scrape_ur(
    pool: &SqlitePool,
    media: &mut Vec<(String, String, String, Vec<String>)>,
) -> Result<()> {
    let ur_enabled: bool = is_source_enabled("UR_B").await?;
    if !ur_enabled {
        return Ok(());
    }

    media.extend(scrape_ur_posts(pool, "https://unicornriot.ninja/category/global/").await?);

    Ok(())
}

pub async fn scrape_ur_posts(
    pool: &SqlitePool,
    url: &str,
) -> Result<Vec<(String, String, String, Vec<String>)>> {
    let mut posts: Vec<(String, String, String, Vec<String>)> = Vec::new();
    let response = reqwest::get(url).await?;
    if !response.status().is_success() {
        tracing::debug!(
            "Non-success response from Unicorn Riot: {}",
            response.status()
        );
        return Ok(posts);
    }

    let mut response: String = response.text().await?;
    response = match look_between(
        &response,
        "<div class=\"archives wrapper\">".to_string(),
        "<div class=\"pagination-wrapper\">".to_string(),
    )? {
        Some(response) => response,
        None => {
            notify_parse_fail("Unicorn Riot posts", &response);
            return Ok(posts);
        }
    };

    let today: String = Local::now().format("%Y-%m-%d").to_string();
    let items: Vec<&str> = response
        .split("<article id=")
        .skip(1)
        .collect::<Vec<&str>>();
    for item in items {
        let date_time: String =
            match look_between(item, "datetime=\"".to_string(), "T".to_string())? {
                Some(date_time) => date_time,
                None => {
                    notify_parse_fail("Unicorn Riot date", item);
                    break;
                }
            };

        if date_time != today {
            break;
        }

        if !item.contains("<div class=\"archive-body-excerpt\">") {
            continue;
        }

        let url: String = match look_between(
            item,
            "<figure class=\"image story-featured-image story-featured-image-archive-home\">"
                .to_string(),
            "<img".to_string(),
        )? {
            Some(url) => strip_html(url.trim())?,
            None => {
                notify_parse_fail("Unicorn Riot url", item);
                break;
            }
        };

        if url_exists(pool, &url).await? {
            break;
        }

        let body: String = match look_between(
            item,
            "<div class=\"archive-body-excerpt\"><p>".to_string(),
            "</p>".to_string(),
        )? {
            Some(body) => strip_html(body)?,
            None => {
                notify_parse_fail("Unicorn Riot body", item);
                break;
            }
        };

        let title: String =
            match look_between(item, "rel=\"bookmark\">".to_string(), "</a>".to_string())? {
                Some(title) => strip_html(title.trim())?,
                None => {
                    notify_parse_fail("Unicorn Riot title", item);
                    break;
                }
            };

        let tags: Vec<String> =
            match look_between(&response, "class=\"".to_string(), "\"".to_string())? {
                Some(tags) => tags
                    .split(' ')
                    .filter_map(|word| {
                        if word.contains("tag-") {
                            Some(word.replace("tag", "").replace('-', " "))
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<String>>(),
                None => {
                    notify_parse_fail("Unicorn Riot tags", &response);
                    break;
                }
            };

        let regions = get_regions(&[&title, &format!("{} {:?}", body, tags)]).await?;
        posts.push((url, title, truncate_string(body)?, regions));
    }

    Ok(posts)
}
