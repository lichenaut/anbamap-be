mod scrape {
    pub mod scrapers {
        pub mod wikidata;
        pub mod wikimedia;
        pub mod youtube;
    }
    pub mod scraper_util;
}
mod db {
    pub mod db_service;
    pub mod keyphrase_db;
}
mod region {
    pub mod regions;
}
use db::keyphrase_db::gen_keyphrase_db;
use scrape::scrapers::youtube::scrape_youtube_channel;
use region::regions::KEYPHRASE_REGION_MAP;
use std::{error::Error, process::Command, str};

#[actix_web::main]
async fn main() -> Result<(), Box<dyn Error>> { 
    //look for rayon uses in the codebase
    //tracing_subscriber::fmt().with_max_level(tracing::Level::TRACE).init();
    gen_keyphrase_db().await?;
    //let flashgeotext_update = Command::new("bash").arg("-c").arg("source /home/lichenaut/p3env/bin/activate && pip install flashgeotext").output()?;
    //tracing::info!("{}", str::from_utf8(&flashgeotext_update.stdout)?.trim().to_string());
    scrape_youtube_channel("UCNye-wNBqNL5ZzHSJj3l8Bg").await?;
    Ok(())
}