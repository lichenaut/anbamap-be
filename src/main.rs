mod db {
    pub mod keyphrase;
    pub mod media;
    pub mod util;
}
mod scrape {
    pub mod scraper {
        pub mod misc {
            pub mod accuracy;
            pub mod amnesty;
            pub mod antiwar;
            pub mod cj;
            pub mod consortium;
            pub mod dn;
            pub mod ei;
            pub mod ge;
            pub mod grayzone;
            pub mod hrw;
            pub mod intercept;
            pub mod jc;
            pub mod os;
            pub mod propublica;
            pub mod ti;
            pub mod truthout;
            pub mod ur;
        }
        pub mod forbes400;
        pub mod substack;
        pub mod wikidata;
        pub mod wikipedia;
        pub mod youtube;
    }
    pub mod region;
    pub mod util;
}
mod service {
    pub mod scrape_service;
    pub mod var_service;
    pub mod venv_service;
    pub mod zip_service;
}
mod prelude;
use crate::prelude::*;
use db::keyphrase::gen_keyphrase_db;
//use scrape::region;
use service::{
    scrape_service::run_scrapers, var_service::get_docker_volume, venv_service::create_venv,
};
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::registry().with(fmt::layer()).init();
    let docker_volume = get_docker_volume().await?;
    create_venv(&docker_volume).await?;
    gen_keyphrase_db(&docker_volume).await?;
    //region::show_region_map().await?;
    run_scrapers(&docker_volume).await?;

    Ok(())
}
