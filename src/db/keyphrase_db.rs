use crate::scrape::scrapers::forbes400::get_largest_billionaires_map;
use crate::scrape::scrapers::wikidata::{region_code_to_figures, verify_codes};
use crate::scrape::scrapers::wikipedia::get_private_enterprises_map;
use crate::util::zip_service::{unzip_files_to, zip_from_url};
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use reqwest::Client;
use sqlx::sqlite::{SqliteConnectOptions, SqliteQueryResult};
use sqlx::{Executor, SqlitePool};
use std::{collections::HashSet, io::BufRead, path::Path};
use unidecode::unidecode;
struct Region {
    region_code: String,
    keyphrases: String,
}

pub async fn get_region_db_pool(db_path: &Path) -> Result<SqlitePool, sqlx::Error> {
    Ok(SqlitePool::connect_with(
        SqliteConnectOptions::new()
            .filename(db_path)
            .create_if_missing(true)
            .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
            .shared_cache(true)
            .synchronous(sqlx::sqlite::SqliteSynchronous::Normal),
    )
    .await?)
}

pub async fn gen_keyphrase_db(exe_parent: String) -> Result<(), Box<dyn std::error::Error>> {
    let db_path = format!("{}/region_db.sqlite", exe_parent);
    let db_path = Path::new(&db_path);
    if db_path.exists() {
        tracing::info!("region_db.sqlite found. Skipping keyphrase database generation.");
        return Ok(());
    }

    let client = Client::new();
    let all_countries_path = format!("{}/allCountries.txt", exe_parent);
    let all_countries_path = Path::new(&all_countries_path);
    if all_countries_path.exists() {
        tracing::info!("allCountries.txt found. Skipping download and decompression.");
    } else {
        let zip_path = format!("{}/allCountries.zip", exe_parent);
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
        unzip_files_to(&zip_path, &exe_parent).await?;
    }

    let pool = get_region_db_pool(&db_path).await?;
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
        let mut fields = line.split("\t");
        let ascii_name = match fields.nth(2) {
            Some(ascii_name) => ascii_name,
            None => continue,
        };

        let feature_class = match fields.nth(3) {
            Some(feature_class) => feature_class,
            None => continue,
        };

        let feature_code = match fields.nth(0) {
            Some(feature_code) => feature_code,
            None => continue,
        };

        if feature_code.contains('H') {
            continue;
        }

        let region_code = match fields.nth(0) {
            Some(region_code) => region_code,
            None => continue,
        };

        if region_code == "MZ" && ascii_name.contains("aza") {
            continue;
        } // "Gaza" is a better keyphrase for Palestine.

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
                keyphrases: format!("{}", ascii_name),
            },
        )
        .await?;
    }

    verify_codes().await;
    let region_codes = vec![
        "AD", "AE", "AF", "AG", "AI", "AL", "AM", "AO", "AQ", "AR", "AS", "AT", "AU", "AW", "AX",
        "AZ", "BA", "BB", "BD", "BE", "BF", "BG", "BH", "BI", "BJ", "BL", "BM", "BN", "BO", "BQ",
        "BR", "BS", "BT", "BV", "BW", "BY", "BZ", "CA", "CC", "CD", "CF", "CG", "CH", "CI", "CK",
        "CL", "CM", "CN", "CO", "CR", "CU", "CV", "CW", "CX", "CY", "CZ", "DE", "DJ", "DK", "DM",
        "DO", "DZ", "EC", "EE", "EG", "EH", "ER", "ES", "ET", "FI", "FJ", "FK", "FM", "FO", "FR",
        "GA", "GB", "GD", "GE", "GF", "GG", "GH", "GI", "GL", "GM", "GN", "GP", "GQ", "GR", "GS",
        "GT", "GU", "GW", "GY", "HK", "HM", "HN", "HR", "HT", "HU", "ID", "IE", "IL", "IM", "IN",
        "IO", "IQ", "IR", "IS", "IT", "JE", "JM", "JO", "JP", "KE", "KG", "KH", "KI", "KM", "KN",
        "KP", "KR", "KW", "KY", "KZ", "LA", "LB", "LC", "LI", "LK", "LR", "LS", "LT", "LU", "LV",
        "LY", "MA", "MC", "MD", "ME", "MF", "MG", "MH", "MK", "ML", "MM", "MN", "MO", "MP", "MQ",
        "MR", "MS", "MT", "MU", "MV", "MW", "MX", "MY", "MZ", "NA", "NC", "NE", "NF", "NG", "NI",
        "NL", "NO", "NP", "NR", "NU", "NZ", "OM", "PA", "PE", "PF", "PG", "PH", "PK", "PL", "PM",
        "PN", "PR", "PS", "PT", "PW", "PY", "QA", "RE", "RO", "RS", "RU", "RW", "SA", "SB", "SC",
        "SD", "SE", "SG", "SH", "SI", "SJ", "SK", "SL", "SM", "SN", "SO", "SR", "SS", "ST", "SV",
        "SX", "SY", "SZ", "TC", "TD", "TF", "TG", "TH", "TJ", "TK", "TL", "TM", "TN", "TO", "TR",
        "TT", "TV", "TW", "TZ", "UA", "UG", "UM", "US", "UY", "UZ", "VA", "VC", "VE", "VG", "VI",
        "VN", "VU", "WF", "WS", "YE", "YT", "ZA", "ZM", "ZW",
    ];
    for region_code in &region_codes {
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
) -> Result<SqliteQueryResult, sqlx::Error> {
    let row: Option<(String,)> =
        sqlx::query_as("SELECT keyphrases FROM regions WHERE region_code = $1")
            .bind(&region.region_code)
            .fetch_optional(pool)
            .await?;

    let region_keyphrases = unidecode(&region.keyphrases.to_lowercase());
    match row {
        Some((existing_keyphrases,)) => {
            let mut keyphrases = existing_keyphrases.split(",").collect::<Vec<&str>>();
            keyphrases.extend(region_keyphrases.split(","));
            let keyphrases: HashSet<&str> = keyphrases.into_par_iter().collect();
            let keyphrases = keyphrases.into_par_iter().collect::<Vec<&str>>().join(",");
            sqlx::query("UPDATE regions SET keyphrases = $1 WHERE region_code = $2")
                .bind(keyphrases)
                .bind(region.region_code)
                .execute(pool)
                .await
        }
        None => {
            sqlx::query("INSERT INTO regions (region_code, keyphrases) VALUES ($1, $2)")
                .bind(region.region_code)
                .bind(region_keyphrases)
                .execute(pool)
                .await
        }
    }
}
