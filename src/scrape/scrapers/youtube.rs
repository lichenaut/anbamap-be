use chrono::Utc;
use serde_json::Value;
use super::super::scraper_util;
use std::{env::var, error::Error, str};

pub async fn scrape_youtube_channel(channel_id: &str) -> Result<Vec<(String, String, String, Vec<String>)>, Box<dyn Error>> {
    let mut videos: Vec<(String, String, String, Vec<String>)> = Vec::new();
    let url =
            format!("https://www.googleapis.com/youtube/v3/search?part=snippet&maxResults=50&channelId={}&type=video&order=date&key={}",
            channel_id,
            var("YOUTUBE_API_KEY")?
    );
    let response = reqwest::get(&url).await?;
    if !response.status().is_success() {
        tracing::error!("Non-success response from Youtube: {}", response.status());
        return Ok(videos)
    }
    
    let json: Value = response.json().await?;
    let today = Utc::now();
    if let Some(items) = json["items"].as_array() {
        for item in items {
            let title;
            let description;
            if let Some(snippet) = item["snippet"].as_object() {

                if let Some(published_at) = snippet["publishedAt"].as_str() {
                    if published_at.chars().take(10).collect::<String>() != today.format("%Y-%m-%d").to_string() { continue; }
                }

                title = match snippet["title"].as_str() {
                    Some(title) => title.to_string(),
                    None => continue,
                };
                
                description = match snippet["description"].as_str() {
                    Some(description) => description.to_string(),
                    None => continue,
                };
            } else {
                continue;
            }

            let id = item["id"]["videoId"].as_str();
            let url = match id {
                Some(id) => { format!("https://www.youtube.com/watch?v={}", id) },
                None => continue,
            };

            let regions = scraper_util::get_regions(&[&title, &description]).await?;
            videos.push((url, title, description, regions));            
        }
    }

    Ok(videos)
}