mod scrape {
    pub mod youtube;
}
use scrape::youtube::scrape_youtube_channel;
use std::error::Error;

#[actix_web::main]
async fn main() -> Result<(), Box<dyn Error>> {
    //TODO: update flashgeotext cmd 
    scrape_youtube_channel("UC8p1vwvWtl6T73JiExfWs1g").await?;

    Ok(())
}