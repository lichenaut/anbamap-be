use crate::db::util::url_exists;
use crate::prelude::*;
use crate::scrape::util::{get_regions, look_between, strip_html, truncate_string};
use crate::service::var_service::is_source_enabled;
use chrono::Local;
use sqlx::SqlitePool;

pub async fn scrape_consortium(
    pool: &SqlitePool,
    media: &mut Vec<(String, String, String, Vec<String>)>,
) -> Result<()> {
    let consortium_enabled: bool = is_source_enabled("CONSORTIUM_B").await?;
    if !consortium_enabled {
        return Ok(());
    }

    media.extend(
        scrape_consortium_posts(
            pool,
            &format!(
                "https://consortiumnews.com/{}/",
                Local::now().format("%Y/%m/%d")
            ),
        )
        .await?,
    );

    Ok(())
}

pub async fn scrape_consortium_posts(
    pool: &SqlitePool,
    url: &str,
) -> Result<Vec<(String, String, String, Vec<String>)>> {
    let mut posts: Vec<(String, String, String, Vec<String>)> = Vec::new();
    let response = reqwest::get(url).await?;
    if !response.status().is_success() {
        tracing::error!(
            "Non-success response from Consortium: {}",
            response.status()
        );
        return Ok(posts);
    }

    let mut response: String = response.text().await?;
    response = match look_between(
        &response,
        "<header id=\"archive-header\">".to_string(),
        "<div id=\"secondary\" class=\"c3 end\" role=\"complementary\">".to_string(),
    )
    .await?
    {
        Some(response) => response,
        None => return Ok(posts),
    };

    let items: Vec<&str> = response
        .split("<article id=")
        .skip(1)
        .collect::<Vec<&str>>();
    if items.is_empty() {
        return Ok(posts);
    }

    for item in items {
        let url: String = match look_between(item, "href=\"".to_string(), "\"".to_string()).await? {
            Some(url) => url,
            None => continue,
        };

        if url_exists(pool, &url).await? {
            break;
        }

        let title: String =
            match look_between(item, "rel=\"bookmark\">".to_string(), "<".to_string()).await? {
                Some(title) => strip_html(title).await?,
                None => continue,
            };

        let body: String = match look_between(
            item,
            "decoding=\"async\" /></a><p>".to_string(),
            "<".to_string(),
        )
        .await?
        {
            Some(body) => truncate_string(strip_html(body).await?).await?,
            None => continue,
        };

        let tags: String =
            match look_between(&response, "category".to_string(), "\"".to_string()).await? {
                Some(response) => response
                    .replace('-', " ")
                    .replace("category", "")
                    .replace("tag", ""),
                None => return Ok(posts),
            };

        let regions = get_regions(&[&title, &format!("{} {}", body, tags)]).await?;
        posts.push((url, title, body, regions));
    }

    Ok(posts)
}
