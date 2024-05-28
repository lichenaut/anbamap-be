use redis::Client;
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

pub async fn get_redis_client() -> Result<Client, Box<dyn Error>> {
    let redis_password = match var("REDIS_PASSWORD") {
        Ok(password) => match password.is_empty() {
            true => {
                let err = "REDIS_PASSWORD is empty";
                tracing::error!(err);
                return Err(err.into());
            }
            false => password,
        },
        Err(e) => {
            let err = format!("REDIS_PASSWORD not found in environment: {:?}", e);
            tracing::error!(err);
            return Err(err.into());
        }
    };

    let redis_endpoint = match var("REDIS_ENDPOINT") {
        Ok(endpoint) => match endpoint.is_empty() {
            true => {
                let err = "REDIS_ENDPOINT is empty";
                tracing::error!(err);
                return Err(err.into());
            }
            false => endpoint,
        },
        Err(e) => {
            let err = format!("REDIS_ENDPOINT not found in environment: {:?}", e);
            tracing::error!(err);
            return Err(err.into());
        }
    };

    Ok(Client::open(format!(
        "rediss://default:{}@{}",
        redis_password, redis_endpoint
    ))?)
}

pub async fn set_logging() -> Result<(), Box<dyn Error>> {
    match var("SENTRY_DSN") {
        Ok(dsn) => match dsn.is_empty() {
            true => {
                tracing::info!("SENTRY_DSN is empty");
                set_subscriber(None).await?;
            }
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
        Err(_) => {
            tracing::info!("SENTRY_DSN not found in environment");
            set_subscriber(None).await?;
        }
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

pub async fn get_youtube_api_key() -> Result<Option<String>, Box<dyn Error>> {
    match var("YOUTUBE_API_KEY") {
        Ok(api_key) => match api_key.is_empty() {
            true => {
                tracing::info!("YOUTUBE_API_KEY is empty");
                return Ok(None);
            }
            false => Ok(Some(api_key)),
        },
        Err(e) => {
            tracing::info!("YOUTUBE_API_KEY not found in environment: {}", e);
            return Ok(None);
        }
    }
}

pub async fn get_youtube_channel_ids() -> Result<Option<String>, Box<dyn Error>> {
    match var("YOUTUBE_CHANNEL_IDS") {
        Ok(channel_ids) => match channel_ids.is_empty() {
            true => {
                tracing::info!("YOUTUBE_CHANNEL_IDS is empty");
                return Ok(None);
            }
            false => Ok(Some(channel_ids)),
        },
        Err(e) => {
            tracing::info!("YOUTUBE_CHANNEL_IDS not found in environment: {}", e);
            return Ok(None);
        }
    }
}
