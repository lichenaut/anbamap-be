use chrono::Utc;
use std::process::Command;
use serde_json::Value;
use std::{error::Error, fs, str};

pub async fn scrape_youtube_channel(channel_id: &str) -> Result<(), Box<dyn Error>> {
    let api_key = fs::read_to_string("keys/youtube.txt")?;
    let url = format!("https://www.googleapis.com/youtube/v3/search?part=snippet&maxResults=10&channelId={}&type=video&order=date&key={}", channel_id, api_key);
    let response = reqwest::get(&url).await?;
    let data: Value = response.json().await?;
    let today = Utc::now();

    if let Some(items) = data["items"].as_array() {
        for item in items {
            let title;
            let description;
            if let Some(snippet) = item["snippet"].as_object() {
                if let Some(published_at) = snippet["publishedAt"].as_str() {
                    if published_at.chars().take(10).collect::<String>() != today.format("%Y-%m-%d").to_string() { continue; }
                }

                title = snippet["title"].as_str().unwrap();
                description = snippet["description"].as_str().unwrap();
            } else {
                continue;
            }

            let url;
            let id = item["id"]["videoId"].as_str();
            match id {
                Some(id) => { url = format!("https://www.youtube.com/watch?v={}", id); },
                None => continue
            }

            // Preprocess title and description for flashgeotext
            let title_capitalized = title.split_whitespace().map(|word| {
                let mut chars = word.chars();
                match chars.next() {
                    None => String::new(),
                    Some(c) => c.to_uppercase().chain(chars).collect()
                }
            }).collect::<Vec<String>>().join(" ");
            let description_capitalized = &description.split_whitespace().map(|word| {
                let mut chars = word.chars();
                match chars.next() {
                    None => String::new(),
                    Some(c) => c.to_uppercase().chain(chars).collect()
                }
            }).collect::<Vec<String>>().join(" ");

            let countries = Command::new("bash").arg("-c").arg(format!("source /home/lichenaut/p3env/bin/activate && python -c 'import sys; sys.path.append(\".\"); from src.scrape.media_to_countries import get_countries; print(get_countries(\"{}\"))'", title_capitalized.to_owned() + " " + description_capitalized)).output()?;
            let countries = str::from_utf8(&countries.stdout)?.trim().to_string();
            let countries = if countries == "[]" {
                vec!["United States".to_string()]
            } else {
                let countries = countries.replace("', '", ", ").replace("['", "").replace("']", "").split(", ").map(|s| s.to_string()).collect::<Vec<String>>();
                if countries.len() == 1 && countries[0] == "" { vec!["United States".to_string()] } else { countries }
            };
            println!("{}: {}", title, countries.join(", "));
        }
    }

    Ok(())
}