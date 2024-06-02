use std::collections::HashSet;

use crate::prelude::*;
use crate::scrape::util::get_regions;
use crate::service::var_service::{get_youtube_api_key, get_youtube_channel_ids};
use chrono::Utc;
use serde_json::Value;

pub async fn scrape_youtube(
    media: &mut Vec<(String, String, String, HashSet<String>)>,
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
        media.extend(scrape_youtube_channel(&youtube_api_key, youtube_channel_id).await?);
    }

    Ok(())
}

pub async fn scrape_youtube_channel(
    api_key: &str,
    channel_id: &str,
) -> Result<Vec<(String, String, String, HashSet<String>)>> {
    let mut videos: Vec<(String, String, String, HashSet<String>)> = Vec::new();
    let url =
            format!("https://www.googleapis.com/youtube/v3/search?part=snippet&maxResults=50&channelId={}&type=video&order=date&key={}",
            channel_id,
            api_key
    );
    let response = reqwest::get(&url).await?;
    if !response.status().is_success() {
        tracing::error!("Non-success response from Youtube: {}", response.status());
        return Ok(videos);
    }

    let json: Value = response.json().await?;
    let today = Utc::now();
    let Some(items) = json["items"].as_array() else {
        return Ok(videos);
    };

    for item in items {
        let Some(snippet) = item["snippet"].as_object() else {
            continue;
        };

        let Some(published_at) = snippet["publishedAt"].as_str() else {
            continue;
        };

        if published_at.chars().take(10).collect::<String>() != today.format("%Y-%m-%d").to_string()
        {
            continue;
        }

        let title = match snippet["title"].as_str() {
            Some(title) => title.to_string(),
            None => continue,
        };

        let description = match snippet["description"].as_str() {
            Some(description) => description.to_string(),
            None => continue,
        };

        let id = item["id"]["videoId"].as_str();
        let url = match id {
            Some(id) => format!("https://www.youtube.com/watch?v={}", id),
            None => continue,
        };

        let regions = get_regions(&[&title, &description]).await?;
        videos.push((url, title, description, regions));
    }

    Ok(videos)
}
