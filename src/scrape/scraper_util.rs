use std::{error::Error, str};
use std::process::Command;
use rayon::iter::IntoParallelIterator;
use unidecode::unidecode;
use rayon::prelude::*;
use crate::REGION_MAP;

pub async fn get_regions(text: &[&str]) -> Result<Vec<String>, Box<dyn Error>> {
    let text = unidecode(&text.join(" "));
    let regions = Command::new("bash").arg("-c").arg(format!("source /home/lichenaut/p3env/bin/activate && python -c 'import sys; sys.path.append(\".\"); from src.scrape.media_to_regions import get_regions; print(get_regions(\"{}\"))'", text)).output()?;
    let regions = str::from_utf8(&regions.stdout)?.trim().to_string();
    let regions: Vec<String> = regions
            .replace("[", "")
            .replace("]", "")
            .replace("'", "")
            .split(", ")
            .map(|s| s.to_string())
            .collect();

    let regions = regions.into_par_iter().filter(|s| match s.as_str() {
        "Georgia" | "Guinea-Bissau" => false,
        _ => true,
    }).collect::<Vec<String>>();

    // only check region map entries with values not already present in regions vec, make sure to quit out of searching for a region's relevancy the moment it is found

    Ok(regions)
}
