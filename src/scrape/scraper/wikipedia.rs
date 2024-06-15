use crate::prelude::*;
use crate::scrape::util::get_iso_from_name;
use reqwest::Client;
use serde_json::{from_str, Value};
use std::collections::HashMap;
use wikitext_table_parser::{
    parser::{Event, WikitextTableParser},
    tokenizer::{get_all_cell_text_special_tokens, get_all_table_special_tokens, Tokenizer},
};

pub async fn get_private_enterprises_map(client: &Client) -> Result<HashMap<String, Vec<String>>> {
    let mut region_to_companies: HashMap<String, Vec<String>> = HashMap::new();
    let url = "https://en.wikipedia.org/w/api.php?action=query&prop=revisions&rvprop=content&rvslots=main&format=json&titles=List_of_largest_private_non-governmental_companies_by_revenue";
    let response = client.get(url).send().await?;
    if !response.status().is_success() {
        tracing::debug!("Non-success response from Wikipedia: {}", response.status());
        return Ok(region_to_companies);
    }

    let text = response.text().await?;
    let parsed: Value = from_str(&text)?;
    let content = match parsed["query"]["pages"].as_object().and_then(|pages| {
        pages
            .values()
            .next()
            .and_then(|page| page["revisions"][0]["slots"]["main"]["*"].as_str())
    }) {
        Some(content) => content,
        None => {
            tracing::error!("Failed to get content from Wikipedia response");
            return Ok(region_to_companies);
        }
    };

    let mut current_region;
    let mut current_enterprise = String::new();
    let wikitext_table_parser = WikitextTableParser::new(
        Tokenizer::build(get_all_table_special_tokens()),
        Tokenizer::build(get_all_cell_text_special_tokens()),
        content,
        true,
    );

    for event in wikitext_table_parser {
        let Event::ColEnd(text) = event else {
            continue;
        };

        let first_char = match text.chars().next() {
            Some(first_char) => first_char,
            None => continue,
        };

        if text.starts_with("[[") && text.ends_with("]]") {
            current_enterprise = text[2..text.len() - 2].to_string();
            if !current_enterprise.contains('|') {
                continue;
            }

            current_enterprise = match current_enterprise.split('|').last() {
                Some(current_enterprise) => current_enterprise.to_string(),
                None => continue,
            };
        } else if !first_char.is_numeric() && !text.starts_with('[') && !text.starts_with('"') {
            current_region = text.to_string();
            if !current_region.contains('.') {
                continue;
            }

            current_region = match current_region.split('.').next() {
                Some(current_region) => current_region.to_string(),
                None => continue,
            };

            let current_region_code = match get_iso_from_name(&current_region) {
                Some(current_region_code) => current_region_code.to_string(),
                None => continue,
            };

            region_to_companies
                .entry(current_region_code)
                .or_default()
                .push(current_enterprise.clone());
        }
    }

    Ok(region_to_companies)
}
