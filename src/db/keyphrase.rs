use super::util::get_db_pool;
use crate::prelude::*;
use crate::scrape::scraper::forbes400::get_largest_billionaires_map;
use crate::scrape::scraper::wikidata::{region_code_to_figures, verify_codes};
use crate::scrape::scraper::wikipedia::get_private_enterprises_map;
use crate::service::zip_service::{zip_from_url, zip_to_txt};
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use reqwest::Client;
use sqlx::sqlite::SqliteQueryResult;
use sqlx::Executor;
use std::{collections::HashSet, io::BufRead, path::Path};
use unidecode::unidecode;

struct Region {
    region_code: String,
    keyphrases: String,
}

pub async fn gen_keyphrase_db(docker_volume: &str) -> Result<()> {
    let db_path = format!("{}/region_db.sqlite", docker_volume);
    let db_path = Path::new(&db_path);
    if db_path.exists() {
        tracing::info!("region_db.sqlite found. Skipping keyphrase database generation.");
        return Ok(());
    }

    let client = Client::new();
    let all_countries_path = format!("{}/allCountries.txt", docker_volume);
    let all_countries_path = Path::new(&all_countries_path);
    if all_countries_path.exists() {
        tracing::info!("allCountries.txt found. Skipping download and decompression.");
    } else {
        let zip_path = format!("{}/allCountries.zip", docker_volume);
        if !Path::new(&zip_path).exists() {
            tracing::info!("allCountries.zip not found. Downloading allCountries.zip.");
            zip_from_url(
                &client,
                "https://download.geonames.org/export/dump/allCountries.zip",
                &zip_path,
            )
            .await?;
        }
        tracing::info!("Decompressing allCountries.zip.");
        zip_to_txt(&zip_path).await?;
    }

    let pool = get_db_pool(db_path).await?;
    pool.execute(
        "CREATE TABLE IF NOT EXISTS regions (
            region_code TEXT PRIMARY KEY,
            keyphrases TEXT
        )",
    )
    .await?;

    let reader = std::io::BufReader::new(std::fs::File::open(all_countries_path)?);
    for line in reader.lines() {
        let line = line?;
        let mut fields = line.split('\t');
        let ascii_name = match fields.nth(2) {
            Some(ascii_name) => ascii_name,
            None => continue,
        };

        let feature_class = match fields.nth(3) {
            Some(feature_class) => feature_class,
            None => continue,
        };

        let feature_code = match fields.next() {
            Some(feature_code) => feature_code,
            None => continue,
        };

        if feature_code.contains('H') {
            continue;
        }

        let region_code = match fields.next() {
            Some(region_code) => region_code,
            None => continue,
        };

        if region_code == "MZ" && ascii_name.contains("aza") {
            continue;
        } // "Gaza" is a better keyphrase for Palestine than Mozambique.

        let population = match fields.nth(5) {
            Some(population) => match population.parse::<u32>() {
                Ok(population) => population,
                Err(_) => continue,
            },
            None => continue,
        };

        match feature_class {
            "A" => {
                if population < 490000 {
                    continue;
                }
            }
            "P" => match feature_code {
                "PPLC" => (),
                _ => {
                    if population < 290000 {
                        continue;
                    }
                }
            },
            _ => continue,
        }

        update_region(
            &pool,
            Region {
                region_code: region_code.to_string(),
                keyphrases: ascii_name.to_string(),
            },
        )
        .await?;
    }

    verify_codes().await;
    let region_codes = vec![
        "ad", "ae", "af", "ag", "ai", "al", "am", "ao", "aq", "ar", "as", "at", "au", "aw", "ax",
        "az", "ba", "bb", "bd", "be", "bf", "bg", "bh", "bi", "bj", "bl", "bm", "bn", "bo", "bq",
        "br", "bs", "bt", "bv", "bw", "by", "bz", "ca", "cc", "cd", "cf", "cg", "ch", "ci", "ck",
        "cl", "cm", "cn", "co", "cr", "cu", "cv", "cw", "cx", "cy", "cz", "de", "dj", "dk", "dm",
        "do", "dz", "ec", "ee", "eg", "eh", "er", "es", "et", "fi", "fj", "fk", "fm", "fo", "fr",
        "ga", "gb", "gd", "ge", "gf", "gg", "gh", "gi", "gl", "gm", "gn", "gp", "gq", "gr", "gs",
        "gt", "gu", "gw", "gy", "hk", "hm", "hn", "hr", "ht", "hu", "id", "ie", "il", "im", "in",
        "io", "iq", "ir", "is", "it", "je", "jm", "jo", "jp", "ke", "kg", "kh", "ki", "km", "kn",
        "kp", "kr", "kw", "ky", "kz", "la", "lb", "lc", "li", "lk", "lr", "ls", "lt", "lu", "lv",
        "ly", "ma", "mc", "md", "me", "mf", "mg", "mh", "mk", "ml", "mm", "mn", "mo", "mp", "mq",
        "mr", "ms", "mt", "mu", "mv", "mw", "mx", "my", "mz", "na", "nc", "ne", "nf", "ng", "ni",
        "nl", "no", "np", "nr", "nu", "nz", "om", "pa", "pe", "pf", "pg", "ph", "pk", "pl", "pm",
        "pn", "pr", "ps", "pt", "pw", "py", "qa", "re", "ro", "rs", "ru", "rw", "sa", "sb", "sc",
        "sd", "se", "sg", "sh", "si", "sj", "sk", "sl", "sm", "sn", "so", "sr", "ss", "st", "sv",
        "sx", "sy", "sz", "tc", "td", "tf", "tg", "th", "tj", "tk", "tl", "tm", "tn", "to", "tr",
        "tt", "tv", "tw", "tz", "ua", "ug", "um", "us", "uy", "uz", "va", "vc", "ve", "vg", "vi",
        "vn", "vu", "wf", "ws", "xk", "ye", "yt", "za", "zm", "zw",
    ];
    for region_code in region_codes {
        let heads_of_state = region_code_to_figures(&client, region_code).await?;
        for head_of_state in heads_of_state {
            update_region(
                &pool,
                Region {
                    region_code: region_code.to_string(),
                    keyphrases: head_of_state,
                },
            )
            .await?;
        }
    }

    let largest_billionaires = get_largest_billionaires_map(&client).await?;
    for (calculated_code, billionaires) in largest_billionaires {
        for keyphrase in billionaires {
            update_region(
                &pool,
                Region {
                    region_code: calculated_code.to_string(),
                    keyphrases: keyphrase,
                },
            )
            .await?;
        }
    }

    let largest_private_enterprises = get_private_enterprises_map(&client).await?;
    for (calculated_code, largest_private_enterprises) in largest_private_enterprises {
        for keyphrase in largest_private_enterprises {
            update_region(
                &pool,
                Region {
                    region_code: calculated_code.to_string(),
                    keyphrases: keyphrase,
                },
            )
            .await?;
        }
    }

    Ok(())
}

