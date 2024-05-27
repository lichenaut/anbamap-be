use sentry::{init, release_name, ClientOptions};
use sentry_tracing::{EventFilter, SentryLayer};
use std::{env::var, error::Error, str};
use tracing_subscriber::{
    fmt,
    layer::{Layered, SubscriberExt},
    prelude::__tracing_subscriber_Layer,
    util::SubscriberInitExt,
    Registry,
};

pub async fn setup_logging() -> Result<(), Box<dyn Error>> {
    match var("SENTRY_DSN") {
        Ok(dsn) => match dsn.is_empty() {
            true => set_subscriber(None).await?,
            false => {
                let _guard = init((
                    dsn,
                    ClientOptions {
                        release: release_name!(),
                        ..Default::default()
                    },
                ));
                let sentry_layer = sentry_tracing::layer().event_filter(|md| match md.level() {
                    &tracing::Level::ERROR => EventFilter::Event,
                    _ => EventFilter::Ignore,
                });
                set_subscriber(Some(sentry_layer)).await?;
            }
        },
        Err(_) => set_subscriber(None).await?,
    }

    Ok(())
}

async fn set_subscriber<S>(layer: Option<SentryLayer<S>>) -> Result<(), Box<dyn Error>>
where
    SentryLayer<S>:
        __tracing_subscriber_Layer<Layered<tracing_subscriber::fmt::Layer<Registry>, Registry>>,
{
    match layer {
        Some(sentry_layer) => {
            tracing_subscriber::registry()
                .with(fmt::layer())
                .with(sentry_layer)
                .init();
        }
        None => tracing_subscriber::registry().with(fmt::layer()).init(),
    }

    Ok(())
}
