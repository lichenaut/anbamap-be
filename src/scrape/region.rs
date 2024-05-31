use crate::prelude::*;
use crate::{db::keyphrase::get_region_db_pool, service::var_service::get_docker_volume};
use async_std::task;
use once_cell::sync::Lazy;
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use sqlx::Row;
use std::{
    collections::{HashMap, HashSet},
    io::stdin,
    path::Path,
    vec,
};

struct RegionKeyphrases {
    pub automated: Option<Vec<String>>,         // src/db/keyphrase.rs
    pub names: Option<Vec<&'static str>>,       // Manual
    pub demonyms: Option<Vec<&'static str>>,    // Manual
    pub enterprises: Option<Vec<&'static str>>, // Manual: https://companiesmarketcap.com/all-countries/
    pub misc: Option<Vec<&'static str>>,        // Manual
}

impl RegionKeyphrases {
    pub fn get_region_vec(self) -> Vec<&'static str> {
        let mut region_vec: Vec<&'static str> = Vec::new();
        // First-order administrative regions ≥ 490k population, capitals, cities ≥ 290k population...
        // ...heads of state and government, largest private enterprises, and billionaires ≥ 9.9B final worth USD.z
        if let Some(automated) = self.automated {
            for s in automated {
                region_vec.push(Box::leak(s.into_boxed_str()));
            }
        }
        if let Some(names) = self.names {
            region_vec.extend(names);
        }
        if let Some(demonyms) = self.demonyms {
            region_vec.extend(demonyms);
        }
        // Public enterprises ≥ 9.9B market cap USD.
        if let Some(enterprises) = self.enterprises {
            region_vec.extend(enterprises);
        }
        // Positions of power, legislative bodies, institutions, buildings, political groups, ideologies, ethnic groups, cultural regions, identifier names, etc.
        if let Some(misc) = self.misc {
            region_vec.extend(misc);
        }

        region_vec.retain(|s| !s.is_empty());
        region_vec.sort_by_key(|a| a.len());
        let mut i = 0;
        while i < region_vec.len() {
            let mut j = i + 1;
            while j < region_vec.len() {
                if region_vec[j] == region_vec[i] {
                    j += 1;
                    continue;
                }

                if region_vec[j].contains(region_vec[i]) {
                    //tracing::debug!("Removing region-level substring-containing string {} because of substring {}", region_vec[j], region_vec[i]);
                    region_vec.remove(j);
                } else {
                    j += 1;
                }
            }

            i += 1;
        }

        let mut short_strings: Vec<&'static str> = Vec::new();
        region_vec.iter_mut().for_each(|s| {
            if s.len() < 4 {
                short_strings.push(Box::leak(format!("'{}'", s).into_boxed_str()));
                short_strings.push(Box::leak(format!("\"{}\"", s).into_boxed_str()));
                short_strings.push(Box::leak(format!("{}.", s).into_boxed_str()));
                short_strings.push(Box::leak(format!("{},", s).into_boxed_str()));
                *s = Box::leak(format!(" {} ", s).into_boxed_str());
            }
        });
        region_vec.extend(short_strings);

        // " inc" is a catch-all for other types here, where I include this string when the enterprise name is ambiguous (ex. 'apple' -> 'apple inc').
        // Enterprise type changes do not have to be tracked this way.
        let mut enterprise_types: Vec<&'static str> = Vec::new();
        region_vec.iter().for_each(|s| {
            let Some(stripped) = s.strip_suffix(" inc") else {
                return;
            };

            enterprise_types.push(Box::leak(format!("{}, inc", stripped).into_boxed_str()));
            enterprise_types.push(Box::leak(format!("{} ltd", stripped).into_boxed_str()));
            enterprise_types.push(Box::leak(format!("{}, ltd", stripped).into_boxed_str()));
            enterprise_types.push(Box::leak(format!("{} limited", stripped).into_boxed_str()));
            enterprise_types.push(Box::leak(format!("{}, limited", stripped).into_boxed_str()));
            enterprise_types.push(Box::leak(format!("{} plc", stripped).into_boxed_str()));
            enterprise_types.push(Box::leak(format!("{}, plc", stripped).into_boxed_str()));
            enterprise_types.push(Box::leak(format!("{} llc", stripped).into_boxed_str()));
            enterprise_types.push(Box::leak(format!("{}, llc", stripped).into_boxed_str()));
        });
        region_vec.extend(enterprise_types.iter().cloned());

        region_vec
    }
}

async fn build_region_map() -> Result<HashMap<String, Vec<String>>> {
    let db_path = format!("{}/region_db.sqlite", get_docker_volume().await?);
    let db_path = Path::new(&db_path);
    let pool = get_region_db_pool(db_path).await?;
    let mut region_map = HashMap::new();
    let rows = sqlx::query("SELECT * FROM regions")
        .fetch_all(&pool)
        .await?;
    for row in &rows {
        region_map.insert(row.get(0), vec![row.get(1)]);
    }

    Ok(region_map)
}

fn get_automated_keyphrases(
    region_map: &HashMap<String, Vec<String>>,
    region_code: &str,
) -> Option<Vec<String>> {
    region_map.get(region_code).map(|g| {
        g.iter()
            .flat_map(|s| s.split(',').map(|s| s.trim().to_string()))
            .collect::<Vec<_>>()
    })
}

fn remove_ambiguities(
    vec: Vec<(Vec<&'static str>, &'static str)>,
    blacklist: HashSet<&'static str>,
) -> Vec<(Vec<&'static str>, &'static str)> {
    // let mut map = HashMap::new();
    // for (key, _) in &vec {
    //     for s in key {
    //         let count = map.entry(s.clone()).or_insert(0);
    //         *count += 1;
    //         if *count > 1 {
    //             tracing::debug!("Duplicate map-level keyphrase: {}", s);
    //         }
    //     }
    // }

    let vec: Vec<(Vec<&'static str>, &'static str)> = vec
        .into_par_iter()
        .map(|(keys, value)| {
            // Removes duplicate strings.
            let unique_keys: Vec<&'static str> = keys
                .clone()
                .into_par_iter()
                .collect::<HashSet<_>>()
                .into_par_iter()
                .collect();
            (unique_keys, value)
        })
        .collect();
    let mut all_strings: Vec<&'static str> = vec
        .clone()
        .into_par_iter()
        .flat_map(|(keys, _)| keys.clone())
        .collect();
    let all_strings_copy = all_strings.clone();
    let mut to_remove = blacklist;

    for string in &all_strings_copy {
        if to_remove.contains(string) {
            continue;
        }

        for other_string in &all_strings_copy {
            if string != other_string && string.contains(other_string) {
                // Removes substrings.
                //tracing::debug!("Removing map-level substring: {} because of {}", other_string, string);
                to_remove.insert(other_string);
            }
        }
    }

    all_strings.retain(|string| !to_remove.contains(string));

    vec.into_par_iter()
        .map(|(keys, value)| {
            let new_keys = keys
                .into_par_iter()
                .filter(|key| all_strings.contains(key))
                .collect();
            (new_keys, value)
        })
        .collect()
}

