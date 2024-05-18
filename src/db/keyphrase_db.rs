use rayon::iter::{IntoParallelIterator, ParallelIterator};
use sqlx::sqlite::{SqliteConnectOptions, SqliteQueryResult};
use sqlx::{Executor, SqlitePool};
use std::collections::HashSet;
use std::fs;
use std::path::Path;
use std::{fs::File, io, io::BufRead};
use unidecode::unidecode;
use crate::scrape::scrapers::wikidata::region_code_to_figures;


struct Region {
    region_code: String,
    keyphrases: String,
}

pub async fn get_region_db_pool() -> Result<SqlitePool, sqlx::Error> {
    Ok(SqlitePool::connect_with(SqliteConnectOptions::new()
        .filename("region_db.sqlite")
        .create_if_missing(true)
        .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
        .shared_cache(true)
        .synchronous(sqlx::sqlite::SqliteSynchronous::Normal)).await?)
}

pub async fn gen_keyphrase_db() -> Result<(), Box<dyn std::error::Error>> {
    let current_dir = std::env::current_dir()?;
    let db_path = format!("{}/region_db.sqlite", current_dir.display());
    if Path::new(&db_path).exists() {
        tracing::info!("region_db.sqlite found. Skipping keyphrase database generation.");
        return Ok(())
    }
    
    for entry in fs::read_dir(&current_dir)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_file() { continue; }

        if let Some(filename) = path.file_name() {
            if let Some(filename) = filename.to_str() {
                if filename.starts_with("region_db") { fs::remove_file(path)?; }
            }
        }
    }

    let pool = get_region_db_pool().await?;
    pool.execute(
        "CREATE TABLE IF NOT EXISTS regions (
            region_code TEXT PRIMARY KEY,
            keyphrases TEXT
        )"
    ).await?;
    
    let all_countries_path = format!("{}/src/db/allCountries.txt", current_dir.display()); // https://download.geonames.org/export/dump/allCountries.zip
    let reader = io::BufReader::new(File::open(Path::new(&all_countries_path))?);
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
        let region_code = match fields.nth(0) {
            Some(region_code) => region_code,
            None => continue,
        };
        let population = match fields.nth(5) {
            Some(population) => {
                match population.parse::<u32>() {
                    Ok(population) => population,
                    Err(_) => continue,
                }
            },
            None => continue,
        };

        if feature_code.contains('H') { continue; }

        match feature_class {
            "A" =>  {
                if population < 490000 { continue; }
            },
            "P" => {
                match feature_code {
                    "PPLC" => {},
                    _ => {
                        if population < 290000 { continue; }
                    },
                }
            },
            _ => continue,
        }

        update_region(&pool, Region {
            region_code: region_code.to_string(),
            keyphrases: format!("{}", ascii_name),
        }).await?;
    }

    let country_info_path = format!("{}/src/db/countryInfo.txt", current_dir.display()); // https://download.geonames.org/export/dump/countryInfo.txt
    let reader = io::BufReader::new(File::open(Path::new(&country_info_path))?);
    for line in reader.lines() {
        let line = line?;
        let mut fields = line.split("\t");
        let region_code = match fields.nth(0) {
            Some(region_code) => region_code,
            None => continue,
        };
        let capital = match fields.nth(4) {
            Some(capital) => capital,
            None => continue,
        };

        update_region(&pool, Region {
            region_code: region_code.to_string(),
            keyphrases: format!("{}", capital),
        }).await?;
    }

    // Wikidata property verifying is only necessary when running for the first time or when it's been a long time since the previous verification.
    //crate::scrape::scrapers::wikidata::verify_codes().await;
    let region_codes = vec!["AD", "AE", "AF", "AG", "AI", "AL", "AM", "AO", "AQ", "AR", "AS", "AT", "AU", "AW", "AX", "AZ", "BA", "BB", "BD", "BE", "BF", "BG", "BH", "BI", "BJ", "BL", "BM", "BN", "BO", "BQ", "BR", "BS", "BT", "BV", "BW", "BY", "BZ", "CA", "CC", "CD", "CF", "CG", "CH", "CI", "CK", "CL", "CM", "CN", "CO", "CR", "CU", "CV", "CW", "CX", "CY", "CZ", "DE", "DJ", "DK", "DM", "DO", "DZ", "EC", "EE", "EG", "EH", "ER", "ES", "ET", "FI", "FJ", "FK", "FM", "FO", "FR", "GA", "GB", "GD", "GE", "GF", "GG", "GH", "GI", "GL", "GM", "GN", "GP", "GQ", "GR", "GS", "GT", "GU", "GW", "GY", "HK", "HM", "HN", "HR", "HT", "HU", "ID", "IE", "IL", "IM", "IN", "IO", "IQ", "IR", "IS", "IT", "JE", "JM", "JO", "JP", "KE", "KG", "KH", "KI", "KM", "KN", "KP", "KR", "KW", "KY", "KZ", "LA", "LB", "LC", "LI", "LK", "LR", "LS", "LT", "LU", "LV", "LY", "MA", "MC", "MD", "ME", "MF", "MG", "MH", "MK", "ML", "MM", "MN", "MO", "MP", "MQ", "MR", "MS", "MT", "MU", "MV", "MW", "MX", "MY", "MZ", "NA", "NC", "NE", "NF", "NG", "NI", "NL", "NO", "NP", "NR", "NU", "NZ", "OM", "PA", "PE", "PF", "PG", "PH", "PK", "PL", "PM", "PN", "PR", "PS", "PT", "PW", "PY", "QA", "RE", "RO", "RS", "RU", "RW", "SA", "SB", "SC", "SD", "SE", "SG", "SH", "SI", "SJ", "SK", "SL", "SM", "SN", "SO", "SR", "SS", "ST", "SV", "SX", "SY", "SZ", "TC", "TD", "TF", "TG", "TH", "TJ", "TK", "TL", "TM", "TN", "TO", "TR", "TT", "TV", "TW", "TZ", "UA", "UG", "UM", "US", "UY", "UZ", "VA", "VC", "VE", "VG", "VI", "VN", "VU", "WF", "WS", "YE", "YT", "ZA", "ZM", "ZW"];
    let client = reqwest::Client::new();
    for region_code in region_codes {
        let heads_of_state = region_code_to_figures(&client, region_code).await?;
        for head_of_state in heads_of_state {
            update_region(&pool, Region {
                region_code: region_code.to_string(),
                keyphrases: head_of_state,
            }).await?;
        }
    }

    Ok(())
}

async fn update_region(pool: &sqlx::Pool<sqlx::Sqlite>, region: Region) -> Result<SqliteQueryResult, sqlx::Error> {
    let row: Option<(String,)> = sqlx::query_as("SELECT keyphrases FROM regions WHERE region_code = $1")
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
        },
        None => {
            sqlx::query("INSERT INTO regions (region_code, keyphrases) VALUES ($1, $2)")
                .bind(region.region_code)
                .bind(region_keyphrases)
                .execute(pool)
                .await
        },
    }
}