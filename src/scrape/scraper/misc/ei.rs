use std::thread;
use std::time::Duration;

use crate::db::util::url_exists;
use crate::prelude::*;
use crate::scrape::util::{get_regions, look_between, strip_html, truncate_string};
use crate::service::var_service::is_source_enabled;
use chrono::Local;
use sqlx::SqlitePool;

pub async fn scrape_ei(
    pool: &SqlitePool,
    media: &mut Vec<(String, String, String, Vec<String>)>,
) -> Result<()> {
    let ei_enabled: bool = is_source_enabled("EI_B").await?;
    if !ei_enabled {
        return Ok(());
    }

    let delay = Duration::from_secs(10);
    media.extend(scrape_ei_blogs(pool, "https://electronicintifada.net/news", &delay).await?);
    thread::sleep(delay);
    media.extend(scrape_ei_blogs(pool, "https://electronicintifada.net/blog", &delay).await?);

    Ok(())
}

pub async fn scrape_ei_blogs(
    pool: &SqlitePool,
    url: &str,
    delay: &Duration,
) -> Result<Vec<(String, String, String, Vec<String>)>> {
    let mut blogs: Vec<(String, String, String, Vec<String>)> = Vec::new();
    let response = reqwest::get(url).await?;
    if !response.status().is_success() {
        tracing::error!(
            "Non-success response from Electronic Intifada: {}",
            response.status()
        );
        return Ok(blogs);
    }

    let mut response: String = response.text().await?;
    response = match look_between(
        &response,
        "<a id=\"main-content\"></a>".to_string(),
        "<ul class=\"pager pager-lite\">".to_string(),
    )? {
        Some(response) => response,
        None => return Ok(blogs),
    };

    let today: String = Local::now().format("%-d %B %Y").to_string();
    let items: Vec<&str> = response
        .split("<h2 class=\"node__title node-title\">")
        .skip(1)
        .collect::<Vec<&str>>();
    for item in items {
        let date_time: String = match look_between(
            item,
            "class=\"date-display-single\">".to_string(),
            "</span>".to_string(),
        )? {
            Some(date_time) => date_time,
            None => continue,
        };

        if date_time != today {
            break;
        }

        let mut url = url.chars().take(url.len() - 4).collect::<String>();
        let url_blog: String = match look_between(item, "href=\"/".to_string(), "\"".to_string())? {
            Some(url) => url,
            None => continue,
        };

        url.push_str(&url_blog);
        if url_exists(pool, &url).await? {
            break;
        }

        let title: String = match look_between(
            item,
            "class=\"balance-text\">".to_string(),
            "</a>".to_string(),
        )? {
            Some(title) => capitalize_words(strip_html(title)?)?,
            None => continue,
        };

        let body: String = match look_between(
            item,
            "</span></span> </p>".to_string(),
            "&nbsp;<a".to_string(),
        )? {
            Some(body) => truncate_string(strip_html(body)?)?,
            None => continue,
        };

        thread::sleep(*delay);
        let response = reqwest::get(&url).await?;
        if !response.status().is_success() {
            tracing::error!(
                "Non-success response from Electronic Intifada: {}",
                response.status()
            );
            break;
        }

        let response: String = response.text().await?;
        let tags = match look_between(
            &response,
            "<ul class=\"field field-tag\">".to_string(),
            "</ul>".to_string(),
        )? {
            Some(tags) => strip_html(tags)?,
            None => continue,
        };

        let regions = get_regions(&[&title, &format!("{} {}", body, tags)]).await?;
        blogs.push((url, title, body, regions));
    }

    Ok(blogs)
}

fn capitalize_words(s: String) -> Result<String> {
    Ok(s.split_whitespace()
        .map(|word| {
            let mut c = word.chars();
            match c.next() {
                None => String::new(),
                Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
            }
        })
        .collect::<Vec<String>>()
        .join(" "))
}
