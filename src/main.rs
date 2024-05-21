mod scrape {
    pub mod scrapers {
        pub mod forbes400;
        pub mod wikidata;
        pub mod wikipedia;
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
use db::{db_service::schedule_scrapers, keyphrase_db::gen_keyphrase_db};
use std::{error::Error, process::Command, str};
use region::regions::KEYPHRASE_REGION_MAP;

#[actix_web::main]
async fn main() -> Result<(), Box<dyn Error>> {
    //look for rayon uses in the codebase
    tracing_subscriber::fmt().with_max_level(tracing::Level::TRACE).init();

    gen_keyphrase_db().await?;

    //let flashgeotext_update = Command::new("bash").arg("-c").arg("source /home/lichenaut/p3env/bin/activate && pip install flashgeotext").output()?;
    //tracing::info!("{}", str::from_utf8(&flashgeotext_update.stdout)?.trim().to_string());

    //regions::show_region_map().await?;

    schedule_scrapers().await?;

    Ok(())
}