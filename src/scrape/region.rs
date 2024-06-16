use crate::prelude::*;
use crate::{db::util::get_db_pool, service::var_service::get_docker_volume};
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
                if *s == "bid" {
                    return;
                }
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
    let pool = get_db_pool(db_path).await?;
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
            .flat_map(|s| {
                // For automatically generated keyphrases that are also subwords, add spaces around them.
                let mut results = Vec::new();
                for s in s.split(',') {
                    let s = s.trim().to_string();
                    match s.as_str() {
                        "acre" | "arges" | "gard" | "marche" | "teni" | "wien" => {
                            results.push(" ".to_owned() + &s.clone() + " ");
                        }
                        _ => results.push(s.clone()),
                    }
                }
                results
            })
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
    //         let count = map.entry(s).or_insert(0);
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
                automated: get_automated_keyphrases(&region_map, "ad"),
                names: Some(vec!["andorra"]),
                demonyms: None,
                enterprises: None,
                misc: Some(vec!["general syndic", "council of the valleys"]),
            }
            .get_region_vec(),
            "ad",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "ae"),
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
            "ae",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "af"),
                names: None,
                demonyms: Some(vec!["afghan"]),
                enterprises: None,
                misc: Some(vec!["taliban"]),
            }
            .get_region_vec(),
            "af",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "ag"),
                names: Some(vec!["antigua", "barbuda", "a&b"]),
                demonyms: None,
                enterprises: None,
                misc: Some(vec!["ablp", "united progressive party"]),
            }
            .get_region_vec(),
            "ag",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "ai"),
                names: Some(vec!["anguilla"]),
                demonyms: None,
                enterprises: None,
                misc: None,
            }
            .get_region_vec(),
            "ai",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "al"),
                names: Some(vec!["albania"]),
                demonyms: None,
                enterprises: None,
                misc: Some(vec!["kuvendi"]),
            }
            .get_region_vec(),
            "al",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "am"),
                names: Some(vec![
                    "armenia ",
                    "armenia'",
                    "armenia\"",
                    "armenia.",
                    "armenia,",
                ]),
                demonyms: Some(vec!["armenian"]),
                enterprises: None,
                misc: Some(vec!["azgayin zhoghov"]),
            }
            .get_region_vec(),
            "am",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "ao"),
                names: Some(vec!["angola"]),
                demonyms: None,
                enterprises: None,
                misc: Some(vec![" mpla ", "unita"]),
            }
            .get_region_vec(),
            "ao",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "aq"),
                names: Some(vec!["antarctica"]),
                demonyms: None,
                enterprises: None,
                misc: Some(vec!["mcmurdo"]),
            }
            .get_region_vec(),
            "aq",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "ar"),
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
            "ar",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "as"),
                names: Some(vec!["american samoa"]),
                demonyms: None,
                enterprises: None,
                misc: None,
            }
            .get_region_vec(),
            "as",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "at"),
                names: Some(vec!["austria", "oesterreich"]),
                demonyms: None,
                enterprises: Some(vec!["verbund", "erste group", "erste bank", "omv"]),
                misc: None,
            }
            .get_region_vec(),
            "at",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "au"),
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
            "au",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "aw"),
                names: Some(vec!["aruba"]),
                demonyms: None,
                enterprises: None,
                misc: None,
            }
            .get_region_vec(),
            "aw",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "ax"),
                names: Some(vec!["aland"]),
                demonyms: None,
                enterprises: None,
                misc: None,
            }
            .get_region_vec(),
            "ax",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "az"),
                names: Some(vec!["azerbaijan"]),
                demonyms: Some(vec!["azeri"]),
                enterprises: None,
                misc: Some(vec!["milli majlis", "democratic reforms party"]),
            }
            .get_region_vec(),
            "az",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "ba"),
                names: Some(vec!["bosnia", "srpska", "brcko"]),
                demonyms: Some(vec!["herzegovin"]),
                enterprises: None,
                misc: Some(vec![
                    "alliance of independent social democrats",
                    "party of democratic action",
                ]),
            }
            .get_region_vec(),
            "ba",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "bb"),
                names: Some(vec!["barbados"]),
                demonyms: None,
                enterprises: None,
                misc: None,
            }
            .get_region_vec(),
            "bb",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "bd"),
                names: Some(vec!["bangladesh"]),
                demonyms: None,
                enterprises: None,
                misc: Some(vec!["jatiya sangsad", "awami league", "jatiya party"]),
            }
            .get_region_vec(),
            "bd",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "be"),
                names: Some(vec!["belgium"]),
                demonyms: Some(vec!["belgian"]),
                enterprises: Some(vec!["anheuser-busch", "kbc", "ucb", "d'leteren", "gbl"]),
                misc: Some(vec!["flemish", "walloon"]),
            }
            .get_region_vec(),
            "be",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "bf"),
                names: Some(vec!["burkina faso"]),
                demonyms: Some(vec!["burkinabe", "burkinese"]),
                enterprises: None,
                misc: Some(vec!["mpsr"]),
            }
            .get_region_vec(),
            "bf",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "bg"),
                names: Some(vec!["bulgaria"]),
                demonyms: None,
                enterprises: None,
                misc: Some(vec!["narodno sabranie", "gerb"]),
            }
            .get_region_vec(),
            "bg",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "bh"),
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
            "bh",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "bi"),
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
            "bi",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "bj"),
                names: Some(vec!["benin"]),
                demonyms: None,
                enterprises: None,
                misc: Some(vec!["progressive union for renewal"]),
            }
            .get_region_vec(),
            "bj",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "bl"),
                names: Some(vec!["saint barthelemy"]),
                demonyms: Some(vec!["barthelemois"]),
                enterprises: None,
                misc: None,
            }
            .get_region_vec(),
            "bl",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "bm"),
                names: Some(vec!["bermuda"]),
                demonyms: None,
                enterprises: None,
                misc: None,
            }
            .get_region_vec(),
            "bm",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "bn"),
                names: None, // Name comes from database.
                demonyms: None,
                enterprises: None,
                misc: None,
            }
            .get_region_vec(),
            "bn",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "bo"),
                names: Some(vec!["bolivia"]),
                demonyms: None,
                enterprises: None,
                misc: Some(vec!["pluritonal", "plaza murillo"]),
            }
            .get_region_vec(),
            "bo",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "bq"),
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
            "bq",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "br"),
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
            "br",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "bs"),
                names: Some(vec!["bahama"]),
                demonyms: Some(vec!["bahamian"]),
                enterprises: None,
                misc: Some(vec!["progressive liberal party", "free national movement"]),
            }
            .get_region_vec(),
            "bs",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "bt"),
                names: Some(vec!["bhutan"]),
                demonyms: None,
                enterprises: None,
                misc: Some(vec!["druk gyalpo"]),
            }
            .get_region_vec(),
            "bt",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "bv"),
                names: Some(vec!["bouvet"]),
                demonyms: None,
                enterprises: None,
                misc: None,
            }
            .get_region_vec(),
            "bv",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "bw"),
                names: Some(vec!["botswana"]),
                demonyms: Some(vec!["batswana", "motswana"]),
                enterprises: None,
                misc: Some(vec!["umbrella for democratic change"]),
            }
            .get_region_vec(),
            "bw",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "by"),
                names: Some(vec!["belarus"]),
                demonyms: None,
                enterprises: None,
                misc: Some(vec!["belaya rus", "ldpb"]),
            }
            .get_region_vec(),
            "by",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "bz"),
                names: Some(vec!["belize"]),
                demonyms: None,
                enterprises: None,
                misc: Some(vec!["people's united party"]),
            }
            .get_region_vec(),
            "bz",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "ca"),
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
            "ca",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "cc"),
                names: Some(vec!["cocos island", "keeling island"]),
                demonyms: None,
                enterprises: None,
                misc: None,
            }
            .get_region_vec(),
            "cc",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "cd"),
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
            "cd",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "cf"),
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
            "cf",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "cg"),
                names: Some(vec!["little congo"]),
                demonyms: None,
                enterprises: None,
                misc: Some(vec!["congolese party of labour", "upads"]),
            }
            .get_region_vec(),
            "cg",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "ch"),
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
            "ch",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "ci"),
                names: Some(vec!["ivory coast", "cote d'ivoire"]),
                demonyms: Some(vec!["ivorian"]),
                enterprises: None,
                misc: None,
            }
            .get_region_vec(),
            "ci",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "ck"),
                names: Some(vec!["cook island"]),
                demonyms: None,
                enterprises: None,
                misc: None,
            }
            .get_region_vec(),
            "ck",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "cl"),
                names: Some(vec!["chile"]),
                demonyms: None,
                enterprises: Some(vec!["quimica y minera", "enel americas", "empresas copec"]),
                misc: None,
            }
            .get_region_vec(),
            "cl",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "cm"),
                names: Some(vec!["cameroon"]),
                demonyms: None,
                enterprises: None,
                misc: Some(vec!["unity palace", "rdpc", "ambazonia"]),
            }
            .get_region_vec(),
            "cm",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "cn"),
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
                    "longi, ",
                    "longi.",
                    "longi'",
                    "longi\"",
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
            "cn",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "co"),
                names: Some(vec!["colombia"]),
                demonyms: None,
                enterprises: None,
                misc: Some(vec!["casa de narino", "capitolio nacional", "eln"]),
            }
            .get_region_vec(),
            "co",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "cr"),
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
            "cr",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "cu"),
                names: Some(vec!["cuba"]),
                demonyms: None,
                enterprises: None,
                misc: Some(vec!["national assembly of people's power"]),
            }
            .get_region_vec(),
            "cu",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "cv"),
                names: Some(vec!["cape verde"]),
                demonyms: Some(vec!["cabo verdean"]),
                enterprises: None,
                misc: Some(vec!["paicv"]),
            }
            .get_region_vec(),
            "cv",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "cw"),
                names: Some(vec!["curacao"]),
                demonyms: None,
                enterprises: None,
                misc: Some(vec!["mfk", "real alternative party"]),
            }
            .get_region_vec(),
            "cw",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "cx"),
                names: Some(vec!["christmas island"]),
                demonyms: None,
                enterprises: None,
                misc: None,
            }
            .get_region_vec(),
            "cx",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "cy"),
                names: Some(vec!["cyprus"]),
                demonyms: Some(vec!["cypriot"]),
                enterprises: None,
                misc: Some(vec!["akel"]),
            }
            .get_region_vec(),
            "cy",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "cz"),
                names: None,
                demonyms: Some(vec!["czech"]),
                enterprises: Some(vec!["cez"]),
                misc: Some(vec!["spolu", "ano 2011"]),
            }
            .get_region_vec(),
            "cz",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "de"),
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
            "de",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "dj"),
                names: Some(vec!["djibouti"]),
                demonyms: None,
                enterprises: None,
                misc: Some(vec!["union for the presidential majority"]),
            }
            .get_region_vec(),
            "dj",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "dk"),
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
            "dk",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "dm"),
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
            "dm",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "do"),
                names: Some(vec!["dominican republic"]),
                demonyms: Some(vec!["quisqueyan"]),
                enterprises: None,
                misc: None,
            }
            .get_region_vec(),
            "do",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "dz"),
                names: Some(vec!["algeria"]),
                demonyms: None,
                enterprises: None,
                misc: Some(vec!["algerie", "fln"]),
            }
            .get_region_vec(),
            "dz",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "ec"),
                names: Some(vec!["ecuador"]),
                demonyms: None,
                enterprises: None,
                misc: Some(vec!["union for hope"]),
            }
            .get_region_vec(),
            "ec",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "ee"),
                names: Some(vec!["estonia"]),
                demonyms: None,
                enterprises: None,
                misc: None,
            }
            .get_region_vec(),
            "ee",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "eg"),
                names: Some(vec!["egypt"]),
                demonyms: None,
                enterprises: None,
                misc: None,
            }
            .get_region_vec(),
            "eg",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "eh"),
                names: Some(vec!["western sahara"]),
                demonyms: None,
                enterprises: None,
                misc: Some(vec!["polisario"]),
            }
            .get_region_vec(),
            "eh",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "er"),
                names: Some(vec!["eritrea"]),
                demonyms: None,
                enterprises: None,
                misc: Some(vec!["pfdj"]),
            }
            .get_region_vec(),
            "er",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "es"),
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
            "es",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "et"),
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
            "et",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "fi"),
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
            "fi",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "fj"),
                names: Some(vec!["fiji"]),
                demonyms: None,
                enterprises: None,
                misc: None,
            }
            .get_region_vec(),
            "fj",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "fk"),
                names: Some(vec!["falkland", "malvinas"]),
                demonyms: None,
                enterprises: None,
                misc: None,
            }
            .get_region_vec(),
            "fk",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "fm"),
                names: Some(vec!["micronesia", "fsm"]),
                demonyms: None,
                enterprises: None,
                misc: None,
            }
            .get_region_vec(),
            "fm",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "fo"),
                names: Some(vec!["faroe island"]),
                demonyms: None,
                enterprises: None,
                misc: Some(vec!["logting"]),
            }
            .get_region_vec(),
            "fo",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "fr"),
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
                    "vinci inc",
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
            "fr",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "ga"),
                names: Some(vec!["gabon"]),
                demonyms: None,
                enterprises: None,
                misc: Some(vec!["ctri"]),
            }
            .get_region_vec(),
            "ga",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "gb"),
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
            "gb",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "gd"),
                names: Some(vec!["grenada"]),
                demonyms: Some(vec!["grenadian"]),
                enterprises: None,
                misc: Some(vec!["rgpf"]),
            }
            .get_region_vec(),
            "gd",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "ge"),
                names: None,
                demonyms: None,
                enterprises: None,
                misc: Some(vec!["abkhaz", "united national movement"]),
            }
            .get_region_vec(),
            "ge",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "gf"),
                names: Some(vec!["french guiana"]),
                demonyms: Some(vec!["french guianan", "french guinese"]),
                enterprises: None,
                misc: None,
            }
            .get_region_vec(),
            "gf",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "gg"),
                names: Some(vec!["guernsey"]),
                demonyms: Some(vec!["giernesiais"]),
                enterprises: None,
                misc: None,
            }
            .get_region_vec(),
            "gg",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "gh"),
                names: Some(vec!["ghana"]),
                demonyms: None,
                enterprises: None,
                misc: Some(vec!["national democratic congress", "new patriotic party"]),
            }
            .get_region_vec(),
            "gh",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "gi"),
                names: None, // Name comes from database.
                demonyms: Some(vec!["llanito"]),
                enterprises: None,
                misc: Some(vec!["gslp"]),
            }
            .get_region_vec(),
            "gi",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "gl"),
                names: Some(vec!["greenland"]),
                demonyms: None,
                enterprises: None,
                misc: Some(vec!["inuit ataqatigiit", "naleraq", "siumut"]),
            }
            .get_region_vec(),
            "gl",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "gm"),
                names: Some(vec!["gambia"]),
                demonyms: None,
                enterprises: None,
                misc: None,
            }
            .get_region_vec(),
            "gm",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "gn"),
                names: None,
                demonyms: None,
                enterprises: None,
                misc: Some(vec!["cnrd"]),
            }
            .get_region_vec(),
            "gn",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "gp"),
                names: Some(vec!["guadeloupe"]),
                demonyms: None,
                enterprises: None,
                misc: None,
            }
            .get_region_vec(),
            "gp",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "gq"),
                names: Some(vec!["equatorial guinea"]),
                demonyms: Some(vec!["equatoguinean"]),
                enterprises: None,
                misc: Some(vec!["pdge"]),
            }
            .get_region_vec(),
            "gq",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "gr"),
                names: Some(vec!["greece"]),
                demonyms: Some(vec!["greek"]),
                enterprises: None,
                misc: Some(vec!["helleni", "syriza"]),
            }
            .get_region_vec(),
            "gr",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "gs"),
                names: Some(vec!["south georgia", "south sandwich"]),
                demonyms: None,
                enterprises: None,
                misc: None,
            }
            .get_region_vec(),
            "gs",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "gt"),
                names: Some(vec!["guatemala"]),
                demonyms: Some(vec!["chapin"]),
                enterprises: None,
                misc: Some(vec!["semilla"]),
            }
            .get_region_vec(),
            "gt",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "gu"),
                names: Some(vec!["guam"]),
                demonyms: None,
                enterprises: None,
                misc: None,
            }
            .get_region_vec(),
            "gu",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "gw"),
                names: Some(vec!["guinea-bissau"]),
                demonyms: Some(vec!["bissau-guinean"]),
                enterprises: None,
                misc: Some(vec!["terra ranka", "paigc", "madem g15", "madem-g15"]),
            }
            .get_region_vec(),
            "gw",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "gy"),
                names: Some(vec!["guyan"]),
                demonyms: None,
                enterprises: None,
                misc: None,
            }
            .get_region_vec(),
            "gy",
        ),
        (
            RegionKeyphrases {
                // I am not including "... China ..." keyphrases for this region, as I value the 'China" keyphrase more for the China region.
                automated: get_automated_keyphrases(&region_map, "hk"),
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
            "hk",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "hm"),
                names: Some(vec!["heard island", "mcdonald island"]),
                demonyms: None,
                enterprises: None,
                misc: None,
            }
            .get_region_vec(),
            "hm",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "hn"),
                names: Some(vec!["hondura"]),
                demonyms: Some(vec!["catrach"]),
                enterprises: None,
                misc: Some(vec!["liberty and refoundation"]),
            }
            .get_region_vec(),
            "hn",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "hr"),
                names: Some(vec!["croatia"]),
                demonyms: None,
                enterprises: None,
                misc: Some(vec!["hdz"]),
            }
            .get_region_vec(),
            "hr",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "ht"),
                names: Some(vec!["haiti"]),
                demonyms: None,
                enterprises: None,
                misc: Some(vec!["phtk"]),
            }
            .get_region_vec(),
            "ht",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "hu"),
                names: Some(vec!["hungar"]),
                demonyms: None,
                enterprises: None,
                misc: Some(vec!["fidesz", "orban"]),
            }
            .get_region_vec(),
            "hu",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "id"),
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
            "id",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "ie"),
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
            "ie",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "il"),
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
                    "zionis",
                    "kibbutz",
                    "shin bet",
                ]),
            }
            .get_region_vec(),
            "il",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "im"),
                names: Some(vec!["isle of man"]),
                demonyms: Some(vec!["manx"]),
                enterprises: None,
                misc: Some(vec!["tynwald"]),
            }
            .get_region_vec(),
            "im",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "in"),
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
            "in",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "io"),
                names: Some(vec!["british indian ocean territory"]),
                demonyms: None,
                enterprises: None,
                misc: None,
            }
            .get_region_vec(),
            "io",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "iq"),
                names: Some(vec!["iraq"]),
                demonyms: None,
                enterprises: None,
                misc: Some(vec!["takadum", "emtidad"]),
            }
            .get_region_vec(),
            "iq",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "ir"),
                names: Some(vec!["iran ", "iran'", "iran\"", "iran.", "iran,"]),
                demonyms: Some(vec!["iranian"]),
                enterprises: None,
                misc: Some(vec!["guardian council", "khomeini"]),
            }
            .get_region_vec(),
            "ir",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "is"),
                names: Some(vec!["iceland"]),
                demonyms: None,
                enterprises: None,
                misc: Some(vec!["althing", "samfylkingin"]),
            }
            .get_region_vec(),
            "is",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "it"),
                names: Some(vec!["italy"]),
                demonyms: Some(vec!["italian"]),
                enterprises: Some(vec![
                    "ferrari",
                    "enel inc",
                    "intesa sanpaolo",
                    "unicredit",
                    "eni inc",
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
            "it",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "je"),
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
            "je",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "jm"),
                names: None, // Name comes from database.
                demonyms: None,
                enterprises: None,
                misc: None,
            }
            .get_region_vec(),
            "jm",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "jo"),
                names: None,
                demonyms: Some(vec!["jordanian"]),
                enterprises: None,
                misc: Some(vec!["islamic action front"]),
            }
            .get_region_vec(),
            "jo",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "jp"),
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
            "jp",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "ke"),
                names: Some(vec!["kenya"]),
                demonyms: None,
                enterprises: None,
                misc: Some(vec!["azimio"]),
            }
            .get_region_vec(),
            "ke",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "kg"),
                names: None,
                demonyms: Some(vec!["kyrgyz"]),
                enterprises: None,
                misc: Some(vec!["jogorku kenesh", "mekenchil", "eldik"]),
            }
            .get_region_vec(),
            "kg",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "kh"),
                names: Some(vec!["cambodia"]),
                demonyms: Some(vec!["khmer"]),
                enterprises: None,
                misc: Some(vec!["funcinpec"]),
            }
            .get_region_vec(),
            "kh",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "ki"),
                names: Some(vec!["kiribati"]),
                demonyms: None,
                enterprises: None,
                misc: None,
            }
            .get_region_vec(),
            "ki",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "km"),
                names: Some(vec!["comoros"]),
                demonyms: Some(vec!["comorian"]),
                enterprises: None,
                misc: Some(vec!["orange party"]),
            }
            .get_region_vec(),
            "km",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "kn"),
                names: Some(vec!["kitts", "nevis"]),
                demonyms: Some(vec!["kittitian", "nevisian"]),
                enterprises: None,
                misc: Some(vec!["concerned citizens' movement"]),
            }
            .get_region_vec(),
            "kn",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "kp"),
                names: Some(vec!["north korea"]),
                demonyms: None,
                enterprises: None,
                misc: Some(vec!["supreme people's assembly", "dprk"]),
            }
            .get_region_vec(),
            "kp",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "kr"),
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
            "kr",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "kw"),
                names: Some(vec!["kuwait"]),
                demonyms: None,
                enterprises: None,
                misc: None,
            }
            .get_region_vec(),
            "kw",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "ky"),
                names: Some(vec!["cayman"]),
                demonyms: None,
                enterprises: None,
                misc: None,
            }
            .get_region_vec(),
            "ky",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "kz"),
                names: None,
                demonyms: Some(vec!["kazakh"]),
                enterprises: None,
                misc: Some(vec!["mazhilis", "amanat", "auyl"]),
            }
            .get_region_vec(),
            "kz",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "la"),
                names: Some(vec!["laos"]),
                demonyms: Some(vec!["lao", "laotian"]), // Strings with length 3 or less are processed before substring checking.
                enterprises: None,
                misc: Some(vec!["lprp"]),
            }
            .get_region_vec(),
            "la",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "lb"),
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
            "lb",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "lc"),
                names: Some(vec!["saint lucia"]),
                demonyms: None,
                enterprises: None,
                misc: None,
            }
            .get_region_vec(),
            "lc",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "li"),
                names: Some(vec!["liechtenstein"]),
                demonyms: None,
                enterprises: None,
                misc: None,
            }
            .get_region_vec(),
            "li",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "lk"),
                names: Some(vec!["sri lanka"]),
                demonyms: None,
                enterprises: None,
                misc: Some(vec!["slpfa", "samagi jana balawegaya"]),
            }
            .get_region_vec(),
            "lk",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "lr"),
                names: Some(vec!["liberia"]),
                demonyms: None,
                enterprises: None,
                misc: Some(vec!["coalition for democratic change"]),
            }
            .get_region_vec(),
            "lr",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "ls"),
                names: None, // Name comes from database.
                demonyms: Some(vec!["mosotho", "basotho"]),
                enterprises: None,
                misc: Some(vec!["revolution for prosperity"]),
            }
            .get_region_vec(),
            "ls",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "lt"),
                names: Some(vec!["lithuania"]),
                demonyms: None,
                enterprises: None,
                misc: Some(vec!["seimas", "homeland union", "lvzs"]),
            }
            .get_region_vec(),
            "lt",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "lu"),
                names: None, // Name comes from database.
                demonyms: None,
                enterprises: Some(vec!["arcelormittal", "tenaris", "eurofins"]),
                misc: Some(vec!["christian social people's party", "lsap"]),
            }
            .get_region_vec(),
            "lu",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "lv"),
                names: Some(vec!["latvia"]),
                demonyms: None,
                enterprises: None,
                misc: Some(vec!["saeima", "zzs"]),
            }
            .get_region_vec(),
            "lv",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "ly"),
                names: None, // Name comes from database.
                demonyms: None,
                enterprises: None,
                misc: Some(vec!["government of national"]),
            }
            .get_region_vec(),
            "ly",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "ma"),
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
            "ma",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "mc"),
                names: None, // Name comes from database.
                demonyms: Some(vec!["monegasque", "monacan"]),
                enterprises: None,
                misc: None,
            }
            .get_region_vec(),
            "mc",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "md"),
                names: Some(vec!["moldova"]),
                demonyms: None,
                enterprises: None,
                misc: Some(vec!["party of action and solidarity", "psrm"]),
            }
            .get_region_vec(),
            "md",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "me"),
                names: Some(vec!["monteneg"]),
                demonyms: None,
                enterprises: None,
                misc: Some(vec!["pes!"]),
            }
            .get_region_vec(),
            "me",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "mf"),
                names: Some(vec!["saint martin"]),
                demonyms: None,
                enterprises: None,
                misc: None,
            }
            .get_region_vec(),
            "mf",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "mg"),
                names: Some(vec!["madagas"]),
                demonyms: Some(vec!["malagas"]),
                enterprises: None,
                misc: None,
            }
            .get_region_vec(),
            "mg",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "mh"),
                names: Some(vec!["marshall island"]),
                demonyms: Some(vec!["marshallese"]),
                enterprises: None,
                misc: None,
            }
            .get_region_vec(),
            "mh",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "mk"),
                names: Some(vec!["north macedonia"]),
                demonyms: None,
                enterprises: None,
                misc: Some(vec!["sobranie", "sdsm", "vmro-dpmne"]),
            }
            .get_region_vec(),
            "mk",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "ml"),
                names: Some(vec!["mali ", "mali'", "mali\"", "mali.", "mali,"]),
                demonyms: Some(vec!["malian ", "malian'", "malian\"", "malian.", "malian,"]),
                enterprises: None,
                misc: None,
            }
            .get_region_vec(),
            "ml",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "mm"),
                names: Some(vec!["myanma"]),
                demonyms: Some(vec!["burmese"]),
                enterprises: None,
                misc: Some(vec!["pyidaungsu hluttaw", "nld"]),
            }
            .get_region_vec(),
            "mm",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "mn"),
                names: Some(vec!["mongol"]),
                demonyms: None,
                enterprises: None,
                misc: Some(vec!["state great khural"]),
            }
            .get_region_vec(),
            "mn",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "mo"),
                names: Some(vec!["macau", "macao"]),
                demonyms: None,
                enterprises: None,
                misc: None,
            }
            .get_region_vec(),
            "mo",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "mp"),
                names: Some(vec!["northern mariana island"]),
                demonyms: Some(vec!["marianan", "chamorro"]),
                enterprises: None,
                misc: None,
            }
            .get_region_vec(),
            "mp",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "mq"),
                names: Some(vec!["martiniq"]),
                demonyms: Some(vec!["martinic"]),
                enterprises: None,
                misc: None,
            }
            .get_region_vec(),
            "mq",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "mr"),
                names: Some(vec!["mauritania"]),
                demonyms: None,
                enterprises: None,
                misc: Some(vec!["tewassoul"]),
            }
            .get_region_vec(),
            "mr",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "ms"),
                names: Some(vec!["montserrat"]),
                demonyms: None,
                enterprises: None,
                misc: Some(vec!["movement for change and prosperity"]),
            }
            .get_region_vec(),
            "ms",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "mt"),
                names: Some(vec!["malta"]),
                demonyms: Some(vec!["maltese"]),
                enterprises: None,
                misc: None,
            }
            .get_region_vec(),
            "mt",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "mu"),
                names: Some(vec!["mauriti"]),
                demonyms: None,
                enterprises: None,
                misc: Some(vec!["mauricien"]),
            }
            .get_region_vec(),
            "mu",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "mv"),
                names: Some(vec!["maldiv"]),
                demonyms: None,
                enterprises: None,
                misc: Some(vec!["people's majlis"]),
            }
            .get_region_vec(),
            "mv",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "mw"),
                names: Some(vec!["malawi"]),
                demonyms: None,
                enterprises: None,
                misc: None,
            }
            .get_region_vec(),
            "mw",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "mx"),
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
            "mx",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "my"),
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
            "my",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "mz"),
                names: Some(vec!["mozambi"]),
                demonyms: None,
                enterprises: None,
                misc: Some(vec!["frelimo", "renamo"]),
            }
            .get_region_vec(),
            "mz",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "na"),
                names: Some(vec!["namibia"]),
                demonyms: None,
                enterprises: None,
                misc: Some(vec!["swapo"]),
            }
            .get_region_vec(),
            "na",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "nc"),
                names: Some(vec!["new caledonia"]),
                demonyms: Some(vec!["caledonian"]),
                enterprises: None,
                misc: Some(vec!["flnks", "l'eo"]),
            }
            .get_region_vec(),
            "nc",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "ne"),
                names: None,
                demonyms: Some(vec!["nigerien"]),
                enterprises: None,
                misc: None,
            }
            .get_region_vec(),
            "ne",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "nf"),
                names: Some(vec!["norfolk island"]),
                demonyms: None,
                enterprises: None,
                misc: None,
            }
            .get_region_vec(),
            "nf",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "ng"),
                names: Some(vec!["nigeria"]),
                demonyms: None,
                enterprises: None,
                misc: Some(vec!["all progressives congress"]),
            }
            .get_region_vec(),
            "ng",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "ni"),
                names: Some(vec!["nicaragua"]),
                demonyms: Some(vec!["pinoler"]),
                enterprises: None,
                misc: Some(vec!["sandinista"]),
            }
            .get_region_vec(),
            "ni",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "nl"),
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
            "nl",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "no"),
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
            "no",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "np"),
                names: Some(vec!["nepal"]),
                demonyms: None,
                enterprises: None,
                misc: None,
            }
            .get_region_vec(),
            "np",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "nr"),
                names: Some(vec!["nauru"]),
                demonyms: None,
                enterprises: None,
                misc: None,
            }
            .get_region_vec(),
            "nr",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "nu"),
                names: Some(vec!["niue"]),
                demonyms: None,
                enterprises: None,
                misc: None,
            }
            .get_region_vec(),
            "nu",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "nz"),
                names: Some(vec!["new zealand"]),
                demonyms: Some(vec!["kiwi"]),
                enterprises: Some(vec!["xero", "fisher & paykel"]),
                misc: Some(vec!["parliament", "nzlp"]),
            }
            .get_region_vec(),
            "nz",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "om"),
                names: Some(vec!["oman"]),
                demonyms: None,
                enterprises: None,
                misc: None,
            }
            .get_region_vec(),
            "om",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "pa"),
                names: Some(vec!["panama"]),
                demonyms: None,
                enterprises: None,
                misc: Some(vec!["molirena"]),
            }
            .get_region_vec(),
            "pa",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "pe"),
                names: Some(vec!["peru"]),
                demonyms: None,
                enterprises: None,
                misc: Some(vec!["fujimoris"]),
            }
            .get_region_vec(),
            "pe",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "pf"),
                names: Some(vec!["french polynesia"]),
                demonyms: None,
                enterprises: None,
                misc: Some(vec!["tavini", "tapura"]),
            }
            .get_region_vec(),
            "pf",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "pg"),
                names: Some(vec!["papua new guinea"]),
                demonyms: None,
                enterprises: None,
                misc: Some(vec!["pangu pati"]),
            }
            .get_region_vec(),
            "pg",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "ph"),
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
            "ph",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "pk"),
                names: Some(vec!["pakistan"]),
                demonyms: None,
                enterprises: None,
                misc: Some(vec!["pml-n", "ittehad council"]),
            }
            .get_region_vec(),
            "pk",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "pl"),
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
            "pl",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "pm"),
                names: Some(vec!["saint pierre", "miquelon"]),
                demonyms: Some(vec!["saint-pierrais", "miquelonnais", "pierrian"]),
                enterprises: None,
                misc: Some(vec!["archipelago tomorrow"]),
            }
            .get_region_vec(),
            "pm",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "pn"),
                names: Some(vec!["pitcairn"]),
                demonyms: Some(vec!["pitkern"]),
                enterprises: None,
                misc: None,
            }
            .get_region_vec(),
            "pn",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "pr"),
                names: Some(vec!["puerto ric"]),
                demonyms: None,
                enterprises: None,
                misc: None,
            }
            .get_region_vec(),
            "pr",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "ps"),
                names: Some(vec!["palestin"]),
                demonyms: None,
                enterprises: None,
                misc: Some(vec![
                    "plo",
                    "hamas",
                    "fatah",
                    "gaza",
                    "rafah",
                    "khan yunis",
                    "khan younis",
                    "khan yunus",
                ]),
            }
            .get_region_vec(),
            "ps",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "pt"),
                names: Some(vec!["portugal"]),
                demonyms: Some(vec!["portuguese"]),
                enterprises: Some(vec!["edp group", "galp energ", "jeronimo martins"]),
                misc: None,
            }
            .get_region_vec(),
            "pt",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "pw"),
                names: Some(vec!["palau"]),
                demonyms: None,
                enterprises: None,
                misc: None,
            }
            .get_region_vec(),
            "pw",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "py"),
                names: Some(vec!["paraguay"]),
                demonyms: None,
                enterprises: None,
                misc: None,
            }
            .get_region_vec(),
            "py",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "qa"),
                names: Some(vec!["qatar"]),
                demonyms: None,
                enterprises: Some(vec!["qnb inc"]),
                misc: Some(vec!["house of thani"]),
            }
            .get_region_vec(),
            "qa",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "re"),
                names: None,
                demonyms: Some(vec!["reunionese"]),
                enterprises: None,
                misc: None,
            }
            .get_region_vec(),
            "re",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "ro"),
                names: Some(vec!["romania"]),
                demonyms: None,
                enterprises: None,
                misc: None,
            }
            .get_region_vec(),
            "ro",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "rs"),
                names: Some(vec!["serbia"]),
                demonyms: None,
                enterprises: None,
                misc: Some(vec!["av-zms", "sps-zs"]),
            }
            .get_region_vec(),
            "rs",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "ru"),
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
            "ru",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "rw"),
                names: Some(vec!["rwand"]),
                demonyms: None,
                enterprises: None,
                misc: None,
            }
            .get_region_vec(),
            "rw",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "sa"),
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
                misc: Some(vec!["mount arafat"]),
            }
            .get_region_vec(),
            "sa",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "sb"),
                names: Some(vec!["solomon island"]),
                demonyms: None,
                enterprises: None,
                misc: Some(vec!["kadere party"]),
            }
            .get_region_vec(),
            "sb",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "sc"),
                names: Some(vec!["seychell"]),
                demonyms: Some(vec!["seselwa"]),
                enterprises: None,
                misc: None,
            }
            .get_region_vec(),
            "sc",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "sd"),
                names: None,
                demonyms: None,
                enterprises: None,
                misc: None,
            }
            .get_region_vec(),
            "sd",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "se"),
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
            "se",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "sg"),
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
            "sg",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "sh"),
                names: Some(vec!["saint helen"]),
                demonyms: Some(vec!["helenian"]),
                enterprises: None,
                misc: None,
            }
            .get_region_vec(),
            "sh",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "si"),
                names: Some(vec!["sloven"]),
                demonyms: None,
                enterprises: None,
                misc: None,
            }
            .get_region_vec(),
            "si",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "sj"),
                names: Some(vec!["svalbard", "jan mayen"]),
                demonyms: None,
                enterprises: None,
                misc: None,
            }
            .get_region_vec(),
            "sj",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "sk"),
                names: None,
                demonyms: Some(vec!["slovak"]),
                enterprises: None,
                misc: Some(vec!["smer-sd", "hlas-sd"]),
            }
            .get_region_vec(),
            "sk",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "sl"),
                names: Some(vec!["sierra leone"]),
                demonyms: None,
                enterprises: None,
                misc: None,
            }
            .get_region_vec(),
            "sl",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "sm"),
                names: Some(vec!["san marino"]),
                demonyms: Some(vec!["sammarinese"]),
                enterprises: None,
                misc: None,
            }
            .get_region_vec(),
            "sm",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "sn"),
                names: Some(vec!["senegal"]),
                demonyms: None,
                enterprises: None,
                misc: None,
            }
            .get_region_vec(),
            "sn",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "so"),
                names: None,
                demonyms: Some(vec!["somali"]),
                enterprises: None,
                misc: None,
            }
            .get_region_vec(),
            "so",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "sr"),
                names: Some(vec!["suriname"]),
                demonyms: None,
                enterprises: None,
                misc: None,
            }
            .get_region_vec(),
            "sr",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "ss"),
                names: Some(vec!["south sudan"]),
                demonyms: None,
                enterprises: None,
                misc: Some(vec!["splm-in-opposition"]),
            }
            .get_region_vec(),
            "ss",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "st"),
                names: Some(vec!["sao tome", "principe"]),
                demonyms: Some(vec!["santomean"]),
                enterprises: None,
                misc: Some(vec!["mlstp"]),
            }
            .get_region_vec(),
            "st",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "sv"),
                names: Some(vec!["el salvador"]),
                demonyms: Some(vec!["salvadoran"]),
                enterprises: None,
                misc: Some(vec!["nuevas ideas"]),
            }
            .get_region_vec(),
            "sv",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "sx"),
                names: Some(vec!["maarten"]),
                demonyms: None,
                enterprises: None,
                misc: None,
            }
            .get_region_vec(),
            "sx",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "sy"),
                names: Some(vec!["syria"]),
                demonyms: None,
                enterprises: None,
                misc: None,
            }
            .get_region_vec(),
            "sy",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "sz"),
                names: Some(vec!["eswatini"]),
                demonyms: Some(vec!["swazi"]),
                enterprises: None,
                misc: Some(vec!["tinkhundla"]),
            }
            .get_region_vec(),
            "sz",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "tc"),
                names: Some(vec!["turks and c", "caicos"]),
                demonyms: Some(vec!["turks islander"]),
                enterprises: None,
                misc: None,
            }
            .get_region_vec(),
            "tc",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "td"),
                names: None,
                demonyms: Some(vec!["chadian"]),
                enterprises: None,
                misc: Some(vec!["national transitional council"]),
            }
            .get_region_vec(),
            "td",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "tf"),
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
            "tf",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "tg"),
                names: Some(vec!["togo"]),
                demonyms: None,
                enterprises: None,
                misc: Some(vec!["union of forces for change"]),
            }
            .get_region_vec(),
            "tg",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "th"),
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
            "th",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "tj"),
                names: None,
                demonyms: Some(vec!["tajik"]),
                enterprises: None,
                misc: None,
            }
            .get_region_vec(),
            "tj",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "tk"),
                names: Some(vec!["tokelau"]),
                demonyms: None,
                enterprises: None,
                misc: None,
            }
            .get_region_vec(),
            "tk",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "tl"),
                names: Some(vec!["timor-leste", "east timor"]),
                demonyms: Some(vec!["timorese"]),
                enterprises: None,
                misc: Some(vec!["national parliament", "cnrt", "fretilin"]),
            }
            .get_region_vec(),
            "tl",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "tm"),
                names: None,
                demonyms: Some(vec!["turkmen"]),
                enterprises: None,
                misc: None,
            }
            .get_region_vec(),
            "tm",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "tn"),
                names: Some(vec!["tunisia"]),
                demonyms: None,
                enterprises: None,
                misc: Some(vec![
                    "assembly of the representatives of the people",
                    "25th of july movement",
                ]),
            }
            .get_region_vec(),
            "tn",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "to"),
                names: Some(vec!["tonga"]),
                demonyms: None,
                enterprises: None,
                misc: None,
            }
            .get_region_vec(),
            "to",
        ),
        (
            RegionKeyphrases {
                // I did not add "Ford Otosan", as I value the 'Ford' keyphrase more for the United States region.
                automated: get_automated_keyphrases(&region_map, "tr"),
                names: Some(vec!["turkey", "turkiye"]),
                demonyms: Some(vec!["turkish"]),
                enterprises: Some(vec!["qnb finansbank", "koc", "garantibank", "akbank"]),
                misc: Some(vec!["grand national assembly"]),
            }
            .get_region_vec(),
            "tr",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "tt"),
                names: Some(vec!["tobago"]),
                demonyms: Some(vec!["trini", "trinbagonian"]),
                enterprises: None,
                misc: None,
            }
            .get_region_vec(),
            "tt",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "tv"),
                names: Some(vec!["tuvalu"]),
                demonyms: None,
                enterprises: None,
                misc: None,
            }
            .get_region_vec(),
            "tv",
        ),
        (
            RegionKeyphrases {
                // I am not including "China Steel", as I value the 'China" keyphrase more for the China region.
                automated: get_automated_keyphrases(&region_map, "tw"),
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
            "tw",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "tz"),
                names: Some(vec!["tanzania"]),
                demonyms: None,
                enterprises: None,
                misc: Some(vec!["chama cha mapinduzi"]),
            }
            .get_region_vec(),
            "tz",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "ua"),
                names: Some(vec!["ukrain"]),
                demonyms: None,
                enterprises: None,
                misc: Some(vec!["verkhovna rada", "zelensky", "azov"]),
            }
            .get_region_vec(),
            "ua",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "ug"),
                names: Some(vec!["uganda"]),
                demonyms: None,
                enterprises: None,
                misc: None,
            }
            .get_region_vec(),
            "ug",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "um"),
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
            "um",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "us"),
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
                misc: Some(vec![
                    "donald trump",
                    "medicaid",
                    "medicare",
                    "biden",
                    "nuland",
                    "blinken",
                    "nyc",
                    "wall street",
                    "world bank",
                    "ifc",
                    "leahy",
                ]),
            }
            .get_region_vec(),
            "us",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "uy"),
                names: Some(vec!["uruguay"]),
                demonyms: None,
                enterprises: None,
                misc: None,
            }
            .get_region_vec(),
            "uy",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "uz"),
                names: Some(vec!["uzbekistan"]),
                demonyms: None,
                enterprises: None,
                misc: Some(vec!["justice social democratic party"]),
            }
            .get_region_vec(),
            "uz",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "va"),
                names: None,
                demonyms: Some(vec!["vatican"]),
                enterprises: None,
                misc: Some(vec!["college of cardinals", "pope"]),
            }
            .get_region_vec(),
            "va",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "vc"),
                names: Some(vec!["saint vincent", "grenadines"]),
                demonyms: Some(vec!["vincentian", "grenadian", "vincy"]),
                enterprises: None,
                misc: None,
            }
            .get_region_vec(),
            "vc",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "ve"),
                names: Some(vec!["venezuela"]),
                demonyms: None,
                enterprises: None,
                misc: Some(vec!["psuv"]),
            }
            .get_region_vec(),
            "ve",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "vg"),
                names: Some(vec!["british virgin islands"]),
                demonyms: None,
                enterprises: None,
                misc: None,
            }
            .get_region_vec(),
            "vg",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "vi"),
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
            "vi",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "vn"),
                names: None,
                demonyms: Some(vec!["viet"]),
                enterprises: None,
                misc: None,
            }
            .get_region_vec(),
            "vn",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "vu"),
                names: Some(vec!["vanua"]),
                demonyms: None,
                enterprises: None,
                misc: None,
            }
            .get_region_vec(),
            "vu",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "wf"),
                names: Some(vec!["wallis", "futuna"]),
                demonyms: None,
                enterprises: None,
                misc: None,
            }
            .get_region_vec(),
            "wf",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "ws"),
                names: None,
                demonyms: None,
                enterprises: None,
                misc: None,
            }
            .get_region_vec(),
            "ws",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "ye"),
                names: Some(vec!["yemen"]),
                demonyms: None,
                enterprises: None,
                misc: Some(vec!["houthi"]),
            }
            .get_region_vec(),
            "ye",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "yt"),
                names: Some(vec!["mayotte"]),
                demonyms: Some(vec!["mahoran", "mahorais"]),
                enterprises: None,
                misc: None,
            }
            .get_region_vec(),
            "yt",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "xk"),
                names: Some(vec!["kosov"]),
                demonyms: None,
                enterprises: None,
                misc: Some(vec!["vetevendosje", "guxo"]),
            }
            .get_region_vec(),
            "xk",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "za"),
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
            "za",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "zm"),
                names: Some(vec!["zambia"]),
                demonyms: None,
                enterprises: None,
                misc: Some(vec!["upnd"]),
            }
            .get_region_vec(),
            "zm",
        ),
        (
            RegionKeyphrases {
                automated: get_automated_keyphrases(&region_map, "zw"),
                names: Some(vec!["zimbabwe"]),
                demonyms: Some(vec!["zimbo"]),
                enterprises: None,
                misc: Some(vec!["zanu-pf", "citizens coalition for change"]),
            }
            .get_region_vec(),
            "zw",
        ),
    ];

    let blacklist: HashSet<&'static str> = vec![
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
        "banan",
        "erode",
        "georgia",
        "georgetown",
        "godda",
        "hassan",
        "kent",
        "morena",
        "nice",
        "reading",
        "saint john's",
        "salem",
        "smic",
        "st. john's",
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
