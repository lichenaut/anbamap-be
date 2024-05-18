use std::{collections::HashSet, error::Error, vec};
use async_std::task;
use rayon::iter::{IntoParallelIterator, IntoParallelRefMutIterator, ParallelIterator};
use crate::db::keyphrase_db::get_region_db_pool;
use sqlx::Row;
use once_cell::sync::Lazy;
use std::collections::HashMap;

struct RegionKeyphrases {
    pub automated: Option<Vec<String>>, // src/db/keyphrase_db.rs
    pub names: Option<Vec<String>>, // Manual
    pub demonyms: Option<Vec<String>>, // Manual
    pub enterprises: Option<Vec<String>>, // Manual: https://companiesmarketcap.com/all-countries/
    pub misc: Option<Vec<String>>, // Manual: https://www.forbes.com/real-time-billionaires/#6bc435393d78
}

impl RegionKeyphrases {
    pub fn get_region_vec(self) -> Vec<String> {
        let mut region_vec = Vec::new();
        // First-order administrative regions ≥ 490k population, capitals, cities ≥ 290k population, and heads of state and government.
        if let Some(automated) = self.automated { region_vec.extend(automated); }
        if let Some(names) = self.names { region_vec.extend(names); }
        if let Some(demonyms) = self.demonyms { region_vec.extend(demonyms); }
        // ≥ 9.9B market cap USD
        if let Some(enterprises) = self.enterprises { region_vec.extend(enterprises); }
        // Positions of power, legislative bodies, institutions, buildings, political groups, ideologies, ethnic groups, cultural regions, billionaires ≥ 9.9B net worth, etc.
        if let Some(misc) = self.misc { region_vec.extend(misc); }

        let mut short_strings = Vec::new();
        region_vec.iter_mut().for_each(|s| if s.len() < 4 { short_strings.push(format!("'{}'", s)); });
        region_vec.iter_mut().for_each(|s| if s.len() < 4 { short_strings.push(format!("\"{}\"", s)); });
        region_vec.iter_mut().for_each(|s| if s.len() < 4 { *s = format!("{}.", s); });
        region_vec.iter_mut().for_each(|s| if s.len() < 4 { *s = format!("{},", s); });
        region_vec.par_iter_mut().for_each(|s| if s.len() < 4 { *s = format!(" {} ", s); });
        region_vec.extend(short_strings);

        // " inc" is a catch-all for other types here, where I include this string when the enterprise name is ambiguous (ex. 'apple' -> 'apple inc').
        // Enterprise type changes do not have to be tracked this way.
        let mut enterprise_types = Vec::new();
        region_vec.iter().for_each(|s| if s.ends_with(" inc") {
            enterprise_types.push(format!("{}, inc", &s[..s.len() - 4]));
            enterprise_types.push(format!("{} ltd", &s[..s.len() - 4]));
            enterprise_types.push(format!("{}, ltd", &s[..s.len() - 4]));
            enterprise_types.push(format!("{} limited", &s[..s.len() - 4]));
            enterprise_types.push(format!("{}, limited", &s[..s.len() - 4]));
            enterprise_types.push(format!("{} plc", &s[..s.len() - 4]));
            enterprise_types.push(format!("{}, plc", &s[..s.len() - 4]));
            enterprise_types.push(format!("{} llc", &s[..s.len() - 4]));
            enterprise_types.push(format!("{}, llc", &s[..s.len() - 4]));
        });
        region_vec.extend(enterprise_types);

        region_vec.sort_by(|a, b| a.len().cmp(&b.len()));
        let mut i = 0;
        while i < region_vec.len() {
            let mut j = i + 1;
            while j < region_vec.len() {
                if region_vec[j].contains(&region_vec[i]) {
                    // tracing::debug!("Removing region-level substring: {}", region_vec[j]);
                    region_vec.remove(j);
                } else {
                    j += 1;
                }
            }
            i += 1;
        }

        region_vec
    }
}

async fn build_region_map() -> Result<HashMap<String, Vec<String>>, Box<dyn Error>> {
    let pool = get_region_db_pool().await?;
    let mut region_map = HashMap::new();
    let rows = sqlx::query("SELECT * FROM regions").fetch_all(&pool).await?;
    for row in &rows { region_map.insert(row.get(0), vec![row.get(1)]); }

    Ok(region_map)
}

fn get_automated_keyphrases(region_map: &HashMap<String, Vec<String>>, region_code: &str) -> Option<Vec<String>> {
    let geo = region_map.get(region_code).cloned();

    geo.map(|g| {
        g.iter()
            .flat_map(|s| s.split(',').map(|s| s.trim().to_string()))
            .collect::<Vec<_>>()
            .into_par_iter()
            .collect()
    })
}

fn remove_ambiguities(vec: Vec<(Vec<String>, String)>, blacklist: HashSet<String>) -> Vec<(Vec<String>, String)> {
    // let mut map = HashMap::new();
    // for (key, _) in vec.clone() {
    //     for s in key {
    //         let count = map.entry(s.clone()).or_insert(0);
    //         *count += 1;
    //         if *count > 1 {
    //             tracing::debug!("Duplicate map-level keyphrase: {}", s);
    //         }
    //     }
    // }

    let mut all_strings: HashSet<String> = vec.iter().flat_map(|(keys, _)| keys.clone()).collect(); // Removes exact duplicates.
    let all_strings_copy = all_strings.clone(); 
    let mut to_remove = blacklist;

    for string in &all_strings_copy {
        if to_remove.contains(string) { continue; }

        for other_string in &all_strings_copy {
            if string != other_string && string.contains(other_string) { // Removes substrings.
                // tracing::debug!("Removing map-level substring: {} from {}", other_string, string);
                to_remove.insert(other_string.clone());
            }
        }
    }

    all_strings.retain(|string| !to_remove.contains(string));

    vec.iter().map(|(keys, value)| {
        let new_keys = keys.iter().filter(|key| all_strings.contains(*key)).cloned().collect();
        (new_keys, value.clone())
    }).collect()
}

