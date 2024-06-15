use crate::prelude::*;
use crate::scrape::util::get_iso_from_name;
use reqwest::Client;
use serde_json::Value;
use std::collections::HashMap;

pub async fn get_largest_billionaires_map(client: &Client) -> Result<HashMap<String, Vec<String>>> {
    let mut billionaires: HashMap<String, Vec<String>> = HashMap::new();
    let url = "https://forbes400.onrender.com/api/forbes400/getAllBillionaires".to_string();
    let response = client.get(&url).send().await?;
    if !response.status().is_success() {
        tracing::debug!("Non-success response from Forbes400: {}", response.status());
        return Ok(billionaires);
    }

    let json: Value = response.json().await?;
    let data = match json.as_array() {
        Some(data) => data,
        None => return Ok(billionaires),
    };

    for billionaire in data {
        let Some(final_worth) = billionaire["finalWorth"].as_f64() else {
            continue;
        };

        if final_worth < 9900.0 {
            continue;
        }

        let citizenship = match billionaire["countryOfCitizenship"].as_str() {
            Some(citizenship) => match get_iso_from_name(citizenship) {
                Some(iso) => iso.to_string(),
                None => {
                    tracing::error!(
                        "Failed to get ISO code for country of citizenship: {}",
                        citizenship
                    );
                    continue;
                }
            },
            None => {
                tracing::error!(
                    "Failed to get country of citizenship for billionaire: {:?}",
                    billionaire
                );
                continue;
            }
        };

        let name = match billionaire["personName"].as_str() {
            Some(name) => name.to_string().replace(" & family", ""),
            None => {
                tracing::error!("Failed to get name for billionaire: {:?}", billionaire);
                continue;
            }
        };

        billionaires.entry(citizenship).or_default().push(name);
    }

    for (_, names) in billionaires.iter_mut() {
        names.retain(|name| {
            !name.to_lowercase().contains("chad")
                && !name.to_lowercase().contains("israel")
                && !name.to_lowercase().contains("jordan")
        });
    }

    Ok(billionaires)
}
