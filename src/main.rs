mod scrape {
    pub mod scrapers {
        pub mod youtube;
    }
    pub mod scraper_util;
}
use scrape::scrapers::youtube::scrape_youtube_channel;
use std::{error::Error/*, process::Command*/};

#[actix_web::main]
async fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt().with_max_level(tracing::Level::TRACE).init();
    //let update = Command::new("bash").arg("-c").arg("source /home/lichenaut/p3env/bin/activate && pip install flashgeotext").output()?;
    scrape_youtube_channel("UC8p1vwvWtl6T73JiExfWs1g").await?;

    Ok(())
}