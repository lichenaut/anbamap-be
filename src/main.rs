mod db {
    pub mod keyphrase_db;
    pub mod upstash;
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
    pub mod env_service;
    pub mod zip_service;
}
use db::keyphrase_db::gen_keyphrase_db;
use region::regions::KEYPHRASE_REGION_MAP;
use scrape::scraper_util::schedule_scrapers;
use sentry::{ClientOptions, init, release_name};
use sentry_tracing::EventFilter;
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt};
use std::{env::var, error::Error, process::Command, str};
use crate::util::env_service::check_env_vars;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let exe_parent = match check_env_vars().await {
        Ok(exe_parent) => exe_parent,
        Err(e) => {
            println!("Err while checking environment variables: {}", e);
            return Ok(())
        }
    };

    let sentry_dsn = var("SENTRY_DSN")?;
    if sentry_dsn.is_empty() {
        tracing_subscriber::registry()
                .with(fmt::layer())
                .init();
    } else {
        let _guard = init((sentry_dsn, ClientOptions {
            release: release_name!(),
            ..Default::default()
        }));
        let sentry_layer = sentry_tracing::layer().event_filter(|md| match md.level() {
            &tracing::Level::ERROR => EventFilter::Event,
            _ => EventFilter::Ignore,
        });
        tracing_subscriber::registry()
                .with(fmt::layer())
                .with(sentry_layer)
                .init();
    }

    let flashgeotext_update = Command::new("bash").arg("-c").arg(format!("source {} && pip install flashgeotext", var("PY_ENV_ACTIVATE_PATH")?)).output()?;
    tracing::debug!("Flashgeotext pip installation output: {}", str::from_utf8(&flashgeotext_update.stdout)?.trim().to_string());

    gen_keyphrase_db(exe_parent).await?;

    //regions::show_region_map().await?;

    schedule_scrapers().await?;

    Ok(())
}