pub static KEYPHRASE_REGION_MAP: Lazy<Vec<(Vec<&'static str>, &'static str)>> = Lazy::new(|| {
    // Please contribute on https://github.com/lichenaut/anbamap-api !
    let region_map = task::block_on(build_region_map());
    let region_map = match region_map {
        Ok(map) => map,
        Err(e) => {
            tracing::error!("Failed to build region map: {:?}", e);
            return Vec::new();
        }
    };

    let map = vec![
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "AD"),
                names: Some(vec!["andorra"]),
                demonyms: None,
                enterprises: None,
                misc: Some(vec!["general syndic", "council of the valleys"]),
            }
            .get_region_vec(),
            "Andorra",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "AE"),
                names: Some(vec!["united arab emirates", "uae"]),
                demonyms: Some(vec!["emirati"]),
                enterprises: Some(vec![
                    "international holding co",
                    "taqa",
                    "adnoc",
                    "emirates telecom",
                    "alpha dhabi",
                    "invest bank",
                    "dewa",
                    "emirates nbd",
                    "emirates pjsc",
                    "borouge",
                    "emaar properties",
                    "q holding",
                    "al dar properties",
                    "pure health holding",
                    "mashreqbank",
                ]),
                misc: None,
            }
            .get_region_vec(),
            "United Arab Emirates",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "AF"),
                names: None,
                demonyms: Some(vec!["afghan"]),
                enterprises: None,
                misc: Some(vec!["taliban"]),
            }
            .get_region_vec(),
            "Afghanistan",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "AG"),
                names: Some(vec!["antigua", "barbuda", "a&b"]),
                demonyms: None,
                enterprises: None,
                misc: Some(vec!["ablp", "united progressive party"]),
            }
            .get_region_vec(),
            "Antigua and Barbuda",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "AI"),
                names: Some(vec!["anguilla"]),
                demonyms: None,
                enterprises: None,
                misc: None,
            }
            .get_region_vec(),
            "Anguilla",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "AL"),
                names: Some(vec!["albania"]),
                demonyms: None,
                enterprises: None,
                misc: Some(vec!["kuvendi"]),
            }
            .get_region_vec(),
            "Albania",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "AM"),
                names: Some(vec![
                    "armenia ",
                    "armenia'",
                    "armenia\"",
                    "armenia.",
                    "armenia,",
                ]),
                demonyms: None,
                enterprises: None,
                misc: Some(vec!["azgayin zhoghov"]),
            }
            .get_region_vec(),
            "Armenia",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "AO"),
                names: Some(vec!["angola"]),
                demonyms: None,
                enterprises: None,
                misc: Some(vec!["mpla", "unita"]),
            }
            .get_region_vec(),
            "Angola",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "AQ"),
                names: Some(vec!["antarctica"]),
                demonyms: None,
                enterprises: None,
                misc: Some(vec!["mcmurdo"]),
            }
            .get_region_vec(),
            "Antarctica",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "AR"),
                names: None,
                demonyms: Some(vec!["argentin"]),
                enterprises: Some(vec!["mercadolibre", "ypf", "yacimientos petroliferos"]),
                misc: Some(vec![
                    "casa rosada",
                    "union for the homeland",
                    "juntos por el cambio",
                    "cambiemos",
                    "peronis",
                    "kirchneris",
                ]),
            }
            .get_region_vec(),
            "Argentina",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "AS"),
                names: Some(vec!["american samoa"]),
                demonyms: None,
                enterprises: None,
                misc: None,
            }
            .get_region_vec(),
            "American Samoa",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "AT"),
                names: Some(vec!["austria", "oesterreich"]),
                demonyms: None,
                enterprises: Some(vec!["verbund", "erste group", "erste bank", "omv"]),
                misc: None,
            }
            .get_region_vec(),
            "Austria",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "AU"),
                names: Some(vec!["australia"]),
                demonyms: Some(vec!["aussie"]),
                enterprises: Some(vec![
                    "bhp group",
                    "commonwealth bank",
                    "csl",
                    "westpac bank",
                    "anz bank",
                    "fortescue",
                    "wesfarmers",
                    "macquarie",
                    "atlassian",
                    "goodman group",
                    "woodside",
                    "telstra",
                    "transurban",
                    "woolworths",
                    "wisetech",
                    "qbe",
                    "santos inc",
                    "aristocrat inc",
                    "rea",
                    "coles group",
                    "cochlear",
                    "suncorp",
                    "brambles limited",
                    "reece group",
                    "origin energy",
                    "northern star inc",
                    "scentre group",
                    "south32",
                    "computershare",
                    "mineral resources inc",
                    "seven group",
                    "sgh",
                ]),
                misc: Some(vec!["aborigin", "assange"]),
            }
            .get_region_vec(),
            "Australia",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "AW"),
                names: Some(vec!["aruba"]),
                demonyms: None,
                enterprises: None,
                misc: None,
            }
            .get_region_vec(),
            "Aruba",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "AX"),
                names: Some(vec!["aland"]),
                demonyms: None,
                enterprises: None,
                misc: None,
            }
            .get_region_vec(),
            "Aland Islands",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "AZ"),
                names: Some(vec!["azerbaijan"]),
                demonyms: Some(vec!["azeri"]),
                enterprises: None,
                misc: Some(vec!["milli majlis", "democratic reforms party"]),
            }
            .get_region_vec(),
            "Azerbaijan",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "BA"),
                names: Some(vec!["bosnia", "srpska", "brcko"]),
                demonyms: Some(vec!["herzegovin"]),
                enterprises: None,
                misc: Some(vec![
                    "alliance of independent social democrats",
                    "party of democratic action",
                ]),
            }
            .get_region_vec(),
            "Bosnia and Herzegovina",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "BB"),
                names: Some(vec!["barbados"]),
                demonyms: None,
                enterprises: None,
                misc: None,
            }
            .get_region_vec(),
            "Barbados",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "BD"),
                names: Some(vec!["bangladesh"]),
                demonyms: None,
                enterprises: None,
                misc: Some(vec![
                    "jatiya sangsad",
                    "awami league",
                    "jatiya party",
                    "bengal",
                ]),
            }
            .get_region_vec(),
            "Bangladesh",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "BE"),
                names: Some(vec!["belgium"]),
                demonyms: Some(vec!["belgian"]),
                enterprises: Some(vec!["anheuser-busch", "kbc", "ucb", "d'leteren", "gbl"]),
                misc: Some(vec!["flemish", "walloon"]),
            }
            .get_region_vec(),
            "Belgium",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "BF"),
                names: Some(vec!["burkina faso"]),
                demonyms: Some(vec!["burkinabe", "burkinese"]),
                enterprises: None,
                misc: Some(vec!["mpsr"]),
            }
            .get_region_vec(),
            "Burkina Faso",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "BG"),
                names: Some(vec!["bulgaria"]),
                demonyms: None,
                enterprises: None,
                misc: Some(vec!["narodno sabranie", "gerb"]),
            }
            .get_region_vec(),
            "Bulgaria",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "BH"),
                names: Some(vec!["bahrain"]),
                demonyms: None,
                enterprises: Some(vec!["ahli united", "ahli bank"]),
                misc: Some(vec![
                    "shura council",
                    "asalah",
                    "progressive democratic tribune",
                    "bchr",
                ]),
            }
            .get_region_vec(),
            "Bahrain",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "BI"),
                names: Some(vec!["burundi"]),
                demonyms: None,
                enterprises: None,
                misc: Some(vec![
                    "cndd",
                    "national congress for liberty",
                    "national congress for freedom",
                ]),
            }
            .get_region_vec(),
            "Burundi",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "BJ"),
                names: Some(vec!["benin"]),
                demonyms: None,
                enterprises: None,
                misc: Some(vec!["progressive union for renewal"]),
            }
            .get_region_vec(),
            "Benin",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "BL"),
                names: Some(vec!["saint barthelemy"]),
                demonyms: Some(vec!["barthelemois"]),
                enterprises: None,
                misc: None,
            }
            .get_region_vec(),
            "Saint Barthelemy",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "BM"),
                names: Some(vec!["bermuda"]),
                demonyms: None,
                enterprises: None,
                misc: None,
            }
            .get_region_vec(),
            "Bermuda",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "BN"),
                names: None, // Name comes from database.
                demonyms: None,
                enterprises: None,
                misc: None,
            }
            .get_region_vec(),
            "Brunei",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "BO"),
                names: Some(vec!["bolivia"]),
                demonyms: None,
                enterprises: None,
                misc: Some(vec!["pluritonal", "plaza murillo"]),
            }
            .get_region_vec(),
            "Bolivia",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "BQ"),
                names: Some(vec![
                    "bonaire",
                    "sint eustatius",
                    "saba",
                    "statia",
                    "bes island",
                ]),
                demonyms: None,
                enterprises: None,
                misc: None,
            }
            .get_region_vec(),
            "Bonaire, Sint Eustatius, and Saba",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "BR"),
                names: Some(vec!["brazil", "brasil"]),
                demonyms: None,
                enterprises: Some(vec![
                    "petrobras",
                    "itau unibanco",
                    "nu holding",
                    "vale inc",
                    "ambev",
                    "btg pactual",
                    "weg on",
                    "bradesco",
                    "klabin",
                    "itausa",
                    "rede d'or sao luiz",
                    "bb seguridade",
                    "seguridade participacoes",
                    "suzano",
                    "jbs",
                    "b3",
                    "xp inc",
                    "sabesp",
                    "localiza",
                ]),
                misc: Some(vec!["planalto", "lula"]),
            }
            .get_region_vec(),
            "Brazil",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "BS"),
                names: Some(vec!["bahama"]),
                demonyms: Some(vec!["bahamian"]),
                enterprises: None,
                misc: Some(vec!["progressive liberal party", "free national movement"]),
            }
            .get_region_vec(),
            "The Bahamas",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "BT"),
                names: Some(vec!["bhutan"]),
                demonyms: None,
                enterprises: None,
                misc: Some(vec!["druk gyalpo"]),
            }
            .get_region_vec(),
            "Bhutan",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "BV"),
                names: Some(vec!["bouvet"]),
                demonyms: None,
                enterprises: None,
                misc: None,
            }
            .get_region_vec(),
            "Bouvet Island",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "BW"),
                names: Some(vec!["botswana"]),
                demonyms: Some(vec!["batswana", "motswana"]),
                enterprises: None,
                misc: Some(vec!["umbrella for democratic change"]),
            }
            .get_region_vec(),
            "Botswana",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "BY"),
                names: Some(vec!["belarus"]),
                demonyms: None,
                enterprises: None,
                misc: Some(vec!["belaya rus", "ldpb"]),
            }
            .get_region_vec(),
            "Belarus",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "BZ"),
                names: Some(vec!["belize"]),
                demonyms: None,
                enterprises: None,
                misc: Some(vec!["people's united party"]),
            }
            .get_region_vec(),
            "Belize",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "CA"),
                names: None, // Name comes from database.
                demonyms: Some(vec!["canadian"]),
                enterprises: Some(vec![
                    "enbridge",
                    "reuters",
                    "shopify",
                    "brookfield",
                    "scotiabank",
                    "constellation software",
                    "alimentation",
                    "couche-tard",
                    "suncor energy",
                    "manulife",
                    "cibc",
                    "lululemon",
                    "tc energy",
                    "cenovus",
                    "imperial oil inc",
                    "loblaw",
                    "agnico eagle",
                    "restaurant brands international",
                    "barrick gold",
                    "bce inc",
                    "sun life financial",
                    "intact financial inc",
                    "great-west lifeco",
                    "nutrien inc",
                    "teck resources",
                    "fairfax",
                    "wheaton precious",
                    "wheaton metals",
                    "dollarama",
                    "franco-nevada",
                    "telus",
                    "cgi inc",
                    "cameco",
                    "rogers comm",
                    "pembina",
                    "fortis",
                    "ivanhoe",
                    "wsp global",
                    "george weston",
                    "hydro one",
                    "tourmaline oil",
                    "ritchie bros",
                    "magna international",
                    "power financial inc",
                    "metro inc",
                    "gfl",
                    "first quantum minerals",
                    "arc resources",
                    "tfi international",
                    "emera",
                    "lundin mining",
                ]),
                misc: Some(vec![
                    "parliament hill",
                    "rcmp",
                    "ndp",
                    "quebecois",
                    "metis",
                    "first nations",
                    "trudeau",
                ]),
            }
            .get_region_vec(),
            "Canada",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "CC"),
                names: Some(vec!["cocos island", "keeling island"]),
                demonyms: None,
                enterprises: None,
                misc: None,
            }
            .get_region_vec(),
            "Cocos (Keeling) Islands",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "CD"),
                names: Some(vec!["democratic republic of the congo", "drc", "big congo"]),
                demonyms: None,
                enterprises: None,
                misc: Some(vec![
                    "udps",
                    "common front for congo",
                    "kabila coalition",
                    "lamuka",
                    "fardc",
                    "monusco",
                ]),
            }
            .get_region_vec(),
            "Democratic Republic of the Congo",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "CF"),
                names: None,
                demonyms: Some(vec!["central african"]),
                enterprises: None,
                misc: Some(vec![
                    "united hearts movement",
                    "kwa na kwa",
                    "fprc",
                    "anti-balaka",
                ]),
            }
            .get_region_vec(),
            "Central African Republic",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "CG"),
                names: Some(vec!["little congo"]),
                demonyms: None,
                enterprises: None,
                misc: Some(vec!["congolese party of labour", "upads"]),
            }
            .get_region_vec(),
            "Republic of the Congo",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "CH"),
                names: Some(vec!["switzerland"]),
                demonyms: Some(vec!["swiss"]),
                enterprises: Some(vec![
                    "nestle",
                    "roche",
                    "novartis",
                    "chubb inc",
                    "ubs",
                    "abb",
                    "richemont",
                    "glencore",
                    "zurich insurance",
                    "sika",
                    "holcim",
                    "te connectivity",
                    "alcon",
                    "givaudan",
                    "lonza",
                    "stmicroelectronics",
                    "partners group",
                    "swiss re",
                    "garmin",
                    "kuhne + nagel",
                    "dsm-firmenich",
                    "schindler group",
                    "lindt",
                    "straumann",
                    "geberit",
                    "ems-chemie",
                    "sonova",
                    "sgs",
                    "vat group",
                    "sandoz",
                    "amcor",
                    "logitech",
                    "julius bar",
                    "on holding inc",
                    "swatch",
                ]),
                misc: None,
            }
            .get_region_vec(),
            "Switzerland",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "CI"),
                names: Some(vec!["ivory coast", "cote d'ivoire"]),
                demonyms: Some(vec!["ivorian"]),
                enterprises: None,
                misc: None,
            }
            .get_region_vec(),
            "Ivory Coast",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "CK"),
                names: Some(vec!["cook island"]),
                demonyms: None,
                enterprises: None,
                misc: None,
            }
            .get_region_vec(),
            "Cook Islands",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "CL"),
                names: Some(vec!["chile"]),
                demonyms: None,
                enterprises: Some(vec!["quimica y minera", "enel americas", "empresas copec"]),
                misc: None,
            }
            .get_region_vec(),
            "Chile",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "CM"),
                names: Some(vec!["cameroon"]),
                demonyms: None,
                enterprises: None,
                misc: Some(vec!["unity palace", "rdpc", "ambazonia"]),
            }
            .get_region_vec(),
            "Cameroon",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "CN"),
                names: Some(vec!["china", "prc"]),
                demonyms: Some(vec!["chinese"]),
                enterprises: Some(vec![
                    "tencent",
                    "kweichow moutai",
                    "icbc",
                    "alibaba",
                    "pinduoduo",
                    "cm bank",
                    "catl inc",
                    "cnooc",
                    "ping an",
                    "shenhua energy",
                    "sinopec",
                    "meituan",
                    "byd",
                    "foxconn industrial",
                    "foxconn internet",
                    "netease",
                    "zijin mining",
                    "nongfu spring",
                    "midea inc",
                    "xiaomi",
                    "jingdong mall",
                    "mindray",
                    "industrial bank inc",
                    "citic",
                    "hikvision",
                    "jiangsu hengrui",
                    "haier smart home",
                    "haier home",
                    "wanhua chem",
                    "baidu",
                    "luzhou laojiao",
                    "trip.com",
                    "muyuan foods",
                    "pudong",
                    "gree electric",
                    "gree appliances",
                    "anta sports",
                    "kuaishou tech",
                    "luxshare",
                    "the people's insurance co",
                    "picc",
                    "cosco shipping",
                    "east money information",
                    "great wall motors",
                    "crrc",
                    "s.f. express",
                    "sf express",
                    "li auto",
                    "yili group",
                    "smic",
                    "ke holding",
                    "saic motor",
                    "didi",
                    "boe tech",
                    "minsheng bank",
                    "yankuang energy",
                    "yanzhou coal",
                    "yanzhou mining",
                    "bank of jiangsu",
                    "sungrow power",
                    "yanghe",
                    "zto",
                    "weichai",
                    "sany heavy industry",
                    "sany industry",
                    "beigene",
                    "longi ",
                    "seres group",
                    "anhui conch",
                    "zte",
                    "shandong gold",
                    "shandong mining",
                    "huaneng",
                    "aier eye",
                    "aier hospital",
                    "huatai securities",
                    "guotai junan",
                    "longyuan power",
                    "hua xia",
                    "hai di lao",
                    "shekou industrial",
                    "hansoh pharma",
                    "tsingtao",
                    "new oriental inc",
                    "longfor group",
                    "geely",
                    "huazhu hotels",
                    "jd health",
                    "vanke",
                    "avinex",
                    "nio",
                    "amec",
                    "enn",
                    "eve energy",
                    "zheshang bank",
                    "gac",
                ]),
                misc: Some(vec![
                    "national people's congress",
                    "cppcc",
                    "kuomintang",
                    "guomindang",
                    "yangtze",
                    "xi",
                ]),
            }
            .get_region_vec(),
            "China",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "CO"),
                names: Some(vec!["colombia"]),
                demonyms: None,
                enterprises: None,
                misc: Some(vec!["casa de narino", "capitolio nacional", "eln"]),
            }
            .get_region_vec(),
            "Colombia",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "CR"),
                names: Some(vec!["costa rica"]),
                demonyms: None,
                enterprises: None,
                misc: Some(vec![
                    "inter-american court of human rights",
                    "social democratic progress party",
                    "national liberation party",
                    "verdiblancos",
                ]),
            }
            .get_region_vec(),
            "Costa Rica",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "CU"),
                names: Some(vec!["cuba"]),
                demonyms: None,
                enterprises: None,
                misc: Some(vec!["national assembly of people's power"]),
            }
            .get_region_vec(),
            "Cuba",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "CV"),
                names: Some(vec!["cape verde"]),
                demonyms: Some(vec!["cabo verdean"]),
                enterprises: None,
                misc: Some(vec!["paicv"]),
            }
            .get_region_vec(),
            "Cape Verde",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "CW"),
                names: Some(vec!["curacao"]),
                demonyms: None,
                enterprises: None,
                misc: Some(vec!["mfk", "real alternative party"]),
            }
            .get_region_vec(),
            "Curacao",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "CX"),
                names: Some(vec!["christmas island"]),
                demonyms: None,
                enterprises: None,
                misc: None,
            }
            .get_region_vec(),
            "Christmas Island",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "CY"),
                names: Some(vec!["cyprus"]),
                demonyms: Some(vec!["cypriot"]),
                enterprises: None,
                misc: Some(vec!["akel"]),
            }
            .get_region_vec(),
            "Cyprus",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "CZ"),
                names: None,
                demonyms: Some(vec!["czech"]),
                enterprises: Some(vec!["cez"]),
                misc: Some(vec!["spolu", "ano 2011"]),
            }
            .get_region_vec(),
            "Czech Republic",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "DE"),
                names: None,
                demonyms: Some(vec!["german", "deutsche"]),
                enterprises: Some(vec![
                    "sap inc",
                    "siemens",
                    "allianz",
                    "porsche",
                    "mercedes-benz",
                    "merck kgaa",
                    "volkswagen",
                    "munchener ruck",
                    "bmw",
                    "infineon",
                    "dhl",
                    "basf",
                    "adidas",
                    "e.on",
                    "beiersdorf",
                    "henkel",
                    "daimler",
                    "hapag-lloyd",
                    "bayer",
                    "hannover ruck",
                    "rwe",
                    "vonovia",
                    "rheinmetall",
                    "uniper inc",
                    "biontech",
                    "talanx",
                    "commerzbank",
                    "enbw energ",
                    "heidelberg",
                    "sartorius",
                    "traton",
                    "fresenius",
                    "symrise",
                    "continental inc",
                    "mtu aero",
                    "mtu engines",
                    "fresenius",
                    "knorr-bremse",
                    "brenntag",
                    "nemetschek",
                    "hella inc",
                    "evonik",
                ]),
                misc: Some(vec!["bundestag", "cdu", "scholz"]),
            }
            .get_region_vec(),
            "Germany",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "DJ"),
                names: Some(vec!["djibouti"]),
                demonyms: None,
                enterprises: None,
                misc: Some(vec!["union for the presidential majority"]),
            }
            .get_region_vec(),
            "Djibouti",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "DK"),
                names: Some(vec!["denmark"]),
                demonyms: Some(vec!["danish", "dane"]),
                enterprises: Some(vec![
                    "novo nordisk",
                    "dsv",
                    "novozymes",
                    "vestas wind",
                    "vestas systems",
                    "coloplast",
                    "orsted",
                    "maersk",
                    "danske bank",
                    "carlsberg",
                    "genmab",
                    "pandora inc",
                    "tryg",
                    "demant",
                ]),
                misc: Some(vec!["folketing"]),
            }
            .get_region_vec(),
            "Denmark",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "DM"),
                names: Some(vec![
                    "dominica ",
                    "dominica'",
                    "dominica\"",
                    "dominica.",
                    "dominica,",
                ]),
                demonyms: None,
                enterprises: None,
                misc: None,
            }
            .get_region_vec(),
            "Dominica",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "DO"),
                names: Some(vec!["dominican republic"]),
                demonyms: None,
                enterprises: None,
                misc: None,
            }
            .get_region_vec(),
            "Dominican Republic",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "DZ"),
                names: Some(vec!["algeria"]),
                demonyms: None,
                enterprises: None,
                misc: Some(vec!["algerie", "fln"]),
            }
            .get_region_vec(),
            "Algeria",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "EC"),
                names: Some(vec!["ecuador"]),
                demonyms: None,
                enterprises: None,
                misc: Some(vec!["union for hope"]),
            }
            .get_region_vec(),
            "Ecuador",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "EE"),
                names: Some(vec!["estonia"]),
                demonyms: None,
                enterprises: None,
                misc: None,
            }
            .get_region_vec(),
            "Estonia",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "EG"),
                names: Some(vec!["egypt"]),
                demonyms: None,
                enterprises: None,
                misc: None,
            }
            .get_region_vec(),
            "Egypt",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "EH"),
                names: Some(vec!["western sahara"]),
                demonyms: None,
                enterprises: None,
                misc: Some(vec!["polisario"]),
            }
            .get_region_vec(),
            "Western Sahara",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "ER"),
                names: Some(vec!["eritrea"]),
                demonyms: None,
                enterprises: None,
                misc: Some(vec!["pfdj"]),
            }
            .get_region_vec(),
            "Eritrea",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "ES"),
                names: Some(vec!["spain"]),
                demonyms: Some(vec!["spaniard"]),
                enterprises: Some(vec![
                    "inditex",
                    "iberdrola",
                    "santander",
                    "bilbao vizcaya",
                    "caixabank",
                    "amadeus it",
                    "ferrovial",
                    "aena",
                    "cellnex",
                    "naturgy",
                    "telefonica",
                    "endesa",
                    "repsol",
                    "edp renovaveis",
                    "international consolidated airlines",
                    "sabadell",
                    "grupo acs",
                ]),
                misc: Some(vec!["cortes generales", "psoe", "sumar"]),
            }
            .get_region_vec(),
            "Spain",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "ET"),
                names: Some(vec!["ethiopia"]),
                demonyms: None,
                enterprises: None,
                misc: Some(vec![
                    "house of federation",
                    "house of people's representatives",
                    "prosperity party",
                    "national movement of amhara",
                ]),
            }
            .get_region_vec(),
            "Ethiopia",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "FI"),
                names: Some(vec!["finland"]),
                demonyms: Some(vec!["finn"]),
                enterprises: Some(vec![
                    "nordea bank",
                    "kone",
                    "sampo",
                    "nokia",
                    "upm-kymmene",
                    "neste",
                    "fortum",
                    "wartsila",
                    "stora enso",
                    "metso",
                ]),
                misc: Some(vec!["eduskunta", "national coalition party"]),
            }
            .get_region_vec(),
            "Finland",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "FJ"),
                names: Some(vec!["fiji"]),
                demonyms: None,
                enterprises: None,
                misc: None,
            }
            .get_region_vec(),
            "Fiji",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "FK"),
                names: Some(vec!["falkland", "malvinas"]),
                demonyms: None,
                enterprises: None,
                misc: None,
            }
            .get_region_vec(),
            "Falkland Islands",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "FM"),
                names: Some(vec!["micronesia", "fsm"]),
                demonyms: None,
                enterprises: None,
                misc: None,
            }
            .get_region_vec(),
            "Micronesia",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "FO"),
                names: Some(vec!["faroe island"]),
                demonyms: None,
                enterprises: None,
                misc: Some(vec!["logting"]),
            }
            .get_region_vec(),
            "Faroe Islands",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "FR"),
                names: Some(vec!["france"]),
                demonyms: None,
                enterprises: Some(vec![
                    "lvmh",
                    "hermes inc",
                    "l'oreal",
                    "totalenergies",
                    "dior",
                    "schneider electric",
                    "sanofi",
                    "air liquide",
                    "essilorluxottica",
                    "safran",
                    "bnp paribas",
                    "axa",
                    "vinci",
                    "dassault",
                    "credit agricole",
                    "compagnie de saint-gobain",
                    "kering",
                    "danone",
                    "engie",
                    "pernod ricard",
                    "capgemini",
                    "thales",
                    "orange inc",
                    "michelin",
                    "legrand",
                    "publicis group",
                    "veolia",
                    "societe generale",
                    "bollore",
                    "renault",
                    "amundi",
                    "bouygues",
                    "sodexo",
                    "bureau veritas",
                    "edenred",
                    "carrefour",
                    "biomerieux",
                    "unibail-rodamco",
                    "rodamco-westfield",
                    "vivendi",
                    "accor inc",
                    "ipsen",
                    "eiffage",
                ]),
                misc: Some(vec!["macron"]),
            }
            .get_region_vec(),
            "France",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "GA"),
                names: Some(vec!["gabon"]),
                demonyms: None,
                enterprises: None,
                misc: Some(vec!["ctri"]),
            }
            .get_region_vec(),
            "Gabon",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "GB"),
                names: Some(vec![
                    "united kingdom",
                    "uk",
                    "britain",
                    "england",
                    "scotland",
                    "wales",
                    "northern ireland",
                ]),
                demonyms: Some(vec!["british", "scottish", "welsh", "northern irish"]),
                enterprises: Some(vec![
                    "astrazeneca",
                    "shell oil",
                    "shell inc",
                    "linde",
                    "hsbc",
                    "unilever",
                    "rio tonto",
                    "arm holding",
                    "bp",
                    "glaxosmithkline",
                    "relx",
                    "diageo",
                    "aon",
                    "national grid inc",
                    "bae systems",
                    "compass group",
                    "anglo american inc",
                    "rolls-royce",
                    "lloyds bank",
                    "ferguson inc",
                    "barclays",
                    "reckitt benckiser",
                    "haleon",
                    "natwest",
                    "3i group",
                    "ashtead",
                    "antofagasta",
                    "prudential inc",
                    "tesco",
                    "vodafone inc",
                    "willis towers watson",
                    "sse",
                    "standard chartered",
                    "imperial brands inc",
                    "legal & general",
                    "bt group",
                    "intercontinental hotels group",
                    "royalty pharma",
                    "segro",
                    "next plc",
                    "informa plc",
                    "cnh",
                    "sage group",
                    "pentair",
                    "rentokil",
                    "nvent electric inc",
                    "bunzi",
                    "wpp",
                    "technipfmc",
                    "smith & nephew",
                    "halma",
                    "wise plc",
                    "intertek",
                    "melrose industries",
                    "admiral group",
                    "severn trent",
                ]),
                misc: Some(vec!["house of lords", "stormont", "sunak"]),
            }
            .get_region_vec(),
            "United Kingdom",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "GD"),
                names: Some(vec!["grenada"]),
                demonyms: Some(vec!["grenadian"]),
                enterprises: None,
                misc: Some(vec!["rgpf"]),
            }
            .get_region_vec(),
            "Grenada",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "GE"),
                names: None,
                demonyms: None,
                enterprises: None,
                misc: Some(vec!["abkhaz", "united national movement"]),
            }
            .get_region_vec(),
            "Georgia",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "GF"),
                names: Some(vec!["french guiana"]),
                demonyms: Some(vec!["french guianan", "french guinese"]),
                enterprises: None,
                misc: None,
            }
            .get_region_vec(),
            "French Guiana",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "GG"),
                names: Some(vec!["guernsey"]),
                demonyms: Some(vec!["giernesiais"]),
                enterprises: None,
                misc: None,
            }
            .get_region_vec(),
            "Guernsey",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "GH"),
                names: Some(vec!["ghana"]),
                demonyms: None,
                enterprises: None,
                misc: Some(vec!["national democratic congress", "new patriotic party"]),
            }
            .get_region_vec(),
            "Ghana",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "GI"),
                names: None, // Name comes from database.
                demonyms: Some(vec!["llanito"]),
                enterprises: None,
                misc: Some(vec!["gslp"]),
            }
            .get_region_vec(),
            "Gibraltar",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "GL"),
                names: Some(vec!["greenland"]),
                demonyms: None,
                enterprises: None,
                misc: Some(vec!["inuit ataqatigiit", "naleraq", "siumut"]),
            }
            .get_region_vec(),
            "Greenland",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "GM"),
                names: Some(vec!["gambia"]),
                demonyms: None,
                enterprises: None,
                misc: None,
            }
            .get_region_vec(),
            "Gambia",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "GN"),
                names: None,
                demonyms: None,
                enterprises: None,
                misc: Some(vec!["cnrd"]),
            }
            .get_region_vec(),
            "Guinea",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "GP"),
                names: Some(vec!["guadeloupe"]),
                demonyms: None,
                enterprises: None,
                misc: None,
            }
            .get_region_vec(),
            "Guadeloupe",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "GQ"),
                names: Some(vec!["equatorial guinea"]),
                demonyms: Some(vec!["equatoguinean"]),
                enterprises: None,
                misc: Some(vec!["pdge"]),
            }
            .get_region_vec(),
            "Equatorial Guinea",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "GR"),
                names: Some(vec!["greece"]),
                demonyms: Some(vec!["greek"]),
                enterprises: None,
                misc: Some(vec!["helleni", "syriza"]),
            }
            .get_region_vec(),
            "Greece",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "GS"),
                names: Some(vec!["south georgia", "south sandwich"]),
                demonyms: None,
                enterprises: None,
                misc: None,
            }
            .get_region_vec(),
            "South Georgia and the South Sandwich Islands",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "GT"),
                names: Some(vec!["guatemala"]),
                demonyms: Some(vec!["chapin"]),
                enterprises: None,
                misc: Some(vec!["semilla"]),
            }
            .get_region_vec(),
            "Guatemala",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "GU"),
                names: Some(vec!["guam"]),
                demonyms: None,
                enterprises: None,
                misc: None,
            }
            .get_region_vec(),
            "Guam",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "GW"),
                names: Some(vec!["guinea-bissau"]),
                demonyms: Some(vec!["bissau-guinean"]),
                enterprises: None,
                misc: Some(vec!["terra ranka", "paigc", "madem g15", "madem-g15"]),
            }
            .get_region_vec(),
            "Guinea-Bissau",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "GY"),
                names: Some(vec!["guyan"]),
                demonyms: None,
                enterprises: None,
                misc: None,
            }
            .get_region_vec(),
            "Guyana",
        ),
        (
            RegionKeyphrases {
                // I am not including "... China ..." keyphrases for this region, as I value the 'China" keyphrase more for the China region.
                automated: get_automated_keyphrases(&region_map, "HK"),
                names: Some(vec!["hong kong"]),
                demonyms: Some(vec!["hongkong"]),
                enterprises: Some(vec![
                    "aia",
                    "sun hung kai",
                    "jardine matheson",
                    "hang seng",
                    "techtronic",
                    "mtr",
                    "galaxy entertainment",
                    "clp",
                    "ck hutchison",
                    "budweiser apac",
                    "lenovo",
                    "ck asset",
                    "ck holding",
                    "ck infrastructure",
                    "chow tai fook",
                    "power assets inc",
                    "link reit",
                    "swire",
                    "orient overseas",
                    "futu holding",
                    "wharf reic",
                    "wharf holding",
                    "sino land",
                ]),
                misc: Some(vec!["legco"]),
            }
            .get_region_vec(),
            "Hong Kong",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "HM"),
                names: Some(vec!["heard island", "mcdonald island"]),
                demonyms: None,
                enterprises: None,
                misc: None,
            }
            .get_region_vec(),
            "Heard Island and McDonald Islands",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "HN"),
                names: Some(vec!["hondura"]),
                demonyms: Some(vec!["catrach"]),
                enterprises: None,
                misc: Some(vec!["liberty and refoundation"]),
            }
            .get_region_vec(),
            "Honduras",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "HR"),
                names: Some(vec!["croatia"]),
                demonyms: None,
                enterprises: None,
                misc: Some(vec!["hdz"]),
            }
            .get_region_vec(),
            "Croatia",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "HT"),
                names: Some(vec!["haiti"]),
                demonyms: None,
                enterprises: None,
                misc: Some(vec!["phtk"]),
            }
            .get_region_vec(),
            "Haiti",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "HU"),
                names: Some(vec!["hungar"]),
                demonyms: None,
                enterprises: None,
                misc: Some(vec!["fidesz", "orban"]),
            }
            .get_region_vec(),
            "Hungary",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "ID"),
                names: Some(vec!["indonesia"]),
                demonyms: None,
                enterprises: Some(vec![
                    "bank central asia",
                    "chandra asri",
                    "raykat",
                    "bayan resources",
                    "mandiri",
                    "astra international",
                ]),
                misc: Some(vec!["pdi-p", "golkar", "prosperous justice party"]),
            }
            .get_region_vec(),
            "Indonesia",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "IE"),
                names: None,
                demonyms: Some(vec!["irish"]),
                enterprises: Some(vec![
                    "accenture",
                    "eaton",
                    "medtronic",
                    "trane tech",
                    "cement roadstone",
                    "johnson controls",
                    "experian",
                    "ingersoll",
                    "flutter entertainment",
                    "ryanair",
                    "icon plc",
                    "steris",
                    "aptiv inc",
                    "seagate",
                    "aercap",
                    "kingspan",
                    "james hardie",
                    "kerry group",
                    "aib",
                    "smurfit kappa",
                    "bank of ireland",
                    "allegion",
                ]),
                misc: Some(vec!["oireachtas", "fianna fail", "fine gael", "sinn fein"]),
            }
            .get_region_vec(),
            "Ireland",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "IL"),
                names: Some(vec!["israel"]),
                demonyms: None,
                enterprises: Some(vec![
                    "mobileye",
                    "teva",
                    "check point software",
                    "nice inc",
                    "leumi",
                    "hapoalim",
                    "monday.com",
                    "cyberark",
                ]),
                misc: Some(vec![
                    "knesset",
                    "likud",
                    "shas",
                    "united torah judaism",
                    "mafdal",
                    "otzma",
                    "yesh atid",
                    "haaretz",
                    "netanyahu",
                    "yoav gallant",
                ]),
            }
            .get_region_vec(),
            "Israel",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "IM"),
                names: Some(vec!["isle of man"]),
                demonyms: Some(vec!["manx"]),
                enterprises: None,
                misc: Some(vec!["tynwald"]),
            }
            .get_region_vec(),
            "Isle of Man",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "IN"),
                names: Some(vec!["india", "hindustan"]),
                demonyms: None,
                enterprises: Some(vec![
                    "reliance industries",
                    "tata",
                    "hdfc",
                    "bharti airtel",
                    "icici",
                    "lic",
                    "infosys",
                    "itc",
                    "larsen & toubro",
                    "bajaj",
                    "maruti suzuki",
                    "sun pharma",
                    "hcl tech",
                    "ntpc",
                    "axis bank",
                    "oil & natural gas inc",
                    "adani",
                    "mahindra",
                    "dmart",
                    "titan company inc",
                    "ultratech cement",
                    "asian paints inc",
                    "wipro",
                    "jio financial",
                    "jio services",
                    "jsw",
                    "dlf",
                    "varun",
                    "bharat electronics",
                    "zomato",
                    "interglobe aviation",
                    "trent limited",
                    "vedanta",
                    "grasim",
                    "power finance corp",
                    "ambuja",
                    "pidilite",
                    "hindalco",
                    "sbi life",
                    "rural electrificaiton group",
                    "ltimindtree",
                    "punjab bank",
                    "punjab national",
                    "bank of baroda",
                    "gail inc",
                    "godrej",
                    "eicher motor",
                    "britannia industries",
                    "lodha",
                    "havells",
                    "cipla",
                    "indusind",
                    "cholamandalam",
                    "zydus",
                    "divis lab",
                    "tvs motor",
                    "canara",
                    "jindal",
                    "hero motocorp",
                    "cg power and",
                    "cg industrial solutions",
                    "nhpc",
                    "dr. reddy's",
                    "dabur",
                    "shree cement",
                    "indus towers",
                    "torrent pharma",
                    "idbi bank",
                    "shriram",
                    "vodafone idea",
                    "samvardhana",
                    "apollo hospitals",
                    "united spirits",
                    "mankind pharma",
                ]),
                misc: Some(vec!["lok sabha", "rajya sabha", "bjp"]),
            }
            .get_region_vec(),
            "India",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "IO"),
                names: Some(vec!["british indian ocean territory"]),
                demonyms: None,
                enterprises: None,
                misc: None,
            }
            .get_region_vec(),
            "British Indian Ocean Territory",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "IQ"),
                names: Some(vec!["iraq"]),
                demonyms: None,
                enterprises: None,
                misc: Some(vec!["takadum", "emtidad"]),
            }
            .get_region_vec(),
            "Iraq",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "IR"),
                names: Some(vec!["iran ", "iran'", "iran\"", "iran.", "iran,"]),
                demonyms: None,
                enterprises: None,
                misc: Some(vec!["guardian council", "khomeini"]),
            }
            .get_region_vec(),
            "Iran",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "IS"),
                names: Some(vec!["iceland"]),
                demonyms: None,
                enterprises: None,
                misc: Some(vec!["althing", "samfylkingin"]),
            }
            .get_region_vec(),
            "Iceland",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "IT"),
                names: Some(vec!["italy"]),
                demonyms: Some(vec!["italian"]),
                enterprises: Some(vec![
                    "ferrari",
                    "enel inc",
                    "intesa sanpaolo",
                    "unicredit",
                    "eni",
                    "generali",
                    "prada",
                    "moncler",
                    "terna",
                    "prysmian",
                    "snam",
                    "leonardo inc",
                    "mediobanca",
                    "davide campari",
                    "campari-milano",
                    "recordati",
                    "banco bpm",
                    "inwit",
                    "finecobank",
                ]),
                misc: Some(vec!["lega", "pd-idp"]),
            }
            .get_region_vec(),
            "Italy",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "JE"),
                names: None,
                demonyms: Some(vec![
                    "jerseyman",
                    "jerseywoman",
                    "jersey bean",
                    "jersey crapaud",
                    "jerriais",
                ]),
                enterprises: None,
                misc: None,
            }
            .get_region_vec(),
            "Jersey",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "JM"),
                names: None, // Name comes from database.
                demonyms: None,
                enterprises: None,
                misc: None,
            }
            .get_region_vec(),
            "Jamaica",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "JO"),
                names: None,
                demonyms: Some(vec!["jordanian"]),
                enterprises: None,
                misc: Some(vec!["islamic action front"]),
            }
            .get_region_vec(),
            "Jordan",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "JP"),
                names: Some(vec!["japan", "nippon"]),
                demonyms: None,
                enterprises: Some(vec![
                    "toyota",
                    "mitsubishi",
                    "keyence",
                    "sony",
                    "hitachi",
                    "ntt",
                    "sumitomo",
                    "mitsui",
                    "fast retailing inc",
                    "softbank",
                    "recruit inc",
                    "shin-etsu",
                    "daiichi",
                    "sankyo",
                    "itochu",
                    "shoji",
                    "nintendo",
                    "kddi",
                    "honda",
                    "chugai pharma",
                    "mizuho",
                    "denso",
                    "oriental land inc",
                    "daikin",
                    "hoya",
                    "takeda pharma",
                    "disco corp",
                    "murata",
                    "7-eleven",
                    "smc corp",
                    "marubeni",
                    "renesas",
                    "bridgestone",
                    "ms&ad",
                    "komatsu",
                    "fanuc",
                    "fujitsu",
                    "canon inc",
                    "nidec",
                    "terumo",
                    "fujifilm",
                    "advantest",
                    "orix",
                    "lasertec",
                    "dai-ichi",
                    "otsuka",
                    "suzuki motor",
                    "kao",
                    "sompo",
                    "panasonic",
                    "ajinomoto",
                    "unicharm",
                    "asahi group",
                    "inpex",
                    "olympus inc",
                    "z holding",
                    "nec",
                    "aeon inc",
                    "kubota",
                    "nomura",
                    "tdk",
                    "astellas pharma",
                    "daiwa",
                    "kyocera",
                    "subaru",
                    "shimano",
                    "resona holding",
                    "pan pacific international holding",
                    "sekisui",
                    "nexon",
                    "eneos",
                    "kepco",
                    "secom",
                    "nitori",
                    "nissan",
                    "bandai namco",
                    "shionogi",
                    "eisai",
                    "shiseido",
                    "obic",
                    "kirin holding",
                    "suntory",
                    "shinkin",
                    "nitto denko",
                    "kikkoman",
                    "sysmex",
                    "rakuten",
                    "yaskawa",
                    "\"k\" line",
                ]),
                misc: Some(vec!["komeito", "tokio"]),
            }
            .get_region_vec(),
            "Japan",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "KE"),
                names: Some(vec!["kenya"]),
                demonyms: None,
                enterprises: None,
                misc: Some(vec!["azimio"]),
            }
            .get_region_vec(),
            "Kenya",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "KG"),
                names: None,
                demonyms: Some(vec!["kyrgyz"]),
                enterprises: None,
                misc: Some(vec!["jogorku kenesh", "mekenchil", "eldik"]),
            }
            .get_region_vec(),
            "Kyrgyzstan",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "KH"),
                names: Some(vec!["cambodia"]),
                demonyms: Some(vec!["khmer"]),
                enterprises: None,
                misc: Some(vec!["funcinpec"]),
            }
            .get_region_vec(),
            "Cambodia",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "KI"),
                names: Some(vec!["kiribati"]),
                demonyms: None,
                enterprises: None,
                misc: None,
            }
            .get_region_vec(),
            "Kiribati",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "KM"),
                names: Some(vec!["comoros"]),
                demonyms: Some(vec!["comorian"]),
                enterprises: None,
                misc: Some(vec!["orange party"]),
            }
            .get_region_vec(),
            "Comoros",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "KN"),
                names: Some(vec!["kitts", "nevis"]),
                demonyms: Some(vec!["kittitian", "nevisian"]),
                enterprises: None,
                misc: Some(vec!["concerned citizens' movement"]),
            }
            .get_region_vec(),
            "Saint Kitts and Nevis",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "KP"),
                names: Some(vec!["north korea"]),
                demonyms: None,
                enterprises: None,
                misc: Some(vec!["supreme people's assembly", "dprk"]),
            }
            .get_region_vec(),
            "North Korea",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "KR"),
                names: Some(vec!["south korea"]),
                demonyms: None,
                enterprises: Some(vec![
                    "samsung",
                    "sk hynix",
                    "lg",
                    "hyundai",
                    "coupang",
                    "kia",
                    "celltrion",
                    "kb financial",
                    "kb group",
                    "posco",
                    "naver",
                    "shinhan",
                    "kakao",
                    "hana financial",
                    "hana group",
                ]),
                misc: Some(vec!["people power party"]),
            }
            .get_region_vec(),
            "South Korea",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "KW"),
                names: Some(vec!["kuwait"]),
                demonyms: None,
                enterprises: None,
                misc: None,
            }
            .get_region_vec(),
            "Kuwait",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "KY"),
                names: Some(vec!["cayman"]),
                demonyms: None,
                enterprises: None,
                misc: None,
            }
            .get_region_vec(),
            "Cayman Islands",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "KZ"),
                names: None,
                demonyms: Some(vec!["kazakh"]),
                enterprises: None,
                misc: Some(vec!["mazhilis", "amanat", "auyl"]),
            }
            .get_region_vec(),
            "Kazakhstan",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "LA"),
                names: Some(vec!["laos"]),
                demonyms: Some(vec!["lao", "laotian"]), // Strings with length 3 or less are processed before substring checking.
                enterprises: None,
                misc: Some(vec!["lprp"]),
            }
            .get_region_vec(),
            "Laos",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "LB"),
                names: Some(vec!["lebanon"]),
                demonyms: Some(vec!["lebanese"]),
                enterprises: None,
                misc: Some(vec![
                    "free patriotic movement",
                    "amal movement",
                    "hezbollah",
                    "march 14 alliance",
                    "march 8 alliance",
                ]),
            }
            .get_region_vec(),
            "Lebanon",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "LC"),
                names: Some(vec!["saint lucia"]),
                demonyms: None,
                enterprises: None,
                misc: None,
            }
            .get_region_vec(),
            "Saint Lucia",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "LI"),
                names: Some(vec!["liechtenstein"]),
                demonyms: None,
                enterprises: None,
                misc: None,
            }
            .get_region_vec(),
            "Liechtenstein",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "LK"),
                names: Some(vec!["sri lanka"]),
                demonyms: None,
                enterprises: None,
                misc: Some(vec!["slpfa", "samagi jana balawegaya"]),
            }
            .get_region_vec(),
            "Sri Lanka",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "LR"),
                names: Some(vec!["liberia"]),
                demonyms: None,
                enterprises: None,
                misc: Some(vec!["coalition for democratic change"]),
            }
            .get_region_vec(),
            "Liberia",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "LS"),
                names: None, // Name comes from database.
                demonyms: Some(vec!["mosotho", "basotho"]),
                enterprises: None,
                misc: Some(vec!["revolution for prosperity"]),
            }
            .get_region_vec(),
            "Lesotho",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "LT"),
                names: Some(vec!["lithuania"]),
                demonyms: None,
                enterprises: None,
                misc: Some(vec!["seimas", "homeland union", "lvzs"]),
            }
            .get_region_vec(),
            "Lithuania",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "LU"),
                names: None, // Name comes from database.
                demonyms: None,
                enterprises: Some(vec!["arcelormittal", "tenaris", "eurofins"]),
                misc: Some(vec!["christian social people's party", "lsap"]),
            }
            .get_region_vec(),
            "Luxembourg",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "LV"),
                names: Some(vec!["latvia"]),
                demonyms: None,
                enterprises: None,
                misc: Some(vec!["saeima", "zzs"]),
            }
            .get_region_vec(),
            "Latvia",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "LY"),
                names: None, // Name comes from database.
                demonyms: None,
                enterprises: None,
                misc: Some(vec!["government of national"]),
            }
            .get_region_vec(),
            "Libya",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "MA"),
                names: Some(vec!["morocc"]),
                demonyms: None,
                enterprises: None,
                misc: Some(vec![
                    "national rally of independents",
                    "istiqlal party",
                    "authenticity and modernity party",
                    "usfp",
                ]),
            }
            .get_region_vec(),
            "Morocco",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "MC"),
                names: None, // Name comes from database.
                demonyms: Some(vec!["monegasque", "monacan"]),
                enterprises: None,
                misc: None,
            }
            .get_region_vec(),
            "Monaco",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "MD"),
                names: Some(vec!["moldova"]),
                demonyms: None,
                enterprises: None,
                misc: Some(vec!["party of action and solidarity", "psrm"]),
            }
            .get_region_vec(),
            "Moldova",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "ME"),
                names: Some(vec!["monteneg"]),
                demonyms: None,
                enterprises: None,
                misc: Some(vec!["pes!"]),
            }
            .get_region_vec(),
            "Montenegro",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "MF"),
                names: Some(vec!["saint martin"]),
                demonyms: None,
                enterprises: None,
                misc: None,
            }
            .get_region_vec(),
            "Saint Martin",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "MG"),
                names: Some(vec!["madagas"]),
                demonyms: Some(vec!["malagas"]),
                enterprises: None,
                misc: None,
            }
            .get_region_vec(),
            "Madagascar",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "MH"),
                names: Some(vec!["marshall island"]),
                demonyms: Some(vec!["marshallese"]),
                enterprises: None,
                misc: None,
            }
            .get_region_vec(),
            "Marshall Islands",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "MK"),
                names: Some(vec!["north macedonia"]),
                demonyms: None,
                enterprises: None,
                misc: Some(vec!["sobranie", "sdsm", "vmro-dpmne"]),
            }
            .get_region_vec(),
            "North Macedonia",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "ML"),
                names: Some(vec!["mali"]),
                demonyms: None,
                enterprises: None,
                misc: None,
            }
            .get_region_vec(),
            "Mali",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "MM"),
                names: Some(vec!["myanma"]),
                demonyms: Some(vec!["burmese"]),
                enterprises: None,
                misc: Some(vec!["pyidaungsu hluttaw", "nld"]),
            }
            .get_region_vec(),
            "Myanmar",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "MN"),
                names: Some(vec!["mongol"]),
                demonyms: None,
                enterprises: None,
                misc: Some(vec!["state great khural"]),
            }
            .get_region_vec(),
            "Mongolia",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "MO"),
                names: Some(vec!["macau", "macao"]),
                demonyms: None,
                enterprises: None,
                misc: None,
            }
            .get_region_vec(),
            "Macau",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "MP"),
                names: Some(vec!["northern mariana island"]),
                demonyms: Some(vec!["marianan", "chamorro"]),
                enterprises: None,
                misc: None,
            }
            .get_region_vec(),
            "Northern Mariana Islands",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "MQ"),
                names: Some(vec!["martiniq"]),
                demonyms: Some(vec!["martinic"]),
                enterprises: None,
                misc: None,
            }
            .get_region_vec(),
            "Martinique",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "MR"),
                names: Some(vec!["mauritania"]),
                demonyms: None,
                enterprises: None,
                misc: Some(vec!["tewassoul"]),
            }
            .get_region_vec(),
            "Mauritania",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "MS"),
                names: Some(vec!["montserrat"]),
                demonyms: None,
                enterprises: None,
                misc: Some(vec!["movement for change and prosperity"]),
            }
            .get_region_vec(),
            "Montserrat",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "MT"),
                names: Some(vec!["malta"]),
                demonyms: Some(vec!["maltese"]),
                enterprises: None,
                misc: None,
            }
            .get_region_vec(),
            "Malta",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "MU"),
                names: Some(vec!["mauriti"]),
                demonyms: None,
                enterprises: None,
                misc: Some(vec!["mauricien"]),
            }
            .get_region_vec(),
            "Mauritius",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "MV"),
                names: Some(vec!["maldiv"]),
                demonyms: None,
                enterprises: None,
                misc: Some(vec!["people's majlis"]),
            }
            .get_region_vec(),
            "Maldives",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "MW"),
                names: Some(vec!["malawi"]),
                demonyms: None,
                enterprises: None,
                misc: None,
            }
            .get_region_vec(),
            "Malawi",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "MX"),
                names: None,
                demonyms: Some(vec!["mexican"]),
                enterprises: Some(vec![
                    "walmex",
                    "america movil",
                    "banorte",
                    "femsa",
                    "grupo carso",
                    "grupo bimbo",
                    "financiero inbursa",
                    "arca continental",
                    "grupo elektra",
                    "cemex",
                    "aeroportuario del sureste",
                ]),
                misc: None,
            }
            .get_region_vec(),
            "Mexico",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "MY"),
                names: Some(vec!["malaysia"]),
                demonyms: None,
                enterprises: Some(vec![
                    "maybank",
                    "pbbank",
                    "bank bhd",
                    "tenaga",
                    "cimb",
                    "pchem",
                    "ihh",
                    "celcomdigi",
                ]),
                misc: None,
            }
            .get_region_vec(),
            "Malaysia",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "MZ"),
                names: Some(vec!["mozambi"]),
                demonyms: None,
                enterprises: None,
                misc: Some(vec!["frelimo", "renamo"]),
            }
            .get_region_vec(),
            "Mozambique",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "NA"),
                names: Some(vec!["namibia"]),
                demonyms: None,
                enterprises: None,
                misc: Some(vec!["swapo"]),
            }
            .get_region_vec(),
            "Namibia",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "NC"),
                names: Some(vec!["new caledonia"]),
                demonyms: Some(vec!["caledonian"]),
                enterprises: None,
                misc: Some(vec!["flnks", "l'eo"]),
            }
            .get_region_vec(),
            "New Caledonia",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "NE"),
                names: None,
                demonyms: Some(vec!["nigerien"]),
                enterprises: None,
                misc: None,
            }
            .get_region_vec(),
            "Niger",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "NF"),
                names: Some(vec!["norfolk island"]),
                demonyms: None,
                enterprises: None,
                misc: None,
            }
            .get_region_vec(),
            "Norfolk Island",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "NG"),
                names: Some(vec!["nigeria"]),
                demonyms: None,
                enterprises: None,
                misc: Some(vec!["all progressives congress"]),
            }
            .get_region_vec(),
            "Nigeria",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "NI"),
                names: Some(vec!["nicaragua"]),
                demonyms: Some(vec!["pinoler"]),
                enterprises: None,
                misc: Some(vec!["sandinista"]),
            }
            .get_region_vec(),
            "Nicaragua",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "NL"),
                names: Some(vec!["netherlands", "nederland"]),
                demonyms: Some(vec!["dutch"]),
                enterprises: Some(vec![
                    "asml",
                    "prosus",
                    "airbus",
                    "nxp",
                    "stellantis",
                    "heineken",
                    "ing",
                    "universal music group",
                    "umg",
                    "adyen",
                    "exor",
                    "wolters kluwer",
                    "asm international",
                    "ahold delhaize",
                    "philips",
                    "argenx",
                    "yandex",
                    "kpn",
                    "abn amro",
                    "nn group",
                    "aegon",
                    "akzonobel",
                    "jde peet",
                    "be semiconductor",
                    "euronext",
                    "qiagen",
                ]),
                misc: Some(vec![
                    "vvd",
                    "d66",
                    "pvv",
                    "icc",
                    "international criminal court",
                ]),
            }
            .get_region_vec(),
            "Netherlands",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "NO"),
                names: Some(vec!["norway"]),
                demonyms: Some(vec!["norwegian"]),
                enterprises: Some(vec![
                    "equinor",
                    "dnb inc",
                    "telenor",
                    "aker bp",
                    "kongsberg gruppen",
                    "adevinta",
                    "norsk hydro",
                ]),
                misc: Some(vec!["storting"]),
            }
            .get_region_vec(),
            "Norway",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "NP"),
                names: Some(vec!["nepal"]),
                demonyms: None,
                enterprises: None,
                misc: None,
            }
            .get_region_vec(),
            "Nepal",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "NR"),
                names: Some(vec!["nauru"]),
                demonyms: None,
                enterprises: None,
                misc: None,
            }
            .get_region_vec(),
            "Nauru",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "NU"),
                names: Some(vec!["niue"]),
                demonyms: None,
                enterprises: None,
                misc: None,
            }
            .get_region_vec(),
            "Niue",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "NZ"),
                names: Some(vec!["new zealand"]),
                demonyms: Some(vec!["kiwi"]),
                enterprises: Some(vec!["xero", "fisher & paykel"]),
                misc: Some(vec!["parliament", "nzlp"]),
            }
            .get_region_vec(),
            "New Zealand",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "OM"),
                names: Some(vec!["oman"]),
                demonyms: None,
                enterprises: None,
                misc: None,
            }
            .get_region_vec(),
            "Oman",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "PA"),
                names: Some(vec!["panama"]),
                demonyms: None,
                enterprises: None,
                misc: Some(vec!["molirena"]),
            }
            .get_region_vec(),
            "Panama",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "PE"),
                names: Some(vec!["peru"]),
                demonyms: None,
                enterprises: None,
                misc: Some(vec!["fujimoris"]),
            }
            .get_region_vec(),
            "Peru",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "PF"),
                names: Some(vec!["french polynesia"]),
                demonyms: None,
                enterprises: None,
                misc: Some(vec!["tavini", "tapura"]),
            }
            .get_region_vec(),
            "French Polynesia",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "PG"),
                names: Some(vec!["papua new guinea"]),
                demonyms: None,
                enterprises: None,
                misc: Some(vec!["pangu pati"]),
            }
            .get_region_vec(),
            "Papua New Guinea",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "PH"),
                names: Some(vec!["philippine"]),
                demonyms: Some(vec!["filipin", "pinoy"]),
                enterprises: Some(vec![
                    "sm investments",
                    "sm corp",
                    "sm prime",
                    "sm holding",
                    "bdo",
                    "international container terminal services",
                    "ayala",
                ]),
                misc: Some(vec!["uniteam alliance", "tropa"]),
            }
            .get_region_vec(),
            "Philippines",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "PK"),
                names: Some(vec!["pakistan"]),
                demonyms: None,
                enterprises: None,
                misc: Some(vec!["pml-n", "ittehad council"]),
            }
            .get_region_vec(),
            "Pakistan",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "PL"),
                names: Some(vec!["poland", "polsk"]),
                demonyms: Some(vec!["polish"]),
                enterprises: Some(vec![
                    "pkn",
                    "orlen",
                    "pko",
                    "powszechny",
                    "zaklad",
                    "ubezpieczen",
                    "pekao",
                    "allegro.eu",
                ]),
                misc: Some(vec!["sejm"]),
            }
            .get_region_vec(),
            "Poland",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "PM"),
                names: Some(vec!["saint pierre", "miquelon"]),
                demonyms: Some(vec!["saint-pierrais", "miquelonnais", "pierrian"]),
                enterprises: None,
                misc: Some(vec!["archipelago tomorrow"]),
            }
            .get_region_vec(),
            "Saint Pierre and Miquelon",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "PN"),
                names: Some(vec!["pitcairn"]),
                demonyms: Some(vec!["pitkern"]),
                enterprises: None,
                misc: None,
            }
            .get_region_vec(),
            "Pitcairn Islands",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "PR"),
                names: Some(vec!["puerto ric"]),
                demonyms: None,
                enterprises: None,
                misc: None,
            }
            .get_region_vec(),
            "Puerto Rico",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "PS"),
                names: Some(vec!["palestin"]),
                demonyms: None,
                enterprises: None,
                misc: Some(vec!["plo", "hamas", "fatah", "gaza", "rafah"]),
            }
            .get_region_vec(),
            "Palestine",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "PT"),
                names: Some(vec!["portugal"]),
                demonyms: Some(vec!["portuguese"]),
                enterprises: Some(vec!["edp group", "galp energ", "jeronimo martins"]),
                misc: None,
            }
            .get_region_vec(),
            "Portugal",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "PW"),
                names: Some(vec!["palau"]),
                demonyms: None,
                enterprises: None,
                misc: None,
            }
            .get_region_vec(),
            "Palau",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "PY"),
                names: Some(vec!["paraguay"]),
                demonyms: None,
                enterprises: None,
                misc: None,
            }
            .get_region_vec(),
            "Paraguay",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "QA"),
                names: Some(vec!["qatar"]),
                demonyms: None,
                enterprises: Some(vec!["qnb inc"]),
                misc: Some(vec!["house of thani"]),
            }
            .get_region_vec(),
            "Qatar",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "RE"),
                names: None,
                demonyms: Some(vec!["reunionese"]),
                enterprises: None,
                misc: None,
            }
            .get_region_vec(),
            "Reunion",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "RO"),
                names: Some(vec!["romania"]),
                demonyms: None,
                enterprises: None,
                misc: None,
            }
            .get_region_vec(),
            "Romania",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "RS"),
                names: Some(vec!["serbia"]),
                demonyms: None,
                enterprises: None,
                misc: Some(vec!["av-zms", "sps-zs"]),
            }
            .get_region_vec(),
            "Serbia",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "RU"),
                names: Some(vec!["russia"]),
                demonyms: None,
                enterprises: Some(vec![
                    "sberbank",
                    "rosneft",
                    "lukoil",
                    "novatek inc",
                    "gazprom",
                    "nornickel",
                    "polyus",
                    "severstal",
                    "tatneft",
                    "novolipetsk",
                    "surgutneftegas",
                ]),
                misc: Some(vec!["state duma", "ldpr", "putin"]),
            }
            .get_region_vec(),
            "Russia",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "RW"),
                names: Some(vec!["rwand"]),
                demonyms: None,
                enterprises: None,
                misc: None,
            }
            .get_region_vec(),
            "Rwanda",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "SA"),
                names: None,
                demonyms: Some(vec!["saudi"]),
                enterprises: Some(vec![
                    "acwa power",
                    "acwa co",
                    "al rajhi",
                    "sabic",
                    "maaden",
                    "dr. sulaiman al habib",
                    "riyad",
                    "alinma",
                    "elm co",
                    "almarai",
                    "albilad",
                    "arab national bank",
                    "etihad etisalat",
                    "mobily",
                ]),
                misc: None,
            }
            .get_region_vec(),
            "Saudi Arabia",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "SB"),
                names: Some(vec!["solomon island"]),
                demonyms: None,
                enterprises: None,
                misc: Some(vec!["kadere party"]),
            }
            .get_region_vec(),
            "Solomon Islands",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "SC"),
                names: Some(vec!["seychell"]),
                demonyms: Some(vec!["seselwa"]),
                enterprises: None,
                misc: None,
            }
            .get_region_vec(),
            "Seychelles",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "SD"),
                names: None,
                demonyms: None,
                enterprises: None,
                misc: None,
            }
            .get_region_vec(),
            "Sudan",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "SE"),
                names: None,
                demonyms: Some(vec!["swedish", "swede"]),
                enterprises: Some(vec![
                    "atlas copco",
                    "investor ab",
                    "spotify",
                    "volvo",
                    "eqt",
                    "assa abloy",
                    "hexgon inc",
                    "skandinaviska",
                    "enskilda banken",
                    "h&m",
                    "sandvik",
                    "epiroc",
                    "evolution gaming",
                    "swedbank",
                    "ericsson",
                    "alfa laval",
                    "svenska",
                    "handelsbanken",
                    "essity",
                    "industrivarden",
                    "lundbergforetagen",
                    "saab",
                    "lifco",
                    "autoliv",
                    "nibe",
                    "telia",
                ]),
                misc: Some(vec!["riksdag"]),
            }
            .get_region_vec(),
            "Sweden",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "SG"),
                names: Some(vec!["singapore"]),
                demonyms: None,
                enterprises: Some(vec![
                    "dbs",
                    "ocbc",
                    "garena",
                    "uob",
                    "singtel",
                    "grab holding",
                    "wilmar international",
                    "flex inc",
                    "capitaland",
                ]),
                misc: Some(vec!["people's action party"]),
            }
            .get_region_vec(),
            "Singapore",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "SH"),
                names: Some(vec!["saint helen"]),
                demonyms: Some(vec!["helenian"]),
                enterprises: None,
                misc: None,
            }
            .get_region_vec(),
            "Saint Helena",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "SI"),
                names: Some(vec!["sloven"]),
                demonyms: None,
                enterprises: None,
                misc: None,
            }
            .get_region_vec(),
            "Slovenia",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "SJ"),
                names: Some(vec!["svalbard", "jan mayen"]),
                demonyms: None,
                enterprises: None,
                misc: None,
            }
            .get_region_vec(),
            "Svalbard and Jan Mayen",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "SK"),
                names: None,
                demonyms: Some(vec!["slovak"]),
                enterprises: None,
                misc: Some(vec!["smer-sd", "hlas-sd"]),
            }
            .get_region_vec(),
            "Slovakia",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "SL"),
                names: Some(vec!["sierra leone"]),
                demonyms: None,
                enterprises: None,
                misc: None,
            }
            .get_region_vec(),
            "Sierra Leone",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "SM"),
                names: Some(vec!["san marino"]),
                demonyms: Some(vec!["sammarinese"]),
                enterprises: None,
                misc: None,
            }
            .get_region_vec(),
            "San Marino",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "SN"),
                names: Some(vec!["senegal"]),
                demonyms: None,
                enterprises: None,
                misc: None,
            }
            .get_region_vec(),
            "Senegal",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "SO"),
                names: None,
                demonyms: Some(vec!["somali"]),
                enterprises: None,
                misc: None,
            }
            .get_region_vec(),
            "Somalia",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "SR"),
                names: Some(vec!["suriname"]),
                demonyms: None,
                enterprises: None,
                misc: None,
            }
            .get_region_vec(),
            "Suriname",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "SS"),
                names: Some(vec!["south sudan"]),
                demonyms: None,
                enterprises: None,
                misc: Some(vec!["splm-in-opposition"]),
            }
            .get_region_vec(),
            "South Sudan",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "ST"),
                names: Some(vec!["sao tome", "principe"]),
                demonyms: Some(vec!["santomean"]),
                enterprises: None,
                misc: Some(vec!["mlstp"]),
            }
            .get_region_vec(),
            "Sao Tome and Principe",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "SV"),
                names: Some(vec!["el salvador"]),
                demonyms: Some(vec!["salvadoran"]),
                enterprises: None,
                misc: Some(vec!["nuevas ideas"]),
            }
            .get_region_vec(),
            "El Salvador",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "SX"),
                names: Some(vec!["maarten"]),
                demonyms: None,
                enterprises: None,
                misc: None,
            }
            .get_region_vec(),
            "Sint Maarten",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "SY"),
                names: Some(vec!["syria"]),
                demonyms: None,
                enterprises: None,
                misc: None,
            }
            .get_region_vec(),
            "Syria",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "SZ"),
                names: Some(vec!["eswatini"]),
                demonyms: Some(vec!["swazi"]),
                enterprises: None,
                misc: Some(vec!["tinkhundla"]),
            }
            .get_region_vec(),
            "Eswatini",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "TC"),
                names: Some(vec!["turks and c", "caicos"]),
                demonyms: Some(vec!["turks islander"]),
                enterprises: None,
                misc: None,
            }
            .get_region_vec(),
            "Turks and Caicos Islands",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "TD"),
                names: None,
                demonyms: Some(vec!["chadian"]),
                enterprises: None,
                misc: Some(vec!["national transitional council"]),
            }
            .get_region_vec(),
            "Chad",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "TF"),
                names: Some(vec![
                    "french southern territories",
                    "adelie land",
                    "crozet island",
                    "kerguelen island",
                    "saint paul and amsterdam island",
                    "scattered islands",
                ]),
                demonyms: Some(vec!["kerguelenois"]),
                enterprises: None,
                misc: None,
            }
            .get_region_vec(),
            "French Southern Territories",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "TG"),
                names: Some(vec!["togo"]),
                demonyms: None,
                enterprises: None,
                misc: Some(vec!["union of forces for change"]),
            }
            .get_region_vec(),
            "Togo",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "TH"),
                names: None,
                demonyms: Some(vec!["thai"]),
                enterprises: Some(vec![
                    "ptt",
                    "advanced info service inc",
                    "cp all",
                    "gulf energy development public co",
                    "bdms",
                    "siam commercial",
                    "siam bank",
                ]),
                misc: Some(vec!["bhumjaithai", "palang pracharath"]),
            }
            .get_region_vec(),
            "Thailand",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "TJ"),
                names: None,
                demonyms: Some(vec!["tajik"]),
                enterprises: None,
                misc: None,
            }
            .get_region_vec(),
            "Tajikistan",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "TK"),
                names: Some(vec!["tokelau"]),
                demonyms: None,
                enterprises: None,
                misc: None,
            }
            .get_region_vec(),
            "Tokelau",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "TL"),
                names: Some(vec!["timor-leste", "east timor"]),
                demonyms: Some(vec!["timorese"]),
                enterprises: None,
                misc: Some(vec!["national parliament", "cnrt", "fretilin"]),
            }
            .get_region_vec(),
            "East Timor",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "TM"),
                names: None,
                demonyms: Some(vec!["turkmen"]),
                enterprises: None,
                misc: None,
            }
            .get_region_vec(),
            "Turkmenistan",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "TN"),
                names: Some(vec!["tunisia"]),
                demonyms: None,
                enterprises: None,
                misc: Some(vec![
                    "assembly of the representatives of the people",
                    "25th of july movement",
                ]),
            }
            .get_region_vec(),
            "Tunisia",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "TO"),
                names: Some(vec!["tonga"]),
                demonyms: None,
                enterprises: None,
                misc: None,
            }
            .get_region_vec(),
            "Tonga",
        ),
        (
            RegionKeyphrases {
                // I did not add "Ford Otosan", as I value the 'Ford' keyphrase more for the United States region.
                automated: get_automated_keyphrases(&region_map, "TR"),
                names: Some(vec!["turkey", "turkiye"]),
                demonyms: Some(vec!["turkish"]),
                enterprises: Some(vec!["qnb finansbank", "koc", "garantibank", "akbank"]),
                misc: Some(vec!["grand national assembly"]),
            }
            .get_region_vec(),
            "Turkey",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "TT"),
                names: Some(vec!["tobago"]),
                demonyms: Some(vec!["trini", "trinbagonian"]),
                enterprises: None,
                misc: None,
            }
            .get_region_vec(),
            "Trinidad and Tobago",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "TV"),
                names: Some(vec!["tuvalu"]),
                demonyms: None,
                enterprises: None,
                misc: None,
            }
            .get_region_vec(),
            "Tuvalu",
        ),
        (
            RegionKeyphrases {
                // I am not including "China Steel", as I value the 'China" keyphrase more for the China region.
                automated: get_automated_keyphrases(&region_map, "TW"),
                names: Some(vec!["taiwan"]),
                demonyms: None,
                enterprises: Some(vec![
                    "tsmc",
                    "foxconn inc",
                    "hon hai",
                    "mediatek",
                    "quanta computer",
                    "chunghwa telecom",
                    "fubon",
                    "delta electronics",
                    "cathay financial",
                    "cathay holding",
                    "ctbc",
                    "ase group",
                    "united microelectronics",
                    "mfhc",
                    "wiwynn",
                    "e.sun bank",
                    "uni-president enterprise",
                    "nan ya",
                    "evergreen marine",
                    "yuanta",
                    "asus",
                    "first financial holding inc",
                    "novatek microelectronics",
                    "hua nan",
                    "hotai motor",
                    "wistron corp",
                ]),
                misc: Some(vec![
                    "legislative yuan",
                    "kuomintang",
                    "guomindang",
                    "formosa",
                ]),
            }
            .get_region_vec(),
            "Taiwan",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "TZ"),
                names: Some(vec!["tanzania"]),
                demonyms: None,
                enterprises: None,
                misc: Some(vec!["chama cha mapinduzi"]),
            }
            .get_region_vec(),
            "Tanzania",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "UA"),
                names: Some(vec!["ukrain"]),
                demonyms: None,
                enterprises: None,
                misc: Some(vec!["verkhovna rada", "zelensky"]),
            }
            .get_region_vec(),
            "Ukraine",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "UG"),
                names: Some(vec!["uganda"]),
                demonyms: None,
                enterprises: None,
                misc: None,
            }
            .get_region_vec(),
            "Uganda",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "UM"),
                names: Some(vec![
                    "united states minor outlying islands",
                    "baker island",
                    "howland island",
                    "jarvis island",
                    "johnston atoll",
                    "kingman reef",
                    "midway atoll",
                    "palmyra atoll",
                    "wake island",
                    "navassa island",
                ]),
                demonyms: None,
                enterprises: None,
                misc: None,
            }
            .get_region_vec(),
            "United States Minor Outlying Islands",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "US"),
                names: Some(vec!["usa", "u.s.a."]),
                demonyms: None,
                enterprises: Some(vec![
                    "microsoft",
                    "apple inc",
                    "nvidia",
                    "alphabet inc",
                    "amazon inc",
                    "meta platforms",
                    "berksire hathaway",
                    "eli lilly",
                    "broadcom",
                    "jpmorgan chase",
                    "visa inc",
                    "tesla",
                    "exxon mobil",
                    "walmart",
                    "unitedhealth",
                    "mastercard",
                    "proctor & gamble",
                    "johnson & johnson",
                    "costco",
                    "home depot",
                    "oracle inc",
                    "merck",
                    "bank of america",
                    "chevron",
                    "abbvie",
                    "salesforce",
                    "coca-cola",
                    "netflix",
                    "amd",
                    "pepsico",
                    "thermo fisher",
                    "adobe",
                    "qualcomm",
                    "wells fargo",
                    "danaher",
                    "mcdonald's",
                    "cisco",
                    "t-mobile",
                    "walt disney",
                    "intuit ",
                    "abbott lab",
                    "texas instruments",
                    "applied materials inc",
                    "general electric",
                    "american express",
                    "caterpillar inc",
                    "verizon",
                    "amgen",
                    "morgan stanley",
                    "pfizer",
                    "servicenow",
                    "nextera energy",
                    "ibm",
                    "philip morris",
                    "comcast",
                    "goldman sachs",
                    "union pacific corp",
                    "charles schwab",
                    "conocophillips",
                    "intuitive surgical",
                    "nike",
                    "micron technology",
                    "raytheon",
                    "s&p global",
                    "uber inc",
                    "intel inc",
                    "honeywell",
                    "lowe's",
                    "ups",
                    "stryker corp",
                    "elevance health",
                    "booking holding",
                    "booking.com",
                    "at&t",
                    "progressive inc",
                    "citigroup",
                    "blackrock",
                    "lam research",
                    "vertex pharma",
                    "tjx co",
                    "boeing",
                    "lockheed martin",
                    "deere",
                    "boston scientific",
                    "regeneron pharma",
                    "dell",
                    "analog devices inc",
                    "marsh & mclennan",
                    "automatic data processing inc",
                    "prologis",
                    "palo alto",
                    "kla",
                    "arista networks",
                    "southern copper inc",
                    "kkr",
                    "cigna",
                    "mondelez",
                    "airbnb",
                    "fiserv",
                    "american tower inc",
                    "blackstone",
                    "bristol-meyers",
                    "chipotle",
                    "starbucks",
                    "southern company inc",
                    "synopsys",
                    "hca health",
                    "waste management inc",
                    "gilead science",
                    "crowdstrike",
                    "general dynamics",
                    "duke energy",
                    "zoetis",
                    "intercontinental exchange inc",
                    "amphenol",
                    "sherwin-williams",
                    "altria group",
                    "cadence design",
                    "freeport-mcmoran",
                    "colgate-palmolive",
                    "cme group",
                    "equinix",
                    "moody's",
                    "illinois tool works",
                    "eog resources",
                    "target inc",
                    "mckesson",
                    "cvs",
                    "transdigm",
                    "cintas",
                    "parker-hannifin",
                    "northrop",
                    "schlumberger",
                    "workday",
                    "becton dickinson",
                    "marriott",
                    "paypal",
                    "constellation energy",
                    "ecolab",
                    "csx corp",
                    "bancorp",
                    "emerson inc",
                    "apollo global",
                    "pnc financial",
                    "fedex",
                    "marathon petro",
                    "pioneer natural resources",
                    "phillips 66",
                    "marvell tech",
                    "enterprise products inc",
                    "motorola",
                    "welltower",
                    "o'reilly auto",
                    "republic services inc",
                    "carrier inc",
                    "air products and chemicals inc",
                    "3m",
                    "roper tech",
                    "monster beverage",
                    "arthur j. gallagher",
                    "occidental petro",
                    "simon property",
                    "paccar",
                    "valero",
                    "capital one",
                    "snowflake inc",
                    "energy transfer partners inc",
                    "edwards lifesciences",
                    "truist financial",
                    "american international group",
                    "metlife",
                    "copart",
                    "norfolk southern",
                    "dexcom",
                    "general motors",
                    "supermicro",
                    "interactive brokers inc",
                    "hilton world",
                    "coinbase",
                    "microchip technology inc",
                    "moderna",
                    "public storage inc",
                    "autozone",
                    "newmont",
                    "the travelers companies",
                    "williams companies",
                    "aflac",
                    "d. r. horton",
                    "sempra",
                    "american electric power",
                    "ford",
                    "hess",
                    "pacific gas and electric",
                    "palantir",
                    "estee lauder",
                    "oneok",
                    "doordash",
                    "realty income inc",
                    "autodesk",
                    "fortinet",
                    "constellation brands",
                    "w. w. grainger",
                    "the trade desk inc",
                    "united rentals",
                    "keurig",
                    "dr pepper",
                    "lennar inc",
                    "paychex",
                    "kimberly-clark",
                    "agilent tech",
                    "ares management",
                    "idexx lab",
                    "dominion energy",
                    "allstate",
                    "crown castle",
                    "block inc",
                    "bank of new york mellon",
                    "ross stores",
                    "cencora",
                    "kinder morgan",
                    "kraft",
                    "heinz",
                    "fidelity national",
                    "prudential financial",
                    "waste connections inc",
                    "ameriprise financial",
                    "humana",
                    "l3harris",
                    "iqvia",
                    "hershey",
                    "centene",
                    "dow inc",
                    "grayscale bitcoin",
                    "mplx",
                    "nucor",
                    "general mills",
                    "datadog",
                    "msci",
                    "yum! brands",
                    "old dominion freight",
                    "kroger",
                    "corteva",
                    "charter comm",
                    "kenvue",
                    "otis world",
                    "cummins",
                    "quanta services",
                    "ametek",
                    "exelon corp",
                    "fastenal",
                    "sysco",
                    "ge health",
                    "pseg",
                    "cheniere",
                    "royal caribbean",
                    "vertiv",
                    "nasdaq",
                    "verisk",
                    "martin marietta",
                    "costar group",
                    "monolithic power systems inc",
                    "diamondback energy",
                    "las vegas sands",
                    "gartner inc",
                    "fico",
                    "xylem",
                    "vulcan materials",
                    "cognizant technology solutions",
                    "electronic arts",
                    "delta air",
                    "veeva",
                    "howmet aero",
                    "bakar hughes",
                    "consolidated edison",
                    "biogen inc",
                    "halliburton",
                    "extra space storage inc",
                    "dupont de nemours",
                    "lyondellbasell",
                    "vistra",
                    "mettler-toledo",
                    "resmed",
                    "vici properties",
                    "ppg industries",
                    "on semiconductor inc",
                    "discover financial",
                    "devon energy",
                    "hubspot",
                    "dollar general",
                    "xcel energy",
                    "tractor supply",
                    "rockwell auto",
                    "equifax",
                    "hp",
                    "the hartford",
                    "archer daniels",
                    "corning",
                    "cdw corp",
                    "globalfoundries",
                    "wabtec",
                    "edison international",
                    "pinterest",
                    "ansys",
                    "avalonbay",
                    "microstrategy",
                    "rocket companies",
                    "cbre group",
                    "global payments inc",
                    "keysight",
                    "fortive",
                    "blue owl capital",
                    "applovin",
                    "mongodb",
                    "wec energy",
                    "zscaler",
                    "splunk",
                    "fifth third bank",
                    "snap inc",
                    "heico",
                    "raymond james",
                    "targa resources",
                    "t. rowe price",
                    "ebay",
                    "american water works inc",
                    "west pharma",
                    "church & dwight",
                    "symbiotic inc",
                    "m&t bank",
                    "brown & brown",
                    "dollar tree",
                    "cloudflare",
                    "first citizens banc",
                    "international flavors & fragrances",
                    "equity residential",
                    "dover",
                    "take 2 interactive",
                    "pultegroup",
                    "zimmer biomet",
                    "tradeweb",
                    "entergy",
                    "cardinal health",
                    "dte energy",
                    "broadridge financial",
                    "nvr",
                    "iron mountain",
                    "cheniere energy",
                    "western digital inc",
                    "state street corp",
                    "hewlett packard",
                    "brown forman",
                    "firstenergy",
                    "deckers brands",
                    "netapp",
                    "weyerhaeuser",
                    "samsara",
                    "live nation inc",
                    "rollins",
                    "ptc",
                    "ppl",
                    "axon enterprise",
                    "fleetcor",
                    "ball corp",
                    "alexandria real estate",
                    "invitation homes",
                    "celsius holding",
                    "markel",
                    "eversource",
                    "tyson foods",
                    "sba comm",
                    "genuine parts co",
                    "first solar inc",
                    "waters corp",
                    "hubbell",
                    "roblox",
                    "draftkings",
                    "kellogg",
                    "steel dynamics inc",
                    "coterra",
                    "carvana",
                    "tyler tech",
                    "erie indemnity",
                    "huntington banc",
                    "teradyne",
                    "freddie mac",
                    "align tech",
                    "builders firstsource",
                    "molina health",
                    "westlake chem",
                    "w. r. berkley",
                    "leidos",
                    "lpl financial",
                    "principal inc",
                    "ameren",
                    "zoom",
                    "hormel foods",
                    "williams-sonoma",
                    "mccormick",
                    "carlisle companies",
                    "ventas",
                    "booz allen",
                    "carnival corporation inc",
                    "entegris",
                    "warner bros",
                    "cooper companies",
                    "cboe",
                    "ulta",
                    "teledyne",
                    "centerpoint",
                    "pure storage inc",
                    "godaddy",
                    "watsco",
                    "corebridge",
                    "alnylam pharma",
                    "cms energy",
                    "omnicom",
                    "cincinnati financial",
                    "regions financial",
                    "darden restaurants",
                    "avery dennison",
                    "eqt corp",
                    "united airlines",
                    "baxter",
                    "atmos energy",
                    "domino's",
                    "emcor",
                    "labcorp",
                    "essex property",
                    "illumina inc",
                    "robinhood",
                    "synchrony",
                    "hologic",
                    "northern trust inc",
                    "lennox",
                    "okta",
                    "loews corp",
                    "celanese",
                    "abiomed",
                    "nutanix",
                    "nrg energy",
                    "reliance steel",
                    "factset",
                    "jacobs engineering",
                    "j. b. hunt",
                    "verisign",
                    "textron",
                    "avantor",
                    "bentley systems",
                    "citizens financial group",
                    "clorox",
                    "idex",
                    "formula one",
                    "southwest airlines",
                    "expeditors inc",
                    "warner music",
                    "mid-america apartment communities inc",
                    "packaging corporation of america",
                    "zebra tech",
                    "quest diagnostics",
                    "dick's sporting",
                    "sun communities",
                    "best buy inc",
                    "ss&c tech",
                    "walgreens",
                    "gen digital",
                    "tpg capital",
                    "enphase energy",
                    "nordson",
                    "carlyle",
                    "masco",
                    "albemarie",
                    "amh",
                    "american homes 4 rent",
                    "owens corning",
                    "aes",
                    "news corp",
                    "expedia",
                    "transunion",
                    "hyatt",
                    "skyworks",
                    "toast inc",
                    "udr apartments",
                    "fox corp",
                    "marathon oil",
                    "biomarin pharma",
                    "snap-on inc",
                    "conagra",
                    "rpm international",
                    "bunge inc",
                    "keycorp",
                    "keybank",
                    "akamai",
                    "western midstream",
                    "neurocrine bio",
                    "dynatrace",
                    "international paper inc",
                    "ryan specialty",
                    "manhattan associates",
                    "poolcorp",
                    "aspentech",
                    "graco",
                    "texas pacific land trust",
                    "physicians realty",
                    "reinsurance group of america",
                    "trimble",
                    "cf industries",
                    "jabil",
                    "black & decker",
                    "avangrid",
                    "campbell soup",
                    "westrock",
                    "toll brothers",
                    "revvity",
                    "us foods inc",
                    "advanced drainage systems inc",
                    "alliant energy",
                    "permian resources",
                    "ovintiv",
                    "equitable holding inc",
                    "bio-techne",
                    "host hotels & resorts",
                    "w. p. carey",
                    "insulet",
                    "nisource",
                    "viatris",
                    "natera",
                    "amerco",
                    "kimco realty",
                    "ares hospital",
                    "lincoln electric",
                    "mgm resorts",
                    "topbuild",
                    "incyte",
                    "xpo logistics",
                    "morningstar",
                    "franklin resources",
                    "floor & decor inc",
                    "evergy",
                    "equity lifestyle",
                    "karuna",
                    "a. o. smith",
                    "tenet health",
                    "lamb western",
                    "gaming and leisure properties",
                    "sarepta",
                    "casey's general",
                    "shockwave",
                    "burlington",
                    "docusign",
                    "jack henry",
                    "cna financial",
                    "davita",
                    "lamar advertising",
                    "smucker",
                    "aecom",
                    "ally inc",
                    "medspace",
                    "plains all american pipeline",
                    "united therapeutics",
                    "core & main",
                    "interpublic",
                    "chesapeake energy",
                    "molson coors",
                    "lkq corp",
                    "albertsons",
                    "universal health services inc",
                    "eastman chem",
                    "tetra tech",
                    "uipath",
                    "sirius xm",
                    "performance food",
                    "clean harbors inc",
                    "itt",
                    "apache corp",
                    "carmax",
                    "uwm holding",
                    "charles river lab",
                    "camden property",
                    "wingstop",
                    "texas roadhouse",
                    "regency centers",
                    "comfort systems inc",
                    "astera lab",
                    "juniper networks",
                    "sinclair",
                    "bath & body works",
                    "pershing square",
                    "american financial group inc",
                    "boston properties inc",
                    "elastic nv",
                    "onto innovation",
                    "woodward",
                    "bruker",
                    "zoominfo",
                    "epam systems",
                    "antero resources",
                    "essential utilities inc",
                    "wynn resorts",
                    "td synnex",
                    "east west bancorp",
                    "ralph lauren",
                    "curtiss-wright",
                    "twilio",
                    "regal rexnord",
                    "bj's wholesale",
                    "paycom",
                    "saia",
                    "affirm inc",
                    "rivian",
                    "penske auto",
                    "skechers",
                    "sharkninja",
                    "zillow",
                    "rexford industrial",
                    "service corporation international",
                    "crown holding",
                    "teleflex",
                    "confluent inc",
                    "guidewire",
                    "f5",
                    "annaly capital",
                    "procore",
                    "reddit",
                    "huntington ingalls",
                    "unum",
                    "cubesmart",
                    "lattice semiconductor",
                    "jefferies financial",
                    "catalent",
                ]),
                misc: None,
            }
            .get_region_vec(),
            "United States",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "UY"),
                names: Some(vec!["uruguay"]),
                demonyms: None,
                enterprises: None,
                misc: Some(vec!["biden", "nuland", "blinken"]),
            }
            .get_region_vec(),
            "Uruguay",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "UZ"),
                names: Some(vec!["uzbekistan"]),
                demonyms: None,
                enterprises: None,
                misc: Some(vec!["justice social democratic party"]),
            }
            .get_region_vec(),
            "Uzbekistan",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "VA"),
                names: None,
                demonyms: Some(vec!["vatican"]),
                enterprises: None,
                misc: Some(vec!["college of cardinals", "pope"]),
            }
            .get_region_vec(),
            "Vatican City",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "VC"),
                names: Some(vec!["saint vincent", "grenadines"]),
                demonyms: Some(vec!["vincentian", "grenadian", "vincy"]),
                enterprises: None,
                misc: None,
            }
            .get_region_vec(),
            "Saint Vincent and the Grenadines",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "VE"),
                names: Some(vec!["venezuela"]),
                demonyms: None,
                enterprises: None,
                misc: Some(vec!["psuv"]),
            }
            .get_region_vec(),
            "Venezuela",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "VG"),
                names: Some(vec!["british virgin islands"]),
                demonyms: None,
                enterprises: None,
                misc: None,
            }
            .get_region_vec(),
            "British Virgin Islands",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "VI"),
                names: Some(vec![
                    "united states virgin islands",
                    "us virgin islands",
                    "u.s. virgin islands",
                ]),
                demonyms: None,
                enterprises: None,
                misc: None,
            }
            .get_region_vec(),
            "United States Virgin Islands",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "VN"),
                names: None,
                demonyms: Some(vec!["viet"]),
                enterprises: None,
                misc: None,
            }
            .get_region_vec(),
            "Vietnam",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "VU"),
                names: Some(vec!["vanua"]),
                demonyms: None,
                enterprises: None,
                misc: None,
            }
            .get_region_vec(),
            "Vanuatu",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "WF"),
                names: Some(vec!["wallis", "futuna"]),
                demonyms: None,
                enterprises: None,
                misc: None,
            }
            .get_region_vec(),
            "Wallis and Futuna",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "WS"),
                names: None,
                demonyms: None,
                enterprises: None,
                misc: None,
            }
            .get_region_vec(),
            "Samoa",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "YE"),
                names: Some(vec!["yemen"]),
                demonyms: None,
                enterprises: None,
                misc: Some(vec!["houthi"]),
            }
            .get_region_vec(),
            "Yemen",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "YT"),
                names: Some(vec!["mayotte"]),
                demonyms: Some(vec!["mahoran", "mahorais"]),
                enterprises: None,
                misc: None,
            }
            .get_region_vec(),
            "Mayotte",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "XK"),
                names: Some(vec!["kosov"]),
                demonyms: None,
                enterprises: None,
                misc: Some(vec!["vetevendosje", "guxo"]),
            }
            .get_region_vec(),
            "Kosovo",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "ZA"),
                names: Some(vec!["south africa"]),
                demonyms: None,
                enterprises: Some(vec![
                    "naspers",
                    "firstrand",
                    "standard bank group inc",
                    "gold fields inc",
                    "capitec",
                    "anglogold",
                    "vodacom",
                ]),
                misc: Some(vec!["african national congress"]),
            }
            .get_region_vec(),
            "South Africa",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "ZM"),
                names: Some(vec!["zambia"]),
                demonyms: None,
                enterprises: None,
                misc: Some(vec!["upnd"]),
            }
            .get_region_vec(),
            "Zambia",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "ZW"),
                names: Some(vec!["zimbabwe"]),
                demonyms: Some(vec!["zimbo"]),
                enterprises: None,
                misc: Some(vec!["zanu-pf", "citizens coalition for change"]),
            }
            .get_region_vec(),
            "Zimbabwe",
        ),
    ];

    let blacklist: HashSet<&'static str> = vec![
        "chad",
        "georgia",
        "jordan",
        "turkey",
        "north east",
        "north west",
        "south east",
        "south west",
        "north central",
        "south central",
        "east central",
        "west central",
        "northern coast",
        "eastern coast",
        "southern coast",
        "western coast",
        "central coast",
        "north coast",
        "east coast",
        "south coast",
        "west coast",
        "northern province",
        "eastern province",
        "southern province",
        "western province",
        "central province",
        "north province",
        "east province",
        "south province",
        "west province",
        "centre province",
        "northern region",
        "eastern region",
        "southern region",
        "western region",
        "central region",
        "north region",
        "east region",
        "south region",
        "west region",
        "centre region",
        "northern territory",
        "eastern territory",
        "southern territory",
        "western territory",
        "central territory",
        "north territory",
        "east territory",
        "south territory",
        "west territory",
        "centre territory",
        "northern island",
        "eastern island",
        "southern island",
        "western island",
        "central island",
        "north island",
        "east island",
        "south island",
        "west island",
        "centre island",
        "arges",
        "gard",
        "georgetown",
        "saint john's",
        "st. john's",
        "stanley",
        "wien",
    ]
    .into_par_iter()
    .collect();

    remove_ambiguities(map, blacklist)
});

#[allow(dead_code)]
pub async fn show_region_map() -> Result<()> {
    let mut regions_iter = KEYPHRASE_REGION_MAP.iter();
    let mut current_region = regions_iter.next();
    let mut next_region = regions_iter.next();
    let mut input = String::new();
    while let Some(region) = current_region {
        tracing::info!("Region: {}", region.1);
        tracing::info!("Keyphrases: {:?}", region.0);
        tracing::info!("Press Enter to go to the next region");
        stdin().read_line(&mut input)?;
        current_region = next_region;
        next_region = regions_iter.next();
    }

    tracing::info!("Finished showing all regions");

    Ok(())
}
