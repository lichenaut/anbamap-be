use std::{error::Error, str};
use std::process::Command;
use rayon::iter::IntoParallelIterator;
use unidecode::unidecode;
use rayon::prelude::*;
use crate::KEYPHRASE_REGION_MAP;

pub async fn get_regions(text: &[&str]) -> Result<Vec<String>, Box<dyn Error>> {
    let text = text.join(" ").replace("&#39;", "'").replace("'s ", " ").replace("s' ", " ");
    let regions = Command::new("bash").arg("-c").arg(format!("source /home/lichenaut/p3env/bin/activate && python -c 'import sys; sys.path.append(\".\"); from src.region.media_to_regions import get_regions; print(get_regions(\"{}\"))'", text)).output()?;
    let regions = str::from_utf8(&regions.stdout)?.trim().to_string();
    let regions: Vec<String> = regions
            .replace("[", "")
            .replace("]", "")
            .replace("'", "")
            .split(", ")
            .map(|s| s.to_string())
            .collect();

    let mut regions = regions.into_par_iter().filter(|s| match s.as_str() {
        "Chad" | "Georgia" | "Guinea-Bissau" | "Jordan" | "Republic of Congo" => false,
        _ => true,
    }).collect::<Vec<String>>();

    let text = unidecode(&text.to_lowercase());
    for (keyphrases, region) in KEYPHRASE_REGION_MAP.iter() {
        if regions.contains(&region.to_string()) { continue; }
        
        for keyphrase in keyphrases.iter() {
            if text.contains(keyphrase) {
                regions.push(region.to_string());
                break;
            }
        }
    }

    Ok(regions)
}

//fn that tests every name into AI