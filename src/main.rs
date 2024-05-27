mod db {
    pub mod keyphrase_db;
    pub mod redis;
}
mod region {
    pub mod regions;
}
mod scrape {
    pub mod scrapers {
        pub mod forbes400;
        pub mod wikidata;
        pub mod wikipedia;
        pub mod youtube;
    }
    pub mod scraper_util;
}
mod util {
    pub mod log_service;
    pub mod path_service;
    pub mod venv_service;
    pub mod zip_service;
}
use crate::scrape::scraper_util::run_scrapers;
use db::keyphrase_db::gen_keyphrase_db;
use std::error::Error;
use util::{log_service::setup_logging, path_service::get_parent_dir, venv_service::create_venv};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    setup_logging().await?;
    let exe_parent = get_parent_dir().await?;
    create_venv(&exe_parent).await?;
    gen_keyphrase_db(exe_parent).await?;
    //regions::show_region_map().await?;
    run_scrapers().await?;

    Ok(())
}
