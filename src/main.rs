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
    pub mod keyphrase_db;
    pub mod upstash;
}
mod region {
    pub mod regions;
}

use db::keyphrase_db::gen_keyphrase_db;
use dotenv::dotenv;
use region::regions::KEYPHRASE_REGION_MAP;
use scrape::scraper_util::schedule_scrapers;
use std::{env, error::Error, process::Command, str};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt().with_max_level(tracing::Level::TRACE).init();
    dotenv().ok();

    gen_keyphrase_db().await?;

    let flashgeotext_update = Command::new("bash").arg("-c").arg(format!("source {} && pip install flashgeotext", env::var("PY_ENV_ACTIVATE_PATH")?)).output()?;
    tracing::info!("{}", str::from_utf8(&flashgeotext_update.stdout)?.trim().to_string());

    //regions::show_region_map().await?;

    schedule_scrapers().await?;

    Ok(())
}