async fn update_region(
    pool: &sqlx::Pool<sqlx::Sqlite>,
    region: Region,
) -> Result<SqliteQueryResult> {
    let region = Region {
        region_code: region.region_code.to_lowercase(),
        keyphrases: region.keyphrases.to_lowercase(),
    };
    let row: Option<(String,)> =
        sqlx::query_as("SELECT keyphrases FROM regions WHERE region_code = $1")
            .bind(&region.region_code)
            .fetch_optional(pool)
            .await?;

    let region_keyphrases = unidecode(&region.keyphrases);
    match row {
        Some((existing_keyphrases,)) => {
            let mut keyphrases = existing_keyphrases.split(',').collect::<Vec<&str>>();
            keyphrases.extend(region_keyphrases.split(','));
            let keyphrases: HashSet<&str> = keyphrases.into_par_iter().collect();
            let keyphrases = keyphrases.into_par_iter().collect::<Vec<&str>>().join(",");
            Ok(
                sqlx::query("UPDATE regions SET keyphrases = $1 WHERE region_code = $2")
                    .bind(keyphrases)
                    .bind(region.region_code)
                    .execute(pool)
                    .await?,
            )
        }
        None => Ok(
            sqlx::query("INSERT INTO regions (region_code, keyphrases) VALUES ($1, $2)")
                .bind(region.region_code)
                .bind(region_keyphrases)
                .execute(pool)
                .await?,
        ),
    }
}
