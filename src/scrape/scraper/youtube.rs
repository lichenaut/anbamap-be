use crate::db::util::url_exists;
use crate::prelude::*;
use crate::scrape::util::{get_regions, notify_parse_fail, strip_html, truncate_string};
use crate::service::var_service::{get_youtube_api_key, get_youtube_channel_ids};
use chrono::Local;
use serde_json::Value;
use sqlx::SqlitePool;

pub async fn scrape_youtube(
    pool: &SqlitePool,
    media: &mut Vec<(String, String, String, Vec<String>)>,
) -> Result<()> {
    let youtube_api_key = match get_youtube_api_key().await? {
        Some(api_key) => api_key,
        None => return Ok(()),
    };

    let youtube_channel_ids = match get_youtube_channel_ids().await? {
        Some(channel_ids) => channel_ids,
        None => return Ok(()),
    };

    let youtube_channel_ids = youtube_channel_ids
        .split(',')
        .filter(|&s| !s.is_empty())
        .collect::<Vec<&str>>();
    for youtube_channel_id in youtube_channel_ids {
        media.extend(scrape_youtube_channel(pool, &youtube_api_key, youtube_channel_id).await?);
    }

    Ok(())
}

pub async fn scrape_youtube_channel(
    pool: &SqlitePool,
    api_key: &str,
    channel_id: &str,
) -> Result<Vec<(String, String, String, Vec<String>)>> {
    let mut videos: Vec<(String, String, String, Vec<String>)> = Vec::new();
    let url =
            format!("https://www.googleapis.com/youtube/v3/search?part=snippet&maxResults=50&channelId={}&type=video&order=date&key={}",
            channel_id,
            api_key
    );
    let response = reqwest::get(&url).await?;
    if !response.status().is_success() {
        tracing::debug!("Non-success response from Youtube: {}", response.status());
        return Ok(videos);
    }

    let json: Value = response.json().await?;
    let today: String = Local::now().format("%Y-%m-%d").to_string();
    let Some(items) = json["items"].as_array() else {
        notify_parse_fail("Youtube items", &json);
        return Ok(videos);
    };

    for item in items {
        let Some(snippet) = item["snippet"].as_object() else {
            notify_parse_fail("Youtube snippet", item);
            break;
        };

        let Some(published_at) = snippet["publishedAt"].as_str() else {
            notify_parse_fail("Youtube publishedAt", "snippet");
            break;
        };

        if published_at.chars().take(10).collect::<String>() != today {
            break;
        }

        let id = item["id"]["videoId"].as_str();
        let url: String = match id {
            Some(id) => format!("https://www.youtube.com/watch?v={}", id),
            None => {
                notify_parse_fail("Youtube videoId", item);
                break;
            }
        };

        if url_exists(pool, &url).await? {
            break;
        }

        let title: String = match snippet["title"].as_str() {
            Some(title) => strip_html(title)?,
            None => {
                notify_parse_fail("Youtube title", "snippet");
                break;
            }
        };

        let body: String = match snippet["description"].as_str() {
            Some(body) => strip_html(body)?,
            None => {
                notify_parse_fail("Youtube description", "snippet");
                break;
            }
        };

        let regions = get_regions(&[&title, &body]).await?;
        videos.push((url, title, truncate_string(body)?, regions));
    }

    Ok(videos)
}