pub static KEYPHRASE_REGION_MAP: Lazy<Vec<(Vec<String>, String)>> = Lazy::new(|| { // Please contribute on https://github.com/lichenaut/anbamap-api !
    let region_map = task::block_on(build_region_map());
    let region_map = match region_map {
        Ok(map) => map,
        Err(e) => {
            tracing::error!("Failed to build region map: {:?}", e);
            return Vec::new();
        }
    };

    let mut map: Vec<(Vec<String>, String)> = Vec::new();
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "AD"),
        names: Some(vec!["andorra".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["general syndic".into(), "council of the valleys".into()]),
    }.get_region_vec(), "Andorra".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "AE"),
        names: Some(vec!["united arab emirates".into(), "uae".into()]),
        demonyms: Some(vec!["emirati".into()]),
        enterprises: None,
        misc: None,
    }.get_region_vec(), "United Arab Emirates".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "AF"),
        names: None,
        demonyms: Some(vec!["afghan".into()]),
        enterprises: None,
        misc: Some(vec!["taliban".into()]),
    }.get_region_vec(), "Afghanistan".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "AG"),
        names: Some(vec!["antigua".into(), "barbuda".into(), "a&b".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["ablp".into(), "united progressive party".into()]),
    }.get_region_vec(), "Antigua and Barbuda".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "AI"),
        names: Some(vec!["anguilla".into()]),
        demonyms: None,
        enterprises: None,
        misc: None,
    }.get_region_vec(), "Anguilla".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "AL"),
        names: Some(vec!["albania".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["kuvendi".into()]),
    }.get_region_vec(), "Albania".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "AM"),
        names: Some(vec!["armenia".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["azgayin zhoghov".into()]),
    }.get_region_vec(), "Armenia".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "AO"),
        names: Some(vec!["angola".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["mpla".into(), "unita".into()]),
    }.get_region_vec(), "Angola".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "AQ"),
        names: Some(vec!["antarctica".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["mcmurdo".into()])
    }.get_region_vec(), "Antarctica".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "AR"),
        names: None,
        demonyms: Some(vec!["argentin".into()]),
        enterprises: Some(vec!["mercadolibre".into(), "mercado libre".into(), "ypf".into(), "yacimientos petroliferos fiscales".into()]),
        misc: Some(vec!["casa rosada".into(), "union for the homeland".into(), "juntos por el cambio".into(), "cambiemos".into(), "peronis".into(), "kirchneris".into()]),
    }.get_region_vec(), "Argentina".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "AS"),
        names: Some(vec!["american samoa".into()]),
        demonyms: None,
        enterprises: None,
        misc: None,
    }.get_region_vec(), "American Samoa".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "AT"),
        names: Some(vec!["austria".into(), "oesterreich".into()]),
        demonyms: None,
        enterprises: Some(vec!["verbund".into(), "erste group".into(), "omv".into()]),
        misc: None,
    }.get_region_vec(), "Austria".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "AU"),
        names: Some(vec!["australia".into()]),
        demonyms: Some(vec!["aussie".into()]),
        enterprises: Some(vec!["bhp group".into(), "commonwealth bank".into(), "csl".into(), "nab limited".into(), "anz bank".into(), "fortescue".into(), "wesfarmers".into(), "macquarie".into(), "atlassian".into(), "goodman group".into(), "woodside".into(), "telstra".into(), "transurban".into(), "woolworths".into(), "wisetech".into(), "qbe insurance".into(), "santos limited".into(), "aristocrat leisure".into(), "rea group".into(), "coles group".into(), "cochlear".into(), "suncorp".into(), "brambles limited".into(), "reece group".into(), "origin energy".into(), "northern star resources".into(), "scentre group".into(), "south32".into(), "computershare".into()]),
        misc: Some(vec!["aborigin".into()]),
    }.get_region_vec(), "Australia".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "AW"),
        names: Some(vec!["aruba".into()]),
        demonyms: None,
        enterprises: None,
        misc: None,
    }.get_region_vec(), "Aruba".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "AX"),
        names: Some(vec!["aland".into()]),
        demonyms: None,
        enterprises: None,
        misc: None,
    }.get_region_vec(), "Aland Islands".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "AZ"),
        names: Some(vec!["azerbaijan".into()]),
        demonyms: Some(vec!["azeri".into()]),
        enterprises: None,
        misc: Some(vec!["milli majlis".into(), "democratic reforms party".into()]),
    }.get_region_vec(), "Azerbaijan".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "BA"),
        names: Some(vec!["bosnia".into(), "srpska".into(), "brcko".into()]),
        demonyms: Some(vec!["herzegovin".into()]),
        enterprises: None,
        misc: Some(vec!["alliance of independent social democrats".into(), "party of democratic action".into()]),
    }.get_region_vec(), "Bosnia and Herzegovina".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "BB"),
        names: Some(vec!["barbados".into()]),
        demonyms: None,
        enterprises: None,
        misc: None,
    }.get_region_vec(), "Barbados".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "BD"),
        names: Some(vec!["bangladesh".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["jatiya sangsad".into(), "awami league".into(), "jatiya party".into(), "bengal".into()]),
    }.get_region_vec(), "Bangladesh".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "BE"),
        names: Some(vec!["belgium".into()]),
        demonyms: Some(vec!["belgian".into()]),
        enterprises: None,
        misc: Some(vec!["flemish".into(), "walloon".into()]),
    }.get_region_vec(), "Belgium".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "BF"),
        names: Some(vec!["burkina faso".into()]),
        demonyms: Some(vec!["burkinabe".into(), "burkinese".into()]),
        enterprises: None,
        misc: Some(vec!["mpsr".into()]),
    }.get_region_vec(), "Burkina Faso".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "BG"),
        names: Some(vec!["bulgaria".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["narodno sabranie".into(), "gerb".into()]),
    }.get_region_vec(), "Bulgaria".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "BH"),
        names: Some(vec!["bahrain".into()]),
        demonyms: None,
        enterprises: Some(vec!["ahli united".into()]),
        misc: Some(vec!["shura council".into(), "asalah".into(), "progressive democratic tribune".into(), "bchr".into()]),
    }.get_region_vec(), "Bahrain".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "BI"),
        names: Some(vec!["burundi".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["cndd".into(), "national congress for liberty".into(), "national congress for freedom".into()]),
    }.get_region_vec(), "Burundi".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "BJ"),
        names: Some(vec!["benin".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["progressive union for renewal".into()]),
    }.get_region_vec(), "Benin".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "BL"),
        names: Some(vec!["saint barthelemy".into()]),
        demonyms: Some(vec!["barthelemois".into()]),
        enterprises: None,
        misc: None,
    }.get_region_vec(), "Saint Barthelemy".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "BM"),
        names: Some(vec!["bermuda".into()]),
        demonyms: None,
        enterprises: None,
        misc: None,
    }.get_region_vec(), "Bermuda".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "BN"),
        names: Some(vec!["brunei".into()]),
        demonyms: None,
        enterprises: None,
        misc: None,
    }.get_region_vec(), "Brunei".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "BO"),
        names: Some(vec!["bolivia".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["pluritonal".into(), "plaza murillo".into()]),
    }.get_region_vec(), "Bolivia".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "BQ"),
        names: Some(vec!["bonaire".into(), "sint eustatius".into(), "saba".into(), "statia".into(), "bes island".into()]),
        demonyms: None,
        enterprises: None,
        misc: None,
    }.get_region_vec(), "Bonaire, Sint Eustatius, and Saba".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "BR"),
        names: Some(vec!["brazil".into(), "brasil".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["planalto".into()]),
    }.get_region_vec(), "Brazil".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "BS"),
        names: Some(vec!["bahama".into()]),
        demonyms: Some(vec!["bahamian".into()]),
        enterprises: None,
        misc: Some(vec!["progressive liberal party".into(), "free national movement".into()]),
    }.get_region_vec(), "The Bahamas".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "BT"),
        names: Some(vec!["bhutan".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["druk gyalpo".into()]),
    }.get_region_vec(), "Bhutan".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "BV"),
        names: Some(vec!["bouvet".into()]),
        demonyms: None,
        enterprises: None,
        misc: None,
    }.get_region_vec(), "Bouvet Island".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "BW"),
        names: Some(vec!["botswana".into()]),
        demonyms: Some(vec!["batswana".into(), "motswana".into()]),
        enterprises: None,
        misc: Some(vec!["umbrella for democratic change".into()]),
    }.get_region_vec(), "Botswana".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "BY"),
        names: Some(vec!["belarus".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["belaya rus".into(), "ldpb".into()]),
    }.get_region_vec(), "Belarus".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "BZ"),
        names: Some(vec!["belize".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["people's united party".into()]),
    }.get_region_vec(), "Belize".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "CA"),
        names: Some(vec!["canada".into()]),
        demonyms: Some(vec!["canadian".into()]),
        enterprises: Some(vec!["enbridge".into(), "reuters".into(), "shopify".into(), "brookfield".into(), "scotiabank".into(), "constellation software".into(), "alimentation".into(), "couche-tard".into(), "suncor energy".into(), "manulife".into(), "cibc".into(), "lululemon".into(), "tc energy".into(), "cenovus".into(), "imperial oil inc".into(), "loblaw".into(), "agnico eagle".into(), "restaurant brands international".into(), "barrick gold".into(), "bce inc".into(), "sun life financial".into(), "intact financial inc".into(), "great-west lifeco".into(), "nutrien inc".into(), "teck resources".into(), "fairfax".into(), "wheaton precious".into(), "wheaton metals".into(), "dollarama".into(), "franco-nevada".into(), "telus".into(), "cgi inc".into(), "cameco".into(), "rogers comm".into(), "pembina".into(), "fortis".into(), "ivanhoe".into(), "wsp global".into(), "george weston".into(), "hydro one".into(), "tourmaline oil".into(), "ritchie bros".into(), "magna international".into(), "power financial inc".into(), "metro inc".into(), "gfl".into(), "first quantum minerals".into(), "arc resources".into(), "tfi international".into(), "emera".into(), "lundin mining".into()]),
        misc: Some(vec!["parliament hill".into(), "rcmp".into(), "ndp".into(), "quebecois".into(), "metis".into(), "first nations".into()]),
    }.get_region_vec(), "Canada".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "CC"),
        names: Some(vec!["cocos island".into(), "keeling island".into()]),
        demonyms: None,
        enterprises: None,
        misc: None,
    }.get_region_vec(), "Cocos (Keeling) Islands".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "CD"),
        names: Some(vec!["democratic republic of the congo".into(), "drc".into(), "big congo".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["udps".into(), "common front for congo".into(), "kabila coalition".into(), "lamuka".into(), "fardc".into(), "monusco".into()]),
    }.get_region_vec(), "Democratic Republic of the Congo".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "CF"),
        names: None,
        demonyms: Some(vec!["central african".into()]),
        enterprises: None,
        misc: Some(vec!["united hearts movement".into(), "kwa na kwa".into(), "fprc".into(), "anti-balaka".into()]),
    }.get_region_vec(), "Central African Republic".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "CG"),
        names: Some(vec!["little congo".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["congolese party of labour".into(), "upads".into()]),
    }.get_region_vec(), "Republic of the Congo".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "CH"),
        names: Some(vec!["switzerland".into()]),
        demonyms: Some(vec!["swiss".into()]),
        enterprises: None,
        misc: None,
    }.get_region_vec(), "Switzerland".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "CI"),
        names: Some(vec!["ivory coast".into(), "cote d'ivoire".into()]),
        demonyms: Some(vec!["ivorian".into()]),
        enterprises: None,
        misc: None,
    }.get_region_vec(), "Ivory Coast".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "CK"),
        names: Some(vec!["cook island".into()]),
        demonyms: None,
        enterprises: None,
        misc: None,
    }.get_region_vec(), "Cook Islands".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "CL"),
        names: Some(vec!["chile".into()]),
        demonyms: None,
        enterprises: None,
        misc: None,
    }.get_region_vec(), "Chile".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "CM"),
        names: Some(vec!["cameroon".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["unity palace".into(), "rdpc".into(), "ambazonia".into()]),
    }.get_region_vec(), "Cameroon".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "CN"),
        names: Some(vec!["china".into(), "prc".into()]),
        demonyms: Some(vec!["chinese".into()]),
        enterprises: Some(vec!["tencent".into(), "kweichow moutai".into(), "icbc".into(), "alibaba".into(), "pinduoduo".into(), "cm bank".into(), "catl inc".into(), "cnooc".into(), "ping an".into(), "shenhua energy".into(), "sinopec".into(), "meituan".into(), "byd".into(), "foxconn".into(), "netease".into(), "zijin mining".into(), "nongfu spring".into(), "midea inc".into(), "xiaomi".into(), "jingdong mall".into(), "mindray".into(), "industrial bank inc".into(), "citic".into(), "hikvision".into(), "jiangsu hengrui".into(), "haier smart home".into(), "haier home".into(), "wanhua chem".into(), "baidu".into(), "luzhou laojiao".into(), "trip.com".into(), "muyuan foods".into(), "pudong".into(), "gree electric".into(), "gree appliances".into(), "anta sports".into(), "kuaishou tech".into(), "luxshare".into(), "the people's insurance co".into(), "picc".into(), "cosco shipping".into(), "east money information".into(), "great wall motors".into(), "crrc".into(), "s.f. express".into(), "sf express".into(), "li auto".into(), "yili group".into(), "smic".into(), "ke holdings".into(), "saic motor".into(), "didi".into(), "boe tech".into(), "minsheng bank".into(), "yankuang energy".into(), "yanzhou coal".into(), "yanzhou mining".into(), "bank of jiangsu".into(), "sungrow power".into(), "yanghe".into(), "zto".into(), "weichai".into(), "sany heavy industry".into(), "sany industry".into(), "beigene".into(), "longi ".into(), "seres group".into(), "anhui conch".into(), "zte".into(), "shandong gold".into(), "shandong mining".into(), "huaneng".into(), "aier eye".into(), "aier hospital".into(), "huatai securities".into(), "guotai junan".into(), "longyuan power".into(), "hua xia".into(), "hai di lao".into(), "shekou industrial".into(), "hansoh pharma".into(), "tsingtao".into(), "new oriental inc".into(), "longfor group".into(), "geely".into(), "huazhu hotels".into(), "jd health".into(), "vanke".into(), "avinex".into(), "nio".into(), "amec".into(), "enn".into(), "eve energy".into(), "zheshang bank".into(), "gac".into()]),
        misc: Some(vec!["national people's congress".into(), "cppcc".into(), "kuomintang".into(), "guomindang".into(), "yangtze".into()]),
    }.get_region_vec(), "China".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "CO"),
        names: Some(vec!["colombia".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["casa de narino".into(), "capitolio nacional".into(), "eln".into()]),
    }.get_region_vec(), "Colombia".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "CR"),
        names: Some(vec!["costa rica".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["inter-american court of human rights".into(), "social democratic progress party".into(), "national liberation party".into(), "verdiblancos".into()]),
    }.get_region_vec(), "Costa Rica".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "CU"),
        names: Some(vec!["cuba".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["national assembly of people's power".into()]),
    }.get_region_vec(), "Cuba".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "CV"),
        names: Some(vec!["cape verde".into()]),
        demonyms: Some(vec!["cabo verdean".into()]),
        enterprises: None,
        misc: Some(vec!["paicv".into()]),
    }.get_region_vec(), "Cape Verde".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "CW"),
        names: Some(vec!["curacao".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["mfk".into(), "real alternative party".into()]),
    }.get_region_vec(), "Curacao".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "CX"),
        names: Some(vec!["christmas island".into()]),
        demonyms: None,
        enterprises: None,
        misc: None,
    }.get_region_vec(), "Christmas Island".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "CY"),
        names: Some(vec!["cyprus".into()]),
        demonyms: Some(vec!["cypriot".into()]),
        enterprises: None,
        misc: Some(vec!["akel".into()]),
    }.get_region_vec(), "Cyprus".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "CZ"),
        names: None,
        demonyms: Some(vec!["czech".into()]),
        enterprises: None,
        misc: Some(vec!["spolu".into(), "ano 2011".into()]),
    }.get_region_vec(), "Czech Republic".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "DE"),
        names: None,
        demonyms: Some(vec!["german".into()]),
        enterprises: None,
        misc: Some(vec!["bundestag".into(), "cdu".into()]),
    }.get_region_vec(), "Germany".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "DJ"),
        names: Some(vec!["djibouti".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["union for the presidential majority".into()]),
    }.get_region_vec(), "Djibouti".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "DK"),
        names: Some(vec!["denmark".into()]),
        demonyms: Some(vec!["danish".into(), "dane".into()]),
        enterprises: None,
        misc: Some(vec!["folketing".into()]),
    }.get_region_vec(), "Denmark".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "DM"),
        names: Some(vec!["dominica ".into(), "dominica'".into(), "dominica\"".into()]),
        demonyms: None,
        enterprises: None,
        misc: None,
    }.get_region_vec(), "Dominica".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "DO"),
        names: Some(vec!["dominican republic".into()]),
        demonyms: None,
        enterprises: None,
        misc: None,
    }.get_region_vec(), "Dominican Republic".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "DZ"),
        names: Some(vec!["algeria".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["algerie".into(), "fln".into()]),
    }.get_region_vec(), "Algeria".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "EC"),
        names: Some(vec!["ecuador".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["union for hope".into()]),
    }.get_region_vec(), "Ecuador".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "EE"),
        names: Some(vec!["estonia".into()]),
        demonyms: None,
        enterprises: None,
        misc: None,
    }.get_region_vec(), "Estonia".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "EG"),
        names: Some(vec!["egypt".into()]),
        demonyms: None,
        enterprises: None,
        misc: None,
    }.get_region_vec(), "Egypt".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "EH"),
        names: Some(vec!["western sahara".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["polisario".into()]),
    }.get_region_vec(), "Western Sahara".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "ER"),
        names: Some(vec!["eritrea".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["pfdj".into()]),
    }.get_region_vec(), "Eritrea".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "ES"),
        names: Some(vec!["spain".into()]),
        demonyms: Some(vec!["spaniard".into()]),
        enterprises: None,
        misc: Some(vec!["cortes generales".into(), "psoe".into(), "sumar".into()]),
    }.get_region_vec(), "Spain".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "ET"),
        names: Some(vec!["ethiopia".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["house of federation".into(), "house of people's representatives".into(), "prosperity party".into(), "national movement of amhara".into()]),
    }.get_region_vec(), "Ethiopia".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "FI"),
        names: Some(vec!["finland".into()]),
        demonyms: Some(vec!["finn".into()]),
        enterprises: None,
        misc: Some(vec!["eduskunta".into(), "national coalition party".into(), ]),
    }.get_region_vec(), "Finland".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "FJ"),
        names: Some(vec!["fiji".into()]),
        demonyms: None,
        enterprises: None,
        misc: None,
    }.get_region_vec(), "Fiji".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "FK"),
        names: Some(vec!["falkland".into(), "malvinas".into()]),
        demonyms: None,
        enterprises: None,
        misc: None,
    }.get_region_vec(), "Falkland Islands".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "FM"),
        names: Some(vec!["micronesia".into(), "fsm".into()]),
        demonyms: None,
        enterprises: None,
        misc: None,
    }.get_region_vec(), "Micronesia".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "FO"),
        names: Some(vec!["faroe island".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["logting".into()]),
    }.get_region_vec(), "Faroe Islands".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "FR"),
        names: Some(vec!["france".into()]),
        demonyms: None,
        enterprises: Some(vec!["lvmh".into(), "hermes inc".into(), "l'oreal".into(), "totalenergies".into(), "dior".into(), "schneider electric".into(), "sanofi".into(), "air liquide".into(), "essilorluxottica".into(), "safran".into(), "bnp paribas".into(), "axa".into(), "vinci".into(), "dassault".into(), "credit agricole".into(), "compagnie de saint-gobain".into(), "kering".into(), "danone".into(), "engie".into(), "pernod ricard".into(), "capgemini".into(), "thales".into(), "orange inc".into(), "michelin".into(), "legrand".into(), "publicis group".into(), "veolia".into(), "societe generale".into(), "bollore".into(), "renault".into(), "amundi".into(), "bouygues".into(), "sodexo".into(), "bureau veritas".into(), "edenred".into(), "carrefour".into(), "biomerieux".into(), "unibail-rodamco".into(), "rodamco-westfield".into(), "vivendi".into(), "accor inc".into(), "ipsen".into(), "eiffage".into()]),
        misc: None,
    }.get_region_vec(), "France".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "GA"),
        names: Some(vec!["gabon".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["ctri".into()]),
    }.get_region_vec(), "Gabon".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "GB"),
        names: Some(vec!["united kingdom".into(), "uk".into(), "britain".into(), "england".into(), "scotland".into(), "wales".into(), "northern ireland".into()]),
        demonyms: Some(vec!["british".into(), "scottish".into(), "welsh".into(), "northern irish".into()]),
        enterprises: Some(vec!["astrazeneca".into(), "shell oil".into(), "shell inc".into(), "linde".into(), "hsbc".into(), "unilever".into(), "rio tonto".into(), "arm holdings".into(), "bp".into(), "glaxosmithkline".into(), "relx".into(), "diageo".into(), "aon".into(), "national grid inc".into(), "bae systems".into(), "compass group".into(), "anglo american inc".into(), "rolls-royce".into(), "lloyds bank".into(), "ferguson inc".into(), "barclays".into(), "reckitt benckiser".into(), "haleon".into(), "natwest".into(), "3i group".into(), "ashtead".into(), "antofagasta".into(), "prudential inc".into(), "tesco".into(), "vodafone inc".into(), "willis towers watson".into(), "sse".into(), "standard chartered".into(), "imperial brands inc".into(), "legal & general".into(), "bt group".into(), "intercontinental hotels group".into(), "royalty pharma".into(), "segro".into(), "next plc".into(), "informa plc".into(), "cnh".into(), "sage group".into(), "pentair".into(), "rentokil".into(), "nvent electric inc".into(), "bunzi".into(), "wpp".into(), "technipfmc".into(), "smith & nephew".into(), "halma".into(), "wise plc".into(), "intertek".into(), "melrose industries".into(), "admiral group".into(), "severn trent".into()]),
        misc: Some(vec!["house of lords".into(), "stormont".into()]),
    }.get_region_vec(), "United Kingdom".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "GD"),
        names: Some(vec!["grenada".into()]),
        demonyms: Some(vec!["grenadian".into()]),
        enterprises: None,
        misc: None,
    }.get_region_vec(), "Grenada".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "GE"),
        names: None,
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["abkhaz".into(), "united national movement".into()]),
    }.get_region_vec(), "Georgia".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "GF"),
        names: Some(vec!["french guiana".into()]),
        demonyms: Some(vec!["french guianan".into(), "french guinese".into()]),
        enterprises: None,
        misc: None,
    }.get_region_vec(), "French Guiana".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "GG"),
        names: Some(vec!["guernsey".into()]),
        demonyms: Some(vec!["giernesiais".into()]),
        enterprises: None,
        misc: None,
    }.get_region_vec(), "Guernsey".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "GH"),
        names: Some(vec!["ghana".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["national democratic congress".into(), "new patriotic party".into()]),
    }.get_region_vec(), "Ghana".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "GI"),
        names: Some(vec!["gibraltar".into()]),
        demonyms: Some(vec!["llanito".into()]),
        enterprises: None,
        misc: Some(vec!["gslp".into()]),
    }.get_region_vec(), "Gibraltar".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "GL"),
        names: Some(vec!["greenland".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["inuit ataqatigiit".into(), "naleraq".into(), "siumut".into()]),
    }.get_region_vec(), "Greenland".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "GM"),
        names: Some(vec!["gambia".into()]),
        demonyms: None,
        enterprises: None,
        misc: None,
    }.get_region_vec(), "Gambia".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "GN"),
        names: None,
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["cnrd".into()]),
    }.get_region_vec(), "Guinea".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "GP"),
        names: Some(vec!["guadeloupe".into()]),
        demonyms: None,
        enterprises: None,
        misc: None,
    }.get_region_vec(), "Guadeloupe".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "GQ"),
        names: Some(vec!["equatorial guinea".into()]),
        demonyms: Some(vec!["equatoguinean".into()]),
        enterprises: None,
        misc: Some(vec!["pdge".into()]),
    }.get_region_vec(), "Equatorial Guinea".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "GR"),
        names: Some(vec!["greece".into()]),
        demonyms: Some(vec!["greek".into()]),
        enterprises: None,
        misc: Some(vec!["helleni".into(), "syriza".into()]),
    }.get_region_vec(), "Greece".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "GS"),
        names: Some(vec!["south georgia".into(), "south sandwich".into()]),
        demonyms: None,
        enterprises: None,
        misc: None,
    }.get_region_vec(), "South Georgia and the South Sandwich Islands".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "GT"),
        names: Some(vec!["guatemala".into()]),
        demonyms: Some(vec!["chapin".into()]),
        enterprises: None,
        misc: Some(vec!["semilla".into()]),
    }.get_region_vec(), "Guatemala".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "GU"),
        names: Some(vec!["guam".into()]),
        demonyms: None,
        enterprises: None,
        misc: None,
    }.get_region_vec(), "Guam".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "GW"),
        names: Some(vec!["guinea-bissau".into()]),
        demonyms: Some(vec!["bissau-guinean".into()]),
        enterprises: None,
        misc: Some(vec!["terra ranka".into(), "paigc".into(), "madem g15".into(), "madem-g15".into()]),
    }.get_region_vec(), "Guinea-Bissau".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "GY"),
        names: Some(vec!["guyan".into()]),
        demonyms: None,
        enterprises: None,
        misc: None,
    }.get_region_vec(), "Guyana".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "HK"),
        names: Some(vec!["hong kong".into()]),
        demonyms: Some(vec!["hongkong".into()]),
        enterprises: None,
        misc: Some(vec!["legco".into()]),
    }.get_region_vec(), "Hong Kong".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "HM"),
        names: Some(vec!["heard island".into(), "mcdonald island".into()]),
        demonyms: None,
        enterprises: None,
        misc: None,
    }.get_region_vec(), "Heard Island and McDonald Islands".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "HN"),
        names: Some(vec!["hondura".into()]),
        demonyms: Some(vec!["catrach".into()]),
        enterprises: None,
        misc: Some(vec!["liberty and refoundation".into()]),
    }.get_region_vec(), "Honduras".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "HR"),
        names: Some(vec!["croatia".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["hdz".into()]),
    }.get_region_vec(), "Croatia".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "HT"),
        names: Some(vec!["haiti".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["phtk".into()]),
    }.get_region_vec(), "Haiti".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "HU"),
        names: Some(vec!["hungar".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["fidesz".into()]),
    }.get_region_vec(), "Hungary".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "ID"),
        names: Some(vec!["indonesia".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["pdi-p".into(), "golkar".into(), "prosperous justice party".into()]),
    }.get_region_vec(), "Indonesia".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "IE"),
        names: Some(vec!["ireland".into()]),
        demonyms: Some(vec!["irish".into()]),
        enterprises: None,
        misc: Some(vec!["oireachtas".into(), "fianna fail".into(), "fine gael".into(), "sinn fein".into()]),
    }.get_region_vec(), "Ireland".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "IL"),
        names: Some(vec!["israel".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["knesset".into(), "likud".into(), "shas".into(), "united torah judaism".into(), "mafdal".into(), "otzma".into(), "yesh atid".into(), "haaretz".into()]),
    }.get_region_vec(), "Israel".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "IM"),
        names: Some(vec!["isle of man".into()]),
        demonyms: Some(vec!["manx".into()]),
        enterprises: None,
        misc: Some(vec!["tynwald".into()]),
    }.get_region_vec(), "Isle of Man".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "IN"),
        names: Some(vec!["india".into(), "hindustan".into()]),
        demonyms: None,
        enterprises: Some(vec!["reliance industries".into(), "tata".into(), "hdfc".into(), "bharti airtel".into(), "icici".into(), "lic".into(), "infosys".into(), "itc".into(), "larsen & toubro".into(), "bajaj".into(), "maruti suzuki".into(), "sun pharma".into(), "hcl tech".into(), "ntpc".into(), "axis bank".into(), "oil & natural gas inc".into(), "adani".into(), "mahindra".into(), "dmart".into(), "titan company inc".into(), "ultratech cement".into(), "asian paints inc".into(), "wipro".into(), "jio financial".into(), "jio services".into(), "jsw".into(), "dlf".into(), "varun".into(), "bharat electronics".into(), "abb".into(), "zomato".into(), "interglobe aviation".into(), "trent limited".into(), "vedanta".into(), "grasim".into(), "power finance corp".into(), "ambuja".into(), "pidilite".into(), "hindalco".into(), "sbi life".into(), "rural electrificaiton group".into(), "ltimindtree".into(), "punjab bank".into(), "punjab national".into(), "bank of baroda".into(), "gail inc".into(), "godrej".into(), "eicher motor".into(), "britannia industries".into(), "lodha".into(), "havells".into(), "cipla".into(), "indusind".into(), "cholamandalam".into(), "zydus".into(), "divis lab".into(), "tvs motor".into(), "canara".into(), "jindal".into(), "hero motocorp".into(), "cg power and".into(), "cg industrial solutions".into(), "nhpc".into(), "dr. reddy's".into(), "dabur".into(), "shree cement".into(), "indus towers".into(), "torrent pharma".into(), "idbi bank".into(), "shriram".into(), "vodafone idea".into(), "samvardhana".into(), "apollo hospitals".into(), "united spirits".into(), "mankind pharma".into()]),
        misc: Some(vec!["lok sabha".into(), "rajya sabha".into(), "bjp".into()]),
    }.get_region_vec(), "India".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "IO"),
        names: Some(vec!["british indian ocean territory".into()]),
        demonyms: None,
        enterprises: None,
        misc: None,
    }.get_region_vec(), "British Indian Ocean Territory".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "IQ"),
        names: Some(vec!["iraq".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["takadum".into(), "emtidad".into()]),
    }.get_region_vec(), "Iraq".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "IR"),
        names: Some(vec!["iran".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["guardian council".into()]),
    }.get_region_vec(), "Iran".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "IS"),
        names: Some(vec!["iceland".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["althing".into(), "samfylkingin".into()]),
    }.get_region_vec(), "Iceland".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "IT"),
        names: Some(vec!["italy".into()]),
        demonyms: Some(vec!["italian".into()]),
        enterprises: None,
        misc: Some(vec!["lega".into(), "pd-idp".into()]),
    }.get_region_vec(), "Italy".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "JE"),
        names: None,
        demonyms: Some(vec!["jerseyman".into(), "jerseywoman".into(), "jersey bean".into(), "jersey crapaud".into(), "jerriais".into()]),
        enterprises: None,
        misc: None,
    }.get_region_vec(), "Jersey".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "JM"),
        names: Some(vec!["jamaica".into()]),
        demonyms: None,
        enterprises: None,
        misc: None,
    }.get_region_vec(), "Jamaica".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "JO"),
        names: None,
        demonyms: Some(vec!["jordanian".into()]),
        enterprises: None,
        misc: Some(vec!["islamic action front".into()]),
    }.get_region_vec(), "Jordan".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "JP"),
        names: Some(vec!["japan".into(), "nippon".into()]),
        demonyms: None,
        enterprises: Some(vec!["toyota".into(), "mitsubishi".into(), "keyence".into(), "sony".into(), "hitachi".into(), "ntt".into(), "sumitomo".into(), "mitsui".into(), "fast retailing inc".into(), "softbank".into(), "recruit inc".into(), "shin-etsu".into(), "daiichi".into(), "sankyo".into(), "itochu".into(), "shoji".into(), "nintendo".into(), "kddi".into(), "honda".into(), "chugai pharma".into(), "mizuho".into(), "denso".into(), "oriental land inc".into(), "daikin".into(), "hoya".into(), "takeda pharma".into(), "disco corp".into(), "murata".into(), "7-eleven".into(), "smc corp".into(), "marubeni".into(), "renesas".into(), "bridgestone".into(), "ms&ad".into(), "komatsu".into(), "fanuc".into(), "fujitsu".into(), "canon inc".into(), "nidec".into(), "terumo".into(), "fujifilm".into(), "advantest".into(), "orix".into(), "lasertec".into(), "dai-ichi".into(), "otsuka".into(), "suzuki motor".into(), "kao".into(), "sompo".into(), "panasonic".into(), "ajinomoto".into(), "unicharm".into(), "asahi group".into(), "inpex".into(), "olympus inc".into(), "z holdings".into(), "nec".into(), "aeon inc".into(), "kubota".into(), "nomura".into(), "tdk".into(), "astellas pharma".into(), "daiwa".into(), "kyocera".into(), "subaru".into(), "shimano".into(), "resona holdings".into(), "pan pacific international holdings".into(), "sekisui".into(), "nexon".into(), "eneos".into(), "kepco".into(), "secom".into(), "nitori".into(), "nissan".into(), "bandai namco".into(), "shionogi".into(), "eisai".into(), "shiseido".into(), "obic".into(), "kirin holdings".into(), "suntory".into(), "shinkin".into(), "nitto denko".into(), "kikkoman".into(), "sysmex".into(), "rakuten".into(), "yaskawa".into(), "\"k\" line".into()]),
        misc: Some(vec!["komeito".into(), "tokio".into()]),
    }.get_region_vec(), "Japan".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "KE"),
        names: Some(vec!["kenya".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["azimio".into()]),
    }.get_region_vec(), "Kenya".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "KG"),
        names: None,
        demonyms: Some(vec!["kyrgyz".into()]),
        enterprises: None,
        misc: Some(vec!["jogorku kenesh".into(), "mekenchil".into(), "eldik".into()]),
    }.get_region_vec(), "Kyrgyzstan".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "KH"),
        names: Some(vec!["cambodia".into()]),
        demonyms: Some(vec!["khmer".into()]),
        enterprises: None,
        misc: Some(vec!["funcinpec".into()]),
    }.get_region_vec(), "Cambodia".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "KI"),
        names: Some(vec!["kiribati".into()]),
        demonyms: None,
        enterprises: None,
        misc: None,
    }.get_region_vec(), "Kiribati".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "KM"),
        names: Some(vec!["comoros".into()]),
        demonyms: Some(vec!["comorian".into()]),
        enterprises: None,
        misc: Some(vec!["orange party".into()]),
    }.get_region_vec(), "Comoros".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "KN"),
        names: Some(vec!["saint kitts".into(), "nevis".into()]),
        demonyms: Some(vec!["kittitian".into(), "nevisian".into()]),
        enterprises: None,
        misc: Some(vec!["concerned citizens' movement".into()]),
    }.get_region_vec(), "Saint Kitts and Nevis".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "KP"),
        names: Some(vec!["north korea".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["supreme people's assembly".into(), "dfrk".into()]),
    }.get_region_vec(), "North Korea".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "KR"),
        names: Some(vec!["south korea".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["people power party".into()]),
    }.get_region_vec(), "South Korea".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "KW"),
        names: Some(vec!["kuwait".into()]),
        demonyms: None,
        enterprises: None,
        misc: None,
    }.get_region_vec(), "Kuwait".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "KY"),
        names: Some(vec!["cayman island".into()]),
        demonyms: Some(vec!["caymanian".into()]),
        enterprises: None,
        misc: None,
    }.get_region_vec(), "Cayman Islands".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "KZ"),
        names: None,
        demonyms: Some(vec!["kazakh".into()]),
        enterprises: None,
        misc: Some(vec!["mazhilis".into(), "amanat".into(), "auyl".into()]),
    }.get_region_vec(), "Kazakhstan".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "LA"),
        names: Some(vec!["laos".into()]),
        demonyms: Some(vec!["lao".into(), "laotian".into()]), // Strings with length 3 or less are processed before substring checking.
        enterprises: None,
        misc: Some(vec!["lprp".into()]),
    }.get_region_vec(), "Laos".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "LB"),
        names: Some(vec!["lebanon".into()]),
        demonyms: Some(vec!["lebanese".into()]),
        enterprises: None,
        misc: Some(vec!["free patriotic movement".into(), "amal movement".into(), "hezbollah".into(), "march 14 alliance".into(), "march 8 alliance".into()]),
    }.get_region_vec(), "Lebanon".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "LC"),
        names: Some(vec!["saint lucia".into()]),
        demonyms: None,
        enterprises: None,
        misc: None,
    }.get_region_vec(), "Saint Lucia".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "LI"),
        names: Some(vec!["liechtenstein".into()]),
        demonyms: None,
        enterprises: None,
        misc: None,
    }.get_region_vec(), "Liechtenstein".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "LK"),
        names: Some(vec!["sri lanka".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["slpfa".into(), "samagi jana balawegaya".into()]),
    }.get_region_vec(), "Sri Lanka".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "LR"),
        names: Some(vec!["liberia".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["coalition for democratic change".into(), ]),
    }.get_region_vec(), "Liberia".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "LS"),
        names: Some(vec!["lesotho".into()]),
        demonyms: Some(vec!["mosotho".into(), "basotho".into()]),
        enterprises: None,
        misc: Some(vec!["revolution for prosperity".into()]),
    }.get_region_vec(), "Lesotho".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "LT"),
        names: Some(vec!["lithuania".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["seimas".into(), "homeland union".into(), "lvzs".into()]),
    }.get_region_vec(), "Lithuania".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "LU"),
        names: Some(vec!["luxembourg".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["christian social people's party".into(), "lsap".into()]),
    }.get_region_vec(), "Luxembourg".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "LV"),
        names: Some(vec!["latvia".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["saeima".into(), "zzs".into()]),
    }.get_region_vec(), "Latvia".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "LY"),
        names: Some(vec!["libya".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["government of national".into()]),
    }.get_region_vec(), "Libya".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "MA"),
        names: Some(vec!["morocc".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["national rally of independents".into(), "istiqlal party".into(), "authenticity and modernity party".into(), "usfp".into()]),
    }.get_region_vec(), "Morocco".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "MC"),
        names: Some(vec!["monaco".into()]),
        demonyms: Some(vec!["monegasque".into(), "monacan".into()]),
        enterprises: None,
        misc: None,
    }.get_region_vec(), "Monaco".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "MD"),
        names: Some(vec!["moldova".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["party of action and solidarity".into(), "psrm".into()]),
    }.get_region_vec(), "Moldova".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "ME"),
        names: Some(vec!["monteneg".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["pes!".into()]),
    }.get_region_vec(), "Montenegro".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "MF"),
        names: Some(vec!["saint martin".into()]),
        demonyms: None,
        enterprises: None,
        misc: None,
    }.get_region_vec(), "Saint Martin".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "MG"),
        names: Some(vec!["madagas".into()]),
        demonyms: Some(vec!["malagas".into()]),
        enterprises: None,
        misc: None,
    }.get_region_vec(), "Madagascar".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "MH"),
        names: Some(vec!["marshall island".into()]),
        demonyms: Some(vec!["marshallese".into()]),
        enterprises: None,
        misc: None,
    }.get_region_vec(), "Marshall Islands".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "MK"),
        names: Some(vec!["macedonia".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["sobranie".into(), "sdsm".into(), "vmro-dpmne".into()]),
    }.get_region_vec(), "North Macedonia".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "ML"),
        names: Some(vec!["mali".into()]),
        demonyms: None,
        enterprises: None,
        misc: None,
    }.get_region_vec(), "Mali".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "MM"),
        names: Some(vec!["myanma".into()]),
        demonyms: Some(vec!["burmese".into()]),
        enterprises: None,
        misc: Some(vec!["pyidaungsu hluttaw".into(), "nld".into()]),
    }.get_region_vec(), "Myanmar".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "MN"),
        names: Some(vec!["mongol".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["state great khural".into()]),
    }.get_region_vec(), "Mongolia".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "MO"),
        names: Some(vec!["macau".into(), "macao".into()]),
        demonyms: None,
        enterprises: None,
        misc: None,
    }.get_region_vec(), "Macau".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "MP"),
        names: Some(vec!["northern mariana island".into()]),
        demonyms: Some(vec!["marianan".into(), "chamorro".into()]),
        enterprises: None,
        misc: None,
    }.get_region_vec(), "Northern Mariana Islands".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "MQ"),
        names: Some(vec!["martiniq".into()]),
        demonyms: Some(vec!["martinic".into()]),
        enterprises: None,
        misc: None,
    }.get_region_vec(), "Martinique".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "MR"),
        names: Some(vec!["mauritania".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["tewassoul".into()]),
    }.get_region_vec(), "Mauritania".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "MS"),
        names: Some(vec!["montserrat".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["movement for change and prosperity".into()]),
    }.get_region_vec(), "Montserrat".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "MT"),
        names: Some(vec!["malta".into()]),
        demonyms: Some(vec!["maltese".into()]),
        enterprises: None,
        misc: None,
    }.get_region_vec(), "Malta".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "MU"),
        names: Some(vec!["mauriti".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["mauricien".into()]),
    }.get_region_vec(), "Mauritius".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "MV"),
        names: Some(vec!["maldiv".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["people's majlis".into()]),
    }.get_region_vec(), "Maldives".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "MW"),
        names: Some(vec!["malawi".into()]),
        demonyms: None,
        enterprises: None,
        misc: None,
    }.get_region_vec(), "Malawi".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "MX"),
        names: Some(vec!["mexic".into()]),
        demonyms: None,
        enterprises: None,
        misc: None,
    }.get_region_vec(), "Mexico".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "MY"),
        names: Some(vec!["malaysia".into()]),
        demonyms: None,
        enterprises: None,
        misc: None,
    }.get_region_vec(), "Malaysia".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "MZ"),
        names: Some(vec!["mozambi".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["frelimo".into(), "renamo".into()]),
    }.get_region_vec(), "Mozambique".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "NA"),
        names: Some(vec!["namibia".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["swapo".into()]),
    }.get_region_vec(), "Namibia".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "NC"),
        names: Some(vec!["new caledonia".into()]),
        demonyms: Some(vec!["caledonian".into()]),
        enterprises: None,
        misc: Some(vec!["flnks".into(), "l'eo".into()]),
    }.get_region_vec(), "New Caledonia".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "NE"),
        names: None,
        demonyms: Some(vec!["nigerien".into()]),
        enterprises: None,
        misc: None,
    }.get_region_vec(), "Niger".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "NF"),
        names: Some(vec!["norfolk island".into()]),
        demonyms: None,
        enterprises: None,
        misc: None,
    }.get_region_vec(), "Norfolk Island".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "NG"),
        names: Some(vec!["nigeria".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["all progressives congress".into()]),
    }.get_region_vec(), "Nigeria".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "NI"),
        names: Some(vec!["nicaragua".into()]),
        demonyms: Some(vec!["pinoler".into()]),
        enterprises: None,
        misc: Some(vec!["sandinista".into(),]),
    }.get_region_vec(), "Nicaragua".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "NL"),
        names: Some(vec!["netherlands".into()]),
        demonyms: Some(vec!["dutch".into()]),
        enterprises: None,
        misc: Some(vec!["vvd".into(), "d66".into(), "pvv".into()]),
    }.get_region_vec(), "Netherlands".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "NO"),
        names: Some(vec!["norway".into()]),
        demonyms: Some(vec!["norwegian".into()]),
        enterprises: None,
        misc: Some(vec!["storting".into()]),
    }.get_region_vec(), "Norway".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "NP"),
        names: Some(vec!["nepal".into()]),
        demonyms: None,
        enterprises: None,
        misc: None,
    }.get_region_vec(), "Nepal".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "NR"),
        names: Some(vec!["nauru".into()]),
        demonyms: None,
        enterprises: None,
        misc: None,
    }.get_region_vec(), "Nauru".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "NU"),
        names: Some(vec!["niue".into()]),
        demonyms: None,
        enterprises: None,
        misc: None,
    }.get_region_vec(), "Niue".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "NZ"),
        names: Some(vec!["new zealand".into()]),
        demonyms: Some(vec!["kiwi".into()]),
        enterprises: None,
        misc: Some(vec!["parliament".into(), "nzlp".into()]),
    }.get_region_vec(), "New Zealand".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "OM"),
        names: Some(vec!["oman".into()]),
        demonyms: None,
        enterprises: None,
        misc: None,
    }.get_region_vec(), "Oman".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "PA"),
        names: Some(vec!["panama".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["molirena".into()]),
    }.get_region_vec(), "Panama".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "PE"),
        names: Some(vec!["peru".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["fujimoris".into()]),
    }.get_region_vec(), "Peru".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "PF"),
        names: Some(vec!["french polynesia".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["tavini".into(), "tapura".into()]),
    }.get_region_vec(), "French Polynesia".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "PG"),
        names: Some(vec!["papua new guinea".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["pangu pati".into()]),
    }.get_region_vec(), "Papua New Guinea".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "PH"),
        names: Some(vec!["philippine".into()]),
        demonyms: Some(vec!["filipin".into(), "pinoy".into()]),
        enterprises: None,
        misc: Some(vec!["uniteam alliance".into(), "tropa".into()]),
    }.get_region_vec(), "Philippines".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "PK"),
        names: Some(vec!["pakistan".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["pml-n".into(), "ittehad council".into()]),
    }.get_region_vec(), "Pakistan".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "PL"),
        names: Some(vec!["poland".into()]),
        demonyms: Some(vec!["polish".into()]),
        enterprises: None,
        misc: Some(vec!["sejm".into()]),
    }.get_region_vec(), "Poland".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "PM"),
        names: Some(vec!["saint pierre".into(), "miquelon".into()]),
        demonyms: Some(vec!["saint-pierrais".into(), "miquelonnais".into(), "pierrian".into()]),
        enterprises: None,
        misc: Some(vec!["archipelago tomorrow".into()]),
    }.get_region_vec(), "Saint Pierre and Miquelon".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "PN"),
        names: Some(vec!["pitcairn".into()]),
        demonyms: Some(vec!["pitkern".into()]),
        enterprises: None,
        misc: None,
    }.get_region_vec(), "Pitcairn Islands".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "PR"),
        names: Some(vec!["puerto ric".into()]),
        demonyms: None,
        enterprises: None,
        misc: None,
    }.get_region_vec(), "Puerto Rico".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "PS"),
        names: Some(vec!["palestin".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["plo".into(), "hamas".into(), "fatah".into()]),
    }.get_region_vec(), "Palestine".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "PT"),
        names: Some(vec!["portugal".into()]),
        demonyms: Some(vec!["portuguese".into()]),
        enterprises: None,
        misc: None,
    }.get_region_vec(), "Portugal".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "PW"),
        names: Some(vec!["palau".into()]),
        demonyms: None,
        enterprises: None,
        misc: None,
    }.get_region_vec(), "Palau".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "PY"),
        names: Some(vec!["paraguay".into()]),
        demonyms: None,
        enterprises: None,
        misc: None,
    }.get_region_vec(), "Paraguay".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "QA"),
        names: Some(vec!["qatar".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["house of thani".into()]),
    }.get_region_vec(), "Qatar".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "RE"),
        names: None,
        demonyms: Some(vec!["reunionese".into()]),
        enterprises: None,
        misc: None,
    }.get_region_vec(), "Reunion".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "RO"),
        names: Some(vec!["romania".into()]),
        demonyms: None,
        enterprises: None,
        misc: None,
    }.get_region_vec(), "Romania".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "RS"),
        names: Some(vec!["serbia".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["av-zms".into(), "sps-zs".into()]),
    }.get_region_vec(), "Serbia".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "RU"),
        names: Some(vec!["russia".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["state duma".into(), "ldpr".into()]),
    }.get_region_vec(), "Russia".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "RW"),
        names: Some(vec!["rwand".into()]),
        demonyms: None,
        enterprises: None,
        misc: None,
    }.get_region_vec(), "Rwanda".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "SA"),
        names: None,
        demonyms: Some(vec!["saudi".into()]),
        enterprises: Some(vec!["acwa power".into(), "acwa co".into(), "al rajhi".into(), "sabic".into(), "maaden".into(), "dr. sulaiman al habib".into(), "riyad".into(), "alinma".into(), "elm co".into(), "almarai".into(), "albilad".into(), "arab national bank".into(), "etihad etisalat".into(), "mobily".into()]),
        misc: None,
    }.get_region_vec(), "Saudi Arabia".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "SB"),
        names: Some(vec!["solomon island".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["kadere party".into()]),
    }.get_region_vec(), "Solomon Islands".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "SC"),
        names: Some(vec!["seychell".into()]),
        demonyms: Some(vec!["seselwa".into()]),
        enterprises: None,
        misc: None,
    }.get_region_vec(), "Seychelles".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "SD"),
        names: None,
        demonyms: None,
        enterprises: None,
        misc: None,
    }.get_region_vec(), "Sudan".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "SE"),
        names: None,
        demonyms: Some(vec!["swedish".into(), "swede".into()]),
        enterprises: None,
        misc: Some(vec!["riksdag".into()]),
    }.get_region_vec(), "Sweden".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "SG"),
        names: Some(vec!["singapore".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["people's action party".into()]),
    }.get_region_vec(), "Singapore".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "SH"),
        names: Some(vec!["saint helen".into()]),
        demonyms: Some(vec!["helenian".into()]),
        enterprises: None,
        misc: None,
    }.get_region_vec(), "Saint Helena".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "SI"),
        names: Some(vec!["sloven".into()]),
        demonyms: None,
        enterprises: None,
        misc: None,
    }.get_region_vec(), "Slovenia".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "SJ"),
        names: Some(vec!["svalbard".into(), "jan mayen".into()]),
        demonyms: None,
        enterprises: None,
        misc: None,
    }.get_region_vec(), "Svalbard and Jan Mayen".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "SK"),
        names: None,
        demonyms: Some(vec!["slovak".into()]),
        enterprises: None,
        misc: Some(vec!["smer-sd".into(), "hlas-sd".into()]),
    }.get_region_vec(), "Slovakia".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "SL"),
        names: Some(vec!["sierra leone".into()]),
        demonyms: None,
        enterprises: None,
        misc: None,
    }.get_region_vec(), "Sierra Leone".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "SM"),
        names: Some(vec!["san marino".into()]),
        demonyms: Some(vec!["sammarinese".into()]),
        enterprises: None,
        misc: None,
    }.get_region_vec(), "San Marino".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "SN"),
        names: Some(vec!["senegal".into()]),
        demonyms: None,
        enterprises: None,
        misc: None,
    }.get_region_vec(), "Senegal".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "SO"),
        names: None,
        demonyms: Some(vec!["somali".into()]),
        enterprises: None,
        misc: None,
    }.get_region_vec(), "Somalia".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "SR"),
        names: Some(vec!["suriname".into()]),
        demonyms: None,
        enterprises: None,
        misc: None,
    }.get_region_vec(), "Suriname".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "SS"),
        names: Some(vec!["south sudan".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["splm-in-opposition".into()]),
    }.get_region_vec(), "South Sudan".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "ST"),
        names: Some(vec!["sao tome".into(), "principe".into()]),
        demonyms: Some(vec!["santomean".into()]),
        enterprises: None,
        misc: Some(vec!["mlstp".into()]),
    }.get_region_vec(), "Sao Tome and Principe".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "SV"),
        names: Some(vec!["el salvador".into()]),
        demonyms: Some(vec!["salvadoran".into()]),
        enterprises: None,
        misc: Some(vec!["nuevas ideas".into()]),
    }.get_region_vec(), "El Salvador".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "SX"),
        names: Some(vec!["maarten".into()]),
        demonyms: None,
        enterprises: None,
        misc: None,
    }.get_region_vec(), "Sint Maarten".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "SY"),
        names: Some(vec!["syria".into()]),
        demonyms: None,
        enterprises: None,
        misc: None,
    }.get_region_vec(), "Syria".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "SZ"),
        names: Some(vec!["eswatini".into()]),
        demonyms: Some(vec!["swazi".into()]),
        enterprises: None,
        misc: Some(vec!["tinkhundla".into()]),
    }.get_region_vec(), "Eswatini".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "TC"),
        names: Some(vec!["turks and c".into(), "caicos".into()]),
        demonyms: Some(vec!["turks islander".into()]),
        enterprises: None,
        misc: None,
    }.get_region_vec(), "Turks and Caicos Islands".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "TD"),
        names: None,
        demonyms: Some(vec!["chadian".into()]),
        enterprises: None,
        misc: Some(vec!["national transitional council".into()]),
    }.get_region_vec(), "Chad".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "TF"),
        names: Some(vec!["french southern territories".into(), "adelie land".into(), "crozet island".into(), "kerguelen island".into(), "saint paul and amsterdam island".into(), "scattered islands".into()]),
        demonyms: Some(vec!["kerguelenois".into()]),
        enterprises: None,
        misc: None,
    }.get_region_vec(), "French Southern Territories".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "TG"),
        names: Some(vec!["togo".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["union of forces for change".into()]),
    }.get_region_vec(), "Togo".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "TH"),
        names: None,
        demonyms: Some(vec!["thai".into()]),
        enterprises: None,
        misc: Some(vec!["bhumjaithai".into(), "palang pracharath".into()]),
    }.get_region_vec(), "Thailand".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "TJ"),
        names: None,
        demonyms: Some(vec!["tajik".into()]),
        enterprises: None,
        misc: None,
    }.get_region_vec(), "Tajikistan".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "TK"),
        names: Some(vec!["tokelau".into()]),
        demonyms: None,
        enterprises: None,
        misc: None,
    }.get_region_vec(), "Tokelau".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "TL"),
        names: Some(vec!["timor-leste".into(), "east timor".into()]),
        demonyms: Some(vec!["timorese".into()]),
        enterprises: None,
        misc: Some(vec!["national parliament".into(), "cnrt".into(), "fretilin".into()]),
    }.get_region_vec(), "East Timor".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "TM"),
        names: None,
        demonyms: Some(vec!["turkmen".into()]),
        enterprises: None,
        misc: None,
    }.get_region_vec(), "Turkmenistan".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "TN"),
        names: Some(vec!["tunisia".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["assembly of the representatives of the people".into(), "25th of july movement".into()]),
    }.get_region_vec(), "Tunisia".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "TO"),
        names: Some(vec!["tonga".into()]),
        demonyms: None,
        enterprises: None,
        misc: None,
    }.get_region_vec(), "Tonga".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "TR"),
        names: Some(vec!["turkey".into()]),
        demonyms: Some(vec!["turkish".into()]),
        enterprises: None,
        misc: Some(vec!["grand national assembly".into()]),
    }.get_region_vec(), "Turkey".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "TT"),
        names: Some(vec!["tobago".into()]),
        demonyms: Some(vec!["trini".into(), "trinbagonian".into()]),
        enterprises: None,
        misc: None,
    }.get_region_vec(), "Trinidad and Tobago".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "TV"),
        names: Some(vec!["tuvalu".into()]),
        demonyms: None,
        enterprises: None,
        misc: None,
    }.get_region_vec(), "Tuvalu".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "TW"),
        names: Some(vec!["taiwan".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["legislative yuan".into(), "kuomintang".into(), "guomindang".into(),]),
    }.get_region_vec(), "Taiwan".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "TZ"),
        names: Some(vec!["tanzania".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["chama cha mapinduzi".into()]),
    }.get_region_vec(), "Tanzania".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "UA"),
        names: Some(vec!["ukrain".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["verkhovna rada".into()]),
    }.get_region_vec(), "Ukraine".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "UG"),
        names: Some(vec!["uganda".into()]),
        demonyms: None,
        enterprises: None,
        misc: None,
    }.get_region_vec(), "Uganda".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "UM"),
        names: Some(vec!["united states minor outlying islands".into(), "baker island".into(), "howland island".into(), "jarvis island".into(), "johnston atoll".into(), "kingman reef".into(), "midway atoll".into(), "palmyra atoll".into(), "wake island".into(), "navassa island".into()]),
        demonyms: None,
        enterprises: None,
        misc: None,
    }.get_region_vec(), "United States Minor Outlying Islands".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "US"),
        names: Some(vec!["united states".into(), "usa".into(), "u.s.a.".into()]),
        demonyms: None,
        enterprises: Some(vec!["microsoft".into(), "apple inc".into(), "nvidia".into(), "alphabet inc".into(), "amazon inc".into(), "meta platforms".into(), "berksire hathaway".into(), "eli lilly".into(), "broadcom".into(), "jpmorgan chase".into(), "visa inc".into(), "tesla".into(), "exxon mobil".into(), "walmart".into(), "unitedhealth".into(), "mastercard".into(), "proctor & gamble".into(), "johnson & johnson".into(), "costco".into(), "home depot".into(), "oracle inc".into(), "merck".into(), "bank of america".into(), "chevron".into(), "abbvie".into(), "salesforce".into(), "coca-cola".into(), "netflix".into(), "amd".into(), "pepsico".into(), "thermo fisher".into(), "adobe".into(), "qualcomm".into(), "wells fargo".into(), "danaher".into(), "mcdonald's".into(), "cisco".into(), "t-mobile".into(), "walt disney".into(), "intuit ".into(), "abbott lab".into(), "texas instruments".into(), "applied materials inc".into(), "general electric".into(), "american express".into(), "caterpillar inc".into(), "verizon".into(), "amgen".into(), "morgan stanley".into(), "pfizer".into(), "servicenow".into(), "nextera energy".into(), "ibm".into(), "philip morris".into(), "comcast".into(), "goldman sachs".into(), "union pacific corp".into(), "charles schwab".into(), "conocophillips".into(), "intuitive surgical".into(), "nike".into(), "micron technology".into(), "raytheon".into(), "s&p global".into(), "uber inc".into(), "intel inc".into(), "honeywell".into(), "lowe's".into(), "ups".into(), "stryker corp".into(), "elevance health".into(), "booking holdings".into(), "booking.com".into(), "at&t".into(), "progressive inc".into(), "citigroup".into(), "blackrock".into(), "lam research".into(), "vertex pharma".into(), "tjx co".into(), "boeing".into(), "lockheed martin".into(), "deere".into(), "boston scientific".into(), "regeneron pharma".into(), "dell".into(), "analog devices inc".into(), "marsh & mclennan".into(), "automatic data processing inc".into(), "prologis".into(), "palo alto".into(), "kla".into(), "arista networks".into(), "southern copper inc".into(), "kkr".into(), "cigna".into(), "mondelez".into(), "airbnb".into(), "fiserv".into(), "american tower inc".into(), "blackstone".into(), "bristol-meyers".into(), "chipotle".into(), "starbucks".into(), "southern company inc".into(), "synopsys".into(), "hca health".into(), "waste management inc".into(), "gilead science".into(), "crowdstrike".into(), "general dynamics".into(), "duke energy".into(), "zoetis".into(), "intercontinental exchange inc".into(), "amphenol".into(), "sherwin-williams".into(), "altria group".into(), "cadence design".into(), "freeport-mcmoran".into(), "colgate-palmolive".into(), "cme group".into(), "equinix".into(), "moody's".into(), "illinois tool works".into(), "eog resources".into(), "target inc".into(), "mckesson".into(), "cvs".into(), "transdigm".into(), "cintas".into(), "parker-hannifin".into(), "northrop".into(), "schlumberger".into(), "workday".into(), "becton dickinson".into(), "marriott".into(), "paypal".into(), "constellation energy".into(), "ecolab".into(), "csx corp".into(), "bancorp".into(), "emerson inc".into(), "apollo global".into(), "pnc financial".into(), "fedex".into(), "marathon petro".into(), "pioneer natural resources".into(), "phillips 66".into(), "marvell tech".into(), "enterprise products inc".into(), "motorola".into(), "welltower".into(), "o'reilly auto".into(), "republic services inc".into(), "carrier inc".into(), "air products and chemicals inc".into(), "3m".into(), "roper tech".into(), "monster beverage".into(), "arthur j. gallagher".into(), "occidental petro".into(), "simon property".into(), "paccar".into(), "valero".into(), "capital one".into(), "snowflake inc".into(), "energy transfer partners inc".into(), "edwards lifesciences".into(), "truist financial".into(), "american international group".into(), "metlife".into(), "copart".into(), "norfolk southern".into(), "dexcom".into(), "general motors".into(), "supermicro".into(), "interactive brokers inc".into(), "hilton world".into(), "coinbase".into(), "microchip technology inc".into(), "moderna".into(), "public storage inc".into(), "autozone".into(), "newmont".into(), "the travelers companies".into(), "williams companies".into(), "aflac".into(), "d. r. horton".into(), "sempra".into(), "american electric power".into(), "ford".into(), "hess".into(), "pacific gas and electric".into(), "palantir".into(), "estee lauder".into(), "oneok".into(), "doordash".into(), "realty income inc".into(), "autodesk".into(), "fortinet".into(), "constellation brands".into(), "w. w. grainger".into(), "the trade desk inc".into(), "united rentals".into(), "keurig".into(), "dr pepper".into(), "lennar inc".into(), "paychex".into(), "kimberly-clark".into(), "agilent tech".into(), "ares management".into(), "idexx lab".into(), "dominion energy".into(), "allstate".into(), "crown castle".into(), "block inc".into(), "bank of new york mellon".into(), "ross stores".into(), "cencora".into(), "kinder morgan".into(), "kraft".into(), "heinz".into(), "fidelity national".into(), "prudential financial".into(), "waste connections inc".into(), "ameriprise financial".into(), "humana".into(), "l3harris".into(), "iqvia".into(), "hershey".into(), "centene".into(), "dow inc".into(), "grayscale bitcoin".into(), "mplx".into(), "nucor".into(), "general mills".into(), "datadog".into(), "msci".into(), "yum! brands".into(), "old dominion freight".into(), "kroger".into(), "corteva".into(), "charter comm".into(), "kenvue".into(), "otis world".into(), "cummins".into(), "quanta services".into(), "ametek".into(), "exelon corp".into(), "fastenal".into(), "sysco".into(), "ge health".into(), "pseg".into(), "cheniere".into(), "royal caribbean".into(), "vertiv".into(), "nasdaq".into(), "verisk".into(), "martin marietta".into(), "costar group".into(), "monolithic power systems inc".into(), "diamondback energy".into(), "las vegas sands".into(), "gartner inc".into(), "fico".into(), "xylem".into(), "vulcan materials".into(), "cognizant technology solutions".into(), "electronic arts".into(), "delta air".into(), "veeva".into(), "howmet aero".into(), "bakar hughes".into(), "consolidated edison".into(), "biogen inc".into(), "halliburton".into(), "extra space storage inc".into(), "dupont de nemours".into(), "lyondellbasell".into(), "vistra".into(), "mettler-toledo".into(), "resmed".into(), "vici properties".into(), "ppg industries".into(), "on semiconductor inc".into(), "discover financial".into(), "devon energy".into(), "hubspot".into(), "dollar general".into(), "xcel energy".into(), "tractor supply".into(), "rockwell auto".into(), "equifax".into(), "hp".into(), "the hartford".into(), "archer daniels".into(), "corning".into(), "cdw corp".into(), "globalfoundries".into(), "wabtec".into(), "edison international".into(), "pinterest".into(), "ansys".into(), "avalonbay".into(), "microstrategy".into(), "rocket companies".into(), "cbre group".into(), "global payments inc".into(), "keysight".into(), "fortive".into(), "blue owl capital".into(), "applovin".into(), "mongodb".into(), "wec energy".into(), "zscaler".into(), "splunk".into(), "fifth third bank".into(), "snap inc".into(), "heico".into(), "raymond james".into(), "targa resources".into(), "t. rowe price".into(), "ebay".into(), "american water works inc".into(), "west pharma".into(), "church & dwight".into(), "symbiotic inc".into(), "m&t bank".into(), "brown & brown".into(), "dollar tree".into(), "cloudflare".into(), "first citizens banc".into(), "international flavors & fragrances".into(), "equity residential".into(), "dover".into(), "take 2 interactive".into(), "pultegroup".into(), "zimmer biomet".into(), "tradeweb".into(), "entergy".into(), "cardinal health".into(), "dte energy".into(), "broadridge financial".into(), "nvr".into(), "iron mountain".into(), "cheniere energy".into(), "western digital inc".into(), "state street corp".into(), "hewlett packard".into(), "brown forman".into(), "firstenergy".into(), "deckers brands".into(), "netapp".into(), "weyerhaeuser".into(), "samsara".into(), "live nation inc".into(), "rollins".into(), "ptc".into(), "ppl".into(), "axon enterprise".into(), "fleetcor".into(), "ball corp".into(), "alexandria real estate".into(), "invitation homes".into(), "celsius holdings".into(), "markel".into(), "eversource".into(), "tyson foods".into(), "sba comm".into(), "genuine parts co".into(), "first solar inc".into(), "waters corp".into(), "hubbell".into(), "roblox".into(), "draftkings".into(), "kellogg".into(), "steel dynamics inc".into(), "coterra".into(), "carvana".into(), "tyler tech".into(), "erie indemnity".into(), "huntington banc".into(), "teradyne".into(), "freddie mac".into(), "align tech".into(), "builders firstsource".into(), "molina health".into(), "westlake chem".into(), "w. r. berkley".into(), "leidos".into(), "lpl financial".into(), "principal inc".into(), "ameren".into(), "zoom".into(), "hormel foods".into(), "williams-sonoma".into(), "mccormick".into(), "carlisle companies".into(), "ventas".into(), "booz allen".into(), "carnival corporation inc".into(), "entegris".into(), "warner bros".into(), "cooper companies".into(), "cboe".into(), "ulta".into(), "teledyne".into(), "centerpoint".into(), "pure storage inc".into(), "godaddy".into(), "watsco".into(), "corebridge".into(), "alnylam pharma".into(), "cms energy".into(), "omnicom".into(), "cincinnati financial".into(), "regions financial".into(), "darden restaurants".into(), "avery dennison".into(), "eqt corp".into(), "united airlines".into(), "baxter".into(), "atmos energy".into(), "domino's".into(), "emcor".into(), "labcorp".into(), "essex property".into(), "illumina inc".into(), "robinhood".into(), "synchrony".into(), "hologic".into(), "northern trust inc".into(), "lennox".into(), "okta".into(), "loews corp".into(), "celanese".into(), "abiomed".into(), "nutanix".into(), "nrg energy".into(), "reliance steel".into(), "factset".into(), "jacobs engineering".into(), "j. b. hunt".into(), "verisign".into(), "textron".into(), "avantor".into(), "bentley systems".into(), "citizens financial group".into(), "clorox".into(), "idex".into(), "formula one".into(), "southwest airlines".into(), "expeditors inc".into(), "warner music".into(), "mid-america apartment communities inc".into(), "packaging corporation of america".into(), "zebra tech".into(), "quest diagnostics".into(), "dick's sporting".into(), "sun communities".into(), "best buy inc".into(), "ss&c tech".into(), "walgreens".into(), "gen digital".into(), "tpg capital".into(), "enphase energy".into(), "nordson".into(), "carlyle".into(), "masco".into(), "albemarie".into(), "amh".into(), "american homes 4 rent".into(), "owens corning".into(), "aes".into(), "news corp".into(), "expedia".into(), "transunion".into(), "hyatt".into(), "skyworks".into(), "toast inc".into(), "udr apartments".into(), "fox corp".into(), "marathon oil".into(), "biomarin pharma".into(), "snap-on inc".into(), "conagra".into(), "rpm international".into(), "bunge inc".into(), "keycorp".into(), "keybank".into(), "akamai".into(), "western midstream".into(), "neurocrine bio".into(), "dynatrace".into(), "international paper inc".into(), "ryan specialty".into(), "manhattan associates".into(), "poolcorp".into(), "aspentech".into(), "graco".into(), "texas pacific land trust".into(), "physicians realty".into(), "reinsurance group of america".into(), "trimble".into(), "cf industries".into(), "jabil".into(), "black & decker".into(), "avangrid".into(), "campbell soup".into(), "westrock".into(), "toll brothers".into(), "revvity".into(), "us foods inc".into(), "advanced drainage systems inc".into(), "alliant energy".into(), "permian resources".into(), "ovintiv".into(), "equitable holdings inc".into(), "bio-techne".into(), "host hotels & resorts".into(), "w. p. carey".into(), "insulet".into(), "nisource".into(), "viatris".into(), "natera".into(), "amerco".into(), "kimco realty".into(), "ares hospital".into(), "lincoln electric".into(), "mgm resorts".into(), "topbuild".into(), "incyte".into(), "xpo logistics".into(), "morningstar".into(), "franklin resources".into(), "floor & decor inc".into(), "evergy".into(), "equity lifestyle".into(), "karuna".into(), "a. o. smith".into(), "tenet health".into(), "lamb western".into(), "gaming and leisure properties".into(), "sarepta".into(), "casey's general".into(), "shockwave".into(), "burlington".into(), "docusign".into(), "jack henry".into(), "cna financial".into(), "davita".into(), "lamar advertising".into(), "smucker".into(), "aecom".into(), "ally inc".into(), "medspace".into(), "plains all american pipeline".into(), "united therapeutics".into(), "core & main".into(), "interpublic".into(), "chesapeake energy".into(), "molson coors".into(), "lkq corp".into(), "albertsons".into(), "universal health services inc".into(), "eastman chem".into(), "tetra tech".into(), "uipath".into(), "sirius xm".into(), "performance food".into(), "clean harbors inc".into(), "itt".into(), "apache corp".into(), "carmax".into(), "uwm holdings".into(), "charles river lab".into(), "camden property".into(), "wingstop".into(), "texas roadhouse".into(), "regency centers".into(), "comfort systems inc".into(), "astera lab".into(), "juniper networks".into(), "sinclair".into(), "bath & body works".into(), "pershing square".into(), "american financial group inc".into(), "boston properties inc".into(), "elastic nv".into(), "onto innovation".into(), "woodward".into(), "bruker".into(), "zoominfo".into(), "epam systems".into(), "antero resources".into(), "essential utilities inc".into(), "wynn resorts".into(), "td synnex".into(), "east west bancorp".into(), "ralph lauren".into(), "curtiss-wright".into(), "twilio".into(), "regal rexnord".into(), "bj's wholesale".into(), "paycom".into(), "saia".into(), "affirm inc".into(), "rivian".into(), "penske auto".into(), "skechers".into(), "sharkninja".into(), "zillow".into(), "rexford industrial".into(), "service corporation international".into(), "crown holdings".into(), "teleflex".into(), "confluent inc".into(), "guidewire".into(), "f5".into(), "annaly capital".into(), "procore".into(), "reddit".into(), "huntington ingalls".into(), "unum".into(), "cubesmart".into(), "lattice semiconductor".into(), "jefferies financial".into(), "catalent".into()]),
        misc: None,
    }.get_region_vec(), "United States".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "UY"),
        names: Some(vec!["uruguay".into()]),
        demonyms: None,
        enterprises: None,
        misc: None,
    }.get_region_vec(), "Uruguay".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "UZ"),
        names: Some(vec!["uzbekistan".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["justice social democratic party".into()]),
    }.get_region_vec(), "Uzbekistan".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "VA"),
        names: None,
        demonyms: Some(vec!["vatican".into()]),
        enterprises: None,
        misc: Some(vec!["college of cardinals".into(), "pope".into()]),
    }.get_region_vec(), "Vatican City".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "VC"),
        names: Some(vec!["saint vincent".into(), "grenadines".into()]),
        demonyms: Some(vec!["vincentian".into(), "grenadian".into(), "vincy".into()]),
        enterprises: None,
        misc: None,
    }.get_region_vec(), "Saint Vincent and the Grenadines".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "VE"),
        names: Some(vec!["venezuela".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["psuv".into()]),
    }.get_region_vec(), "Venezuela".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "VG"),
        names: Some(vec!["british virgin islands".into()]),
        demonyms: None,
        enterprises: None,
        misc: None,
    }.get_region_vec(), "British Virgin Islands".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "VI"),
        names: Some(vec!["united states virgin islands".into(), "us virgin islands".into(), "u.s. virgin islands".into()]),
        demonyms: None,
        enterprises: None,
        misc: None,
    }.get_region_vec(), "United States Virgin Islands".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "VN"),
        names: None,
        demonyms: Some(vec!["viet".into()]),
        enterprises: None,
        misc: None,
    }.get_region_vec(), "Vietnam".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "VU"),
        names: Some(vec!["vanua".into()]),
        demonyms: None,
        enterprises: None,
        misc: None,
    }.get_region_vec(), "Vanuatu".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "WF"),
        names: Some(vec!["wallis".into(), "futuna".into()]),
        demonyms: None,
        enterprises: None,
        misc: None,
    }.get_region_vec(), "Wallis and Futuna".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "WS"),
        names: None,
        demonyms: None,
        enterprises: None,
        misc: None,
    }.get_region_vec(), "Samoa".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "YE"),
        names: Some(vec!["yemen".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["houthi".into()]),
    }.get_region_vec(), "Yemen".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "YT"),
        names: Some(vec!["mayotte".into()]),
        demonyms: Some(vec!["mahoran".into(), "mahorais".into()]),
        enterprises: None,
        misc: None,
    }.get_region_vec(), "Mayotte".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "XK"),
        names: Some(vec!["kosov".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["vetevendosje".into(), "guxo".into()]),
    }.get_region_vec(), "Kosovo".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "ZA"),
        names: Some(vec!["south africa".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["african national congress".into()]),
    }.get_region_vec(), "South Africa".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "ZM"),
        names: Some(vec!["zambia".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["upnd".into()]),
    }.get_region_vec(), "Zambia".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "ZW"),
        names: Some(vec!["zimbabwe".into()]),
        demonyms: Some(vec!["zimbo".into()]),
        enterprises: None,
        misc: Some(vec!["zanu-pf".into(), "citizens coalition for change".into()]),
    }.get_region_vec(), "Zimbabwe".into()));

    remove_ambiguities(map, vec!["chad".into(), "georgia".into(), "jordan".into(), "turkey".into()].into_par_iter().collect()) //TODO: look at sqlite db for more
    //TODO temporary overrides: like gaza and rafah
});