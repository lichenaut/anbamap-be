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
    pub mod path_service;
    pub mod venv_service;
    pub mod zip_service;
}
use crate::scrape::scraper_util::run_scrapers;
use db::keyphrase_db::gen_keyphrase_db;
use region::regions::KEYPHRASE_REGION_MAP;
use sentry::{init, release_name, ClientOptions};
use sentry_tracing::EventFilter;
use std::{
    env::{current_exe, var},
    error::Error,
    str,
};
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt};
use util::{path_service::get_parent_dir, venv_service::create_venv};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let sentry_dsn = var("SENTRY_DSN")?;
    if sentry_dsn.is_empty() {
        tracing_subscriber::registry().with(fmt::layer()).init();
    } else {
        let _guard = init((
            sentry_dsn,
            ClientOptions {
                release: release_name!(),
                ..Default::default()
            },
        ));
        let sentry_layer = sentry_tracing::layer().event_filter(|md| match md.level() {
            &tracing::Level::ERROR => EventFilter::Event,
            _ => EventFilter::Ignore,
        });
        tracing_subscriber::registry()
            .with(fmt::layer())
            .with(sentry_layer)
            .init();
    }

    let exe_parent = get_parent_dir().await?;
    create_venv(&exe_parent).await?;
    gen_keyphrase_db(exe_parent).await?;
    //regions::show_region_map().await?;
    run_scrapers().await?;

    Ok(())
}
