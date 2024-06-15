use crate::db::util::url_exists;
use crate::prelude::*;
use crate::scrape::util::{
    get_regions, look_between, notify_parse_fail, strip_html, truncate_string,
};
use crate::service::var_service::is_source_enabled;
use chrono::Local;
use sqlx::SqlitePool;

pub async fn scrape_grayzone(
    pool: &SqlitePool,
    media: &mut Vec<(String, String, String, Vec<String>)>,
) -> Result<()> {
    let grayzone_enabled: bool = is_source_enabled("GRAYZONE_B").await?;
    if !grayzone_enabled {
        return Ok(());
    }

    let today: String = Local::now().format("%Y/%m/%d").to_string();
    media.extend(
        scrape_grayzone_stories(pool, &format!("https://thegrayzone.com/{}/", today), today)
            .await?,
    );

    Ok(())
}

pub async fn scrape_grayzone_stories(
    pool: &SqlitePool,
    url: &str,
    today: String,
) -> Result<Vec<(String, String, String, Vec<String>)>> {
    let mut stories: Vec<(String, String, String, Vec<String>)> = Vec::new();
    let response = reqwest::get(url).await?;
    if !response.status().is_success() {
        tracing::debug!("Non-success response from Grayzone: {}", response.status());
        return Ok(stories);
    }

    let mut response: String = response.text().await?;
    response = match look_between(
        &response,
        "<div id=\"cb-content\" class=\"contents-wrap clearfix wrap side-spacing sb--right\">"
            .to_string(),
        "<footer id=\"cb-footer\" class=\"site-footer\">".to_string(),
    )? {
        Some(response) => response,
        None => {
            notify_parse_fail("Grayzone stories", &response);
            return Ok(stories);
        }
    };

    let items: Vec<&str> = response
        .split("<div class=\"cb-mask mask\" style=\"background:#bc2c27;\">")
        .skip(1)
        .collect::<Vec<&str>>();
    for item in items {
        let url: String = match look_between(item, "href=\"/".to_string(), "\"".to_string())? {
            Some(url) => url,
            None => {
                notify_parse_fail("Grayzone url", item);
                break;
            }
        };

        if url_exists(pool, &url).await? || !url.contains(&today) {
            break;
        }

        let title: String = match look_between(
            item,
            "<h2 class=\"title cb-post-title\">".to_string(),
            "</h2>".to_string(),
        )? {
            Some(title) => strip_html(title.trim())?,
            None => {
                notify_parse_fail("Grayzone title", item);
                break;
            }
        };

        let response = reqwest::get(&url).await?;
        if !response.status().is_success() {
            tracing::debug!("Non-success response from Grayzone: {}", response.status());
            break;
        }

        let response: String = response.text().await?;
        let body: String = match look_between(
            &response,
            "description\" content=\"".to_string(),
            "\"".to_string(),
        )? {
            Some(body) => strip_html(body)?,
            None => {
                notify_parse_fail("Grayzone body", &response);
                break;
            }
        };

        let tags = match look_between(&response, "\"keywords\":[".to_string(), "]".to_string())? {
            Some(tags) => tags.replace('"', "").replace(',', " "),
            None => {
                notify_parse_fail("Grayzone tags", &response);
                break;
            }
        };

        let regions = get_regions(&[&title, &format!("{} {}", body, tags)]).await?;
        stories.push((url, title, truncate_string(body)?, regions));
    }

    Ok(stories)
}
