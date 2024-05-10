use chrono::Utc;
use serde_json::Value;
use super::super::scraper_util;
use std::{error::Error, fs, str};

pub async fn scrape_youtube_channel(channel_id: &str) -> Result<Vec<(String, String, String, Vec<String>)>, Box<dyn Error>> {
    let api_key = fs::read_to_string("keys/youtube.txt")?;
    let url =
            format!("https://www.googleapis.com/youtube/v3/search?part=snippet&maxResults=10&channelId={}&type=video&order=date&key={}", channel_id, api_key);
    let response = reqwest::get(&url).await?;
    let data: Value = response.json().await?;
    let today = Utc::now();
    let mut videos: Vec<(String, String, String, Vec<String>)> = Vec::new();

    if let Some(items) = data["items"].as_array() {
        for item in items {
            let title;
            let description;
            if let Some(snippet) = item["snippet"].as_object() {
                if let Some(published_at) = snippet["publishedAt"].as_str() {
                    if published_at.chars().take(10).collect::<String>() != today.format("%Y-%m-%d").to_string() { continue; }
                }

                title = match snippet["title"].as_str() {
                    Some(t) => t.to_string(),
                    None => continue,
                };
                description = match snippet["description"].as_str() {
                    Some(d) => d.to_string(),
                    None => continue,
                };
            } else {
                continue;
            }

            let url;
            let id = item["id"]["videoId"].as_str();
            match id {
                Some(id) => { url = format!("https://www.youtube.com/watch?v={}", id); },
                None => continue,
            }

            let regions = scraper_util::get_regions(&[&title, &description]).await?;
            videos.push((url, title, description, regions));
        }
    }

    Ok(videos)
}