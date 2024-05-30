mod db {
    pub mod keyphrase_db;
    pub mod redis;
}
mod scrape {
    pub mod scraper {
        pub mod forbes400;
        pub mod wikidata;
        pub mod wikipedia;
        pub mod youtube;
    }
    pub mod region;
    pub mod scraper_util;
}
mod util {
    pub mod path_service;
    pub mod var_service;
    pub mod venv_service;
    pub mod zip_service;
}
mod prelude;
use crate::prelude::*;
use crate::scrape::scraper_util::run_scrapers;
use db::keyphrase_db::gen_keyphrase_db;
use util::{path_service::get_parent_dir, var_service::set_logging, venv_service::create_venv};

#[tokio::main]
async fn main() -> Result<()> {
    set_logging().await?;
    let exe_parent = get_parent_dir().await?;
    create_venv(&exe_parent).await?;
    gen_keyphrase_db(exe_parent).await?;
    //regions::show_region_map().await?;
    run_scrapers().await?;

    Ok(())
}
