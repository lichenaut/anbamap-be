use crate::db::util::url_exists;
use crate::prelude::*;
use crate::scrape::util::{
    get_regions, look_between, notify_parse_fail, strip_html, truncate_string,
};
use crate::service::var_service::is_source_enabled;
use chrono::Local;
use sqlx::SqlitePool;

pub async fn scrape_hrw(
    pool: &SqlitePool,
    media: &mut Vec<(String, String, String, Vec<String>)>,
) -> Result<()> {
    let hrw_enabled: bool = is_source_enabled("HRW_B").await?;
    if !hrw_enabled {
        return Ok(());
    }

    media.extend(scrape_hrw_releases(pool, "https://www.hrw.org/news").await?);

    Ok(())
}

pub async fn scrape_hrw_releases(
    pool: &SqlitePool,
    url: &str,
) -> Result<Vec<(String, String, String, Vec<String>)>> {
    let mut releases: Vec<(String, String, String, Vec<String>)> = Vec::new();
    let response = reqwest::get(url).await?;
    if !response.status().is_success() {
        tracing::debug!(
            "Non-success response from Human Rights Watch: {}",
            response.status()
        );
        return Ok(releases);
    }

    let mut response: String = response.text().await?;
    response = match look_between(
        &response,
        "<div class=\"form-group mx-sm-3\">".to_string(),
        "<nav class=\"pager\" role=\"navigation\" aria-labelledby=\"pagination-heading\">"
            .to_string(),
    )? {
        Some(response) => response,
        None => {
            notify_parse_fail("Human Rights Watch releases", &response);
            return Ok(releases);
        }
    };

    let today: String = Local::now().format("%B %-d, %Y").to_string();
    let items: Vec<&str> = response
        .split("<article class=\"media-block flex w-full flex-row-reverse justify-end \">")
        .skip(1)
        .collect::<Vec<&str>>();
    for item in items {
        let date_time: String = match look_between(
            item,
            "<span class=\"media-block__date\">".to_string(),
            "</span>".to_string(),
        )? {
            Some(date_time) => date_time,
            None => {
                notify_parse_fail("Human Rights Watch date", item);
                break;
            }
        };

        if date_time != today {
            break;
        }

        let mut url: String = url.to_string();
        let url_release: String =
            match look_between(item, "href=\"/news".to_string(), "\"".to_string())? {
                Some(url) => url,
                None => {
                    notify_parse_fail("Human Rights Watch url", item);
                    break;
                }
            };

        url.push_str(&url_release);
        if url_exists(pool, &url).await? {
            break;
        }

        let title: String =
            match look_between(item, "\"><span>".to_string(), "</span>".to_string())? {
                Some(title) => strip_html(title)?,
                None => {
                    notify_parse_fail("Human Rights Watch title", item);
                    break;
                }
            };

        let response = reqwest::get(&url).await?;
        if !response.status().is_success() {
            tracing::error!(
                "Non-success response from Human Rights Watch: {}",
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
            Some(body) => strip_html(body)?,
            None => {
                notify_parse_fail("Human Rights Watch body", &response);
                break;
            }
        };

        let tags = match look_between(
            &response,
            "<ul class=\"tag-block__region-list flex flex-wrap\">".to_string(),
            "</ul>".to_string(),
        )? {
            Some(tags) => strip_html(tags.trim())?,
            None => {
                notify_parse_fail("Human Rights Watch tags", &response);
                break;
            }
        };

        let regions = get_regions(&[&title, &format!("{} {}", body, tags)]).await?;
        releases.push((url, title, truncate_string(body)?, regions));
    }

    Ok(releases)
}
