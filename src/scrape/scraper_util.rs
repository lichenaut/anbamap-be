use std::{error::Error, str};

pub async fn get_regions(text: &[&str]) -> Result<Vec<String>, Box<dyn Error>> {
    let text = text.join(" ").to_lowercase();
    let regions = Vec::new();
    //stuff
    println!("{:?}", regions);

    Ok(regions)
}