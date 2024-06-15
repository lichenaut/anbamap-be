use crate::db::{
    media::update_media_db,
    util::{create_media_db, get_db_pool},
};
use crate::prelude::*;
use crate::scrape::scraper::{
    misc::{
        accuracy::scrape_accuracy, amnesty::scrape_amnesty, antiwar::scrape_antiwar, cj::scrape_cj,
        consortium::scrape_consortium, dn::scrape_dn, ei::scrape_ei, ge::scrape_ge,
        grayzone::scrape_grayzone, hrw::scrape_hrw, intercept::scrape_intercept, jc::scrape_jc,
        os::scrape_os, propublica::scrape_propublica, ti::scrape_ti, truthout::scrape_truthout,
        ur::scrape_ur,
    },
    {substack::scrape_substack, youtube::scrape_youtube},
};
use std::path::Path;

pub async fn run_scrapers(docker_volume: &str) -> Result<()> {
    let db_path = format!("{}/media_db.sqlite", docker_volume);
    let db_path = Path::new(&db_path);
    let pool = get_db_pool(db_path).await?;
    create_media_db(&pool).await?;
    let mut media = Vec::new();
    scrape_accuracy(&pool, &mut media).await?;
    scrape_amnesty(&pool, &mut media).await?;
    scrape_antiwar(&pool, docker_volume, &mut media).await?;
    scrape_cj(&pool, &mut media).await?;
    scrape_consortium(&pool, &mut media).await?;
    scrape_dn(&pool, &mut media).await?;
    scrape_ei(&pool, &mut media).await?;
    scrape_ge(&pool, &mut media).await?;
    scrape_grayzone(&pool, &mut media).await?;
    scrape_hrw(&pool, &mut media).await?;
    scrape_intercept(&pool, &mut media).await?;
    scrape_jc(&pool, &mut media).await?;
    scrape_os(&pool, &mut media).await?;
    scrape_propublica(&pool, &mut media).await?;
    scrape_ti(&pool, &mut media).await?;
    scrape_truthout(&pool, &mut media).await?;
    scrape_ur(&pool, &mut media).await?;
    scrape_substack(&pool, &mut media).await?;
    scrape_youtube(&pool, &mut media).await?;
    update_media_db(&pool, media).await?;

    Ok(())
}
