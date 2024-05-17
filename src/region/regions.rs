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
    pub misc: Option<Vec<String>>, // Manual
}

impl RegionKeyphrases {
    pub fn get_region_vec(self) -> Vec<String> {
        let mut region_vec = Vec::new();
        // First-order administrative regions ≥ 490k population, capitals, cities ≥ 290k population, and heads of state and governmetn.
        if let Some(automated) = self.automated { region_vec.extend(automated); }
        if let Some(names) = self.names { region_vec.extend(names); }
        if let Some(demonyms) = self.demonyms { region_vec.extend(demonyms); }
        // ≥ 9.9B market cap USD
        if let Some(enterprises) = self.enterprises { region_vec.extend(enterprises); }
        // Positions of power, legislative bodies, institutions, buildings, political groups, ideologies, ethnic groups, cultural regions, etc.
        if let Some(misc) = self.misc { region_vec.extend(misc); }

        let mut quotationed_short_strings = Vec::new();
        region_vec.iter_mut().for_each(|s| if s.len() < 5 { quotationed_short_strings.push(format!("'{}'", s)); });
        region_vec.iter_mut().for_each(|s| if s.len() < 5 { quotationed_short_strings.push(format!("\"{}\"", s)); });
        region_vec.par_iter_mut().for_each(|s| if s.len() < 5 { *s = format!(" {} ", s); });
        region_vec.extend(quotationed_short_strings);
        region_vec.sort_by(|a, b| a.len().cmp(&b.len()));
        let mut i = 0;
        while i < region_vec.len() {
            let mut j = i + 1;
            while j < region_vec.len() {
                if region_vec[j].contains(&region_vec[i]) {
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
    let mut all_strings: HashSet<String> = vec.iter().flat_map(|(keys, _)| keys.clone()).collect();
    let all_strings_copy = all_strings.clone(); // Removes exact duplicates.
    let mut to_remove = blacklist;

    for string in &all_strings_copy {
        if to_remove.contains(string) { continue; }

        for other_string in &all_strings_copy {
            if string != other_string && string.contains(other_string) { to_remove.insert(other_string.clone()); } // Removes substrings.
        }
    }

    all_strings.retain(|string| !to_remove.contains(string));

    vec.iter().map(|(keys, value)| {
        let new_keys = keys.iter().filter(|key| all_strings.contains(*key)).cloned().collect();
        (new_keys, value.clone())
    }).collect()
}

pub static KEYPHRASE_REGION_MAP: Lazy<Vec<(Vec<String>, String)>> = Lazy::new(|| { // Feel free to submit pull requests!
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
        automated: get_automated_keyphrases(&region_map, "AN"),
        names: Some(vec!["netherlands antilles".into()]),
        demonyms: Some(vec!["netherlands antillean".into()]),
        enterprises: None,
        misc: None,
    }.get_region_vec(), "Netherlands Antilles".into()));
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
        names: Some(vec!["bonaire".into(), "sint eustatius".into(), "saba".into(), "statia".into()]),
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
        enterprises: None,
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
        misc: Some(vec!["compagnie ivoirienne".into()]),
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
        enterprises: None,
        misc: Some(vec!["national people's congress".into(), "cppcc".into(), "kuomintang".into(), "guomindang".into()]),
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
        demonyms: Some(vec!["cuban".into()]), // Strings with length 4 or less are processed before substring checking.
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
        demonyms: Some(vec!["fijian".into()]), // Strings with length 4 or less are processed before substring checking.
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
        demonyms: Some(vec!["french".into()]),
        enterprises: None,
        misc: None,
    }.get_region_vec(), "France".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "GA"),
        names: Some(vec!["gabon".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["pdg".into()]),
    }.get_region_vec(), "Gabon".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "GB"),
        names: Some(vec!["united kingdom".into(), "uk".into()]),
        demonyms: Some(vec!["british".into()]),
        enterprises: None,
        misc: Some(vec!["house of commons".into(), "house of lords".into(), "scottish parliament".into(), "welsh parliament".into(), "northern ireland assembly".into()]),
    }.get_region_vec(), "United Kingdom".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "GD"),
        names: Some(vec!["grenada".into()]),
        demonyms: None,
        enterprises: None,
        misc: None,
    }.get_region_vec(), "Grenada".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "GE"),
        names: Some(vec!["georgia".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["sakartvelo".into(), "dream".into(), "georgian dream".into()]),
    }.get_region_vec(), "Georgia".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "GF"),
        names: Some(vec!["french guiana".into()]),
        demonyms: None,
        enterprises: None,
        misc: None,
    }.get_region_vec(), "French Guiana".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "GG"),
        names: Some(vec!["guernsey".into()]),
        demonyms: None,
        enterprises: None,
        misc: None,
    }.get_region_vec(), "Guernsey".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "GH"),
        names: Some(vec!["ghana".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["ndc".into(), "npp".into()]),
    }.get_region_vec(), "Ghana".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "GI"),
        names: Some(vec!["gibraltar".into()]),
        demonyms: None,
        enterprises: None,
        misc: None,
    }.get_region_vec(), "Gibraltar".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "GL"),
        names: Some(vec!["greenland".into()]),
        demonyms: None,
        enterprises: None,
        misc: None,
    }.get_region_vec(), "Greenland".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "GM"),
        names: Some(vec!["gambia".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["ndp".into(), "udp".into()]),
    }.get_region_vec(), "Gambia".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "GN"),
        names: Some(vec!["guinea".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["rpg".into(), "upr".into()]),
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
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["pdge".into()]),
    }.get_region_vec(), "Equatorial Guinea".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "GR"),
        names: Some(vec!["greece".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["hellenic parliament".into(), "syriza".into(), "new democracy".into()]),
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
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["congreso de la republica".into(), "ucn".into(), "vamos".into()]),
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
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["paigc".into()]),
    }.get_region_vec(), "Guinea-Bissau".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "GY"),
        names: Some(vec!["guyana".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["pnc".into(), "ppp".into()]),
    }.get_region_vec(), "Guyana".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "HK"),
        names: Some(vec!["hong kong".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["legco".into(), "demosisto".into(), "dab".into()]),
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
        names: Some(vec!["honduras".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["congress of the republic".into(), "national party".into(), "libre".into()]),
    }.get_region_vec(), "Honduras".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "HR"),
        names: Some(vec!["croatia".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["sabor".into(), "banski dvori".into(), "hdz".into()]),
    }.get_region_vec(), "Croatia".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "HT"),
        names: Some(vec!["haiti".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["senat".into(), "chamber of deputies".into(), "phtk".into(), "fanmi lavalas".into()]),
    }.get_region_vec(), "Haiti".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "HU"),
        names: Some(vec!["hungary".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["orszaggyules".into(), "fidesz".into(), "jobbik".into()]),
    }.get_region_vec(), "Hungary".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "ID"),
        names: Some(vec!["indonesia".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["dpr".into(), "mpr".into(), "golkar".into(), "pdi-p".into()]),
    }.get_region_vec(), "Indonesia".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "IE"),
        names: Some(vec!["ireland".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["dail eireann".into(), "fine gael".into(), "fianna fail".into()]),
    }.get_region_vec(), "Ireland".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "IL"),
        names: Some(vec!["israel".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["knesset".into(), "likud".into(), "blue and white".into()]),
    }.get_region_vec(), "Israel".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "IM"),
        names: Some(vec!["isle of man".into()]),
        demonyms: None,
        enterprises: None,
        misc: None,
    }.get_region_vec(), "Isle of Man".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "IN"),
        names: Some(vec!["india".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["lok sabha".into(), "rajya sabha".into(), "bjp".into(), "inc".into()]),
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
        misc: Some(vec!["council of representatives".into(), "krg".into(), "puk".into(), "kdp".into()]),
    }.get_region_vec(), "Iraq".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "IR"),
        names: Some(vec!["iran".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["majlis".into(), "guardian council".into(), "principlists".into(), "reformists".into()]),
    }.get_region_vec(), "Iran".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "IS"),
        names: Some(vec!["iceland".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["althing".into(), "sja".into(), "samfylkingin".into()]),
    }.get_region_vec(), "Iceland".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "IT"),
        names: Some(vec!["italy".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["camera dei deputati".into(), "senato della repubblica".into(), "m5s".into(), "pd".into()]),
    }.get_region_vec(), "Italy".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "JE"),
        names: Some(vec!["jersey".into()]),
        demonyms: None,
        enterprises: None,
        misc: None,
    }.get_region_vec(), "Jersey".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "JM"),
        names: Some(vec!["jamaica".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["house of representatives".into(), "jlp".into(), "pnp".into()]),
    }.get_region_vec(), "Jamaica".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "JO"),
        names: Some(vec!["jordan".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["house of representatives".into(), "house of senate".into(), "hakama".into(), "muslim brotherhood".into()]),
    }.get_region_vec(), "Jordan".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "JP"),
        names: Some(vec!["japan".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["national diet".into(), "ldp".into(), "dpj".into()]),
    }.get_region_vec(), "Japan".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "KE"),
        names: Some(vec!["kenya".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["national assembly".into(), "senate".into(), "jubilee party".into(), "odm".into()]),
    }.get_region_vec(), "Kenya".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "KG"),
        names: Some(vec!["kyrgyzstan".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["jogorku kenesh".into(), "sdp".into(), "atambaev".into()]),
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
        misc: Some(vec!["house of assembly".into(), "teberio".into()]),
    }.get_region_vec(), "Kiribati".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "KM"),
        names: Some(vec!["comoros".into()]),
        demonyms: Some(vec!["comorian".into()]),
        enterprises: None,
        misc: Some(vec!["orange party".into(), "republican organization for the future of new generations".into()]),
    }.get_region_vec(), "Comoros".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "KN"),
        names: Some(vec!["saint kitts".into(), "nevis".into()]),
        demonyms: None,
        enterprises: None,
        misc: None,
    }.get_region_vec(), "Saint Kitts and Nevis".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "KP"),
        names: Some(vec!["north korea".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["supreme people's assembly".into(), "kwp".into(), "kpa".into()]),
    }.get_region_vec(), "North Korea".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "KR"),
        names: Some(vec!["south korea".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["national assembly".into(), "saenuri".into(), "minjoo".into()]),
    }.get_region_vec(), "South Korea".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "KW"),
        names: Some(vec!["kuwait".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["national assembly".into(), "cabinet".into(), "kdp".into(), "pjp".into()]),
    }.get_region_vec(), "Kuwait".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "KY"),
        names: Some(vec!["cayman island".into()]),
        demonyms: None,
        enterprises: None,
        misc: None,
    }.get_region_vec(), "Cayman Islands".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "KZ"),
        names: Some(vec!["kazakhstan".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["mazhilis".into(), "nur otan".into(), "ak zhol".into()]),
    }.get_region_vec(), "Kazakhstan".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "LA"),
        names: Some(vec!["laos".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["national assembly".into(), "lprp".into()]),
    }.get_region_vec(), "Laos".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "LB"),
        names: Some(vec!["lebanon".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["parliament".into(), "march 14 alliance".into(), "march 8 alliance".into()]),
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
        misc: Some(vec!["landtag".into(), "vaterland".into()]),
    }.get_region_vec(), "Liechtenstein".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "LK"),
        names: Some(vec!["sri lanka".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["parliament".into(), "unp".into(), "slfp".into()]),
    }.get_region_vec(), "Sri Lanka".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "LR"),
        names: Some(vec!["liberia".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["house of representatives".into(), "senate".into(), "up".into(), "cpp".into()]),
    }.get_region_vec(), "Liberia".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "LS"),
        names: Some(vec!["lesotho".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["national assembly".into(), "abc".into(), "bnp".into()]),
    }.get_region_vec(), "Lesotho".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "LT"),
        names: Some(vec!["lithuania".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["seimas".into(), "ts-lkd".into(), "lsdp".into()]),
    }.get_region_vec(), "Lithuania".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "LU"),
        names: Some(vec!["luxembourg".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["chamber of deputies".into(), "csv".into(), "dp".into()]),
    }.get_region_vec(), "Luxembourg".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "LV"),
        names: Some(vec!["latvia".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["saeima".into(), "sdp".into(), "jkp".into()]),
    }.get_region_vec(), "Latvia".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "LY"),
        names: Some(vec!["libya".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["house of representatives".into(), "gna".into(), "hoc".into()]),
    }.get_region_vec(), "Libya".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "MA"),
        names: Some(vec!["morocco".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["parliament".into(), "pjd".into(), "rni".into()]),
    }.get_region_vec(), "Morocco".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "MC"),
        names: Some(vec!["monaco".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["conseil national".into(), "upr".into()]),
    }.get_region_vec(), "Monaco".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "MD"),
        names: Some(vec!["moldova".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["parliament".into(), "psrm".into(), "pdm".into()]),
    }.get_region_vec(), "Moldova".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "ME"),
        names: Some(vec!["montenegro".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["parliament".into(), "dps".into(), "df".into()]),
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
        names: Some(vec!["madagascar".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["national assembly".into(), "hvm".into(), "tim".into()]),
    }.get_region_vec(), "Madagascar".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "MH"),
        names: Some(vec!["marshall island".into()]),
        demonyms: None,
        enterprises: None,
        misc: None,
    }.get_region_vec(), "Marshall Islands".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "MK"),
        names: Some(vec!["north macedonia".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["sobranie".into(), "vmro-dpmne".into(), "sdsm".into()]),
    }.get_region_vec(), "North Macedonia".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "ML"),
        names: Some(vec!["mali".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["national assembly".into(), "rally for mali".into(), "adema".into()]),
    }.get_region_vec(), "Mali".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "MM"),
        names: Some(vec!["myanmar".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["pyidaungsu hluttaw".into(), "usdp".into(), "nld".into()]),
    }.get_region_vec(), "Myanmar".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "MN"),
        names: Some(vec!["mongolia".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["state great khural".into(), "mpp".into(), "dp".into()]),
    }.get_region_vec(), "Mongolia".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "MO"),
        names: Some(vec!["macau".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["legislative assembly".into(), "dab".into(), "adp".into()]),
    }.get_region_vec(), "Macau".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "MP"),
        names: Some(vec!["northern mariana island".into()]),
        demonyms: None,
        enterprises: None,
        misc: None,
    }.get_region_vec(), "Northern Mariana Islands".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "MQ"),
        names: Some(vec!["martinique".into()]),
        demonyms: None,
        enterprises: None,
        misc: None,
    }.get_region_vec(), "Martinique".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "MR"),
        names: Some(vec!["mauritania".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["national assembly".into(), "upr".into(), "rfd".into()]),
    }.get_region_vec(), "Mauritania".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "MS"),
        names: Some(vec!["montserrat".into()]),
        demonyms: None,
        enterprises: None,
        misc: None,
    }.get_region_vec(), "Montserrat".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "MT"),
        names: Some(vec!["malta".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["house of representatives".into(), "pn".into(), "pl".into()]),
    }.get_region_vec(), "Malta".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "MU"),
        names: Some(vec!["mauritius".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["national assembly".into()]),
    }.get_region_vec(), "Mauritius".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "MV"),
        names: Some(vec!["maldives".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["people's majlis".into(), "mdp".into(), "ppm".into()]),
    }.get_region_vec(), "Maldives".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "MW"),
        names: Some(vec!["malawi".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["national assembly".into(), "dpp".into(), "udf".into()]),
    }.get_region_vec(), "Malawi".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "MX"),
        names: Some(vec!["mexico".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["congreso de la union".into(), "morena".into(), "pan".into()]),
    }.get_region_vec(), "Mexico".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "MY"),
        names: Some(vec!["malaysia".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["parliament".into(), "bn".into(), "ph".into()]),
    }.get_region_vec(), "Malaysia".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "MZ"),
        names: Some(vec!["mozambique".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["assembly of the republic".into(), "frelimo".into(), "renamo".into()]),
    }.get_region_vec(), "Mozambique".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "NA"),
        names: Some(vec!["namibia".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["national assembly".into(), "swapo".into(), "dtc".into()]),
    }.get_region_vec(), "Namibia".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "NC"),
        names: Some(vec!["new caledonia".into()]),
        demonyms: None,
        enterprises: None,
        misc: None,
    }.get_region_vec(), "New Caledonia".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "NE"),
        names: Some(vec!["niger".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["national assembly".into(), "pn".into(), "mnsd".into()]),
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
        misc: Some(vec!["national assembly".into(), "apc".into(), "pdp".into()]),
    }.get_region_vec(), "Nigeria".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "NI"),
        names: Some(vec!["nicaragua".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["national assembly".into(), "fsln".into(), "pli".into()]),
    }.get_region_vec(), "Nicaragua".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "NL"),
        names: Some(vec!["netherlands".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["tweede kamer".into()]),
    }.get_region_vec(), "Netherlands".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "NO"),
        names: Some(vec!["norway".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["storting".into(), "ap".into(), "h".into()]),
    }.get_region_vec(), "Norway".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "NP"),
        names: Some(vec!["nepal".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["parliament".into(), "ncp".into(), "nc".into()]),
    }.get_region_vec(), "Nepal".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "NR"),
        names: Some(vec!["nauru".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["parliament".into(), "anf".into(), "nlp".into()]),
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
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["parliament".into(), "nzlp".into(), "nznp".into()]),
    }.get_region_vec(), "New Zealand".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "OM"),
        names: Some(vec!["oman".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["consultative council".into(), "shura council".into(), "majlis al-shura".into()]),
    }.get_region_vec(), "Oman".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "PA"),
        names: Some(vec!["panama".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["national assembly".into(), "prd".into(), "cd".into()]),
    }.get_region_vec(), "Panama".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "PE"),
        names: Some(vec!["peru".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["congress".into(), "fujimorismo".into(), "apra".into()]),
    }.get_region_vec(), "Peru".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "PF"),
        names: Some(vec!["french polynesia".into()]),
        demonyms: None,
        enterprises: None,
        misc: None,
    }.get_region_vec(), "French Polynesia".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "PG"),
        names: Some(vec!["papua new guinea".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["parliament".into(), "pnc".into(), "pangu".into()]),
    }.get_region_vec(), "Papua New Guinea".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "PH"),
        names: Some(vec!["philippines".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["congress".into(), "duterte".into(), "aquino".into()]),
    }.get_region_vec(), "Philippines".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "PK"),
        names: Some(vec!["pakistan".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["national assembly".into(), "senate".into(), "pti".into(), "pml-n".into()]),
    }.get_region_vec(), "Pakistan".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "PL"),
        names: Some(vec!["poland".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["sejm".into(), "pis".into(), "po".into()]),
    }.get_region_vec(), "Poland".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "PM"),
        names: Some(vec!["saint pierre".into(), "miquelon".into()]),
        demonyms: None,
        enterprises: None,
        misc: None,
    }.get_region_vec(), "Saint Pierre and Miquelon".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "PN"),
        names: Some(vec!["pitcairn island".into()]),
        demonyms: None,
        enterprises: None,
        misc: None,
    }.get_region_vec(), "Pitcairn Islands".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "PR"),
        names: Some(vec!["puerto rico".into()]),
        demonyms: None,
        enterprises: None,
        misc: None,
    }.get_region_vec(), "Puerto Rico".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "PS"),
        names: Some(vec!["palestine".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["legislative council".into(), "plo".into(), "hamas".into()]),
    }.get_region_vec(), "Palestine".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "PT"),
        names: Some(vec!["portugal".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["assembly of the republic".into(), "ps".into(), "psd".into()]),
    }.get_region_vec(), "Portugal".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "PW"),
        names: Some(vec!["palau".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["house of delegates".into(), "senate".into(), "pdp".into()]),
    }.get_region_vec(), "Palau".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "PY"),
        names: Some(vec!["paraguay".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["congress".into(), "colorado".into(), "plra".into()]),
    }.get_region_vec(), "Paraguay".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "QA"),
        names: Some(vec!["qatar".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["advisory council".into(), "shura council".into(), "al thani".into()]),
    }.get_region_vec(), "Qatar".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "RE"),
        names: Some(vec!["reunion".into()]),
        demonyms: None,
        enterprises: None,
        misc: None,
    }.get_region_vec(), "Reunion".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "RO"),
        names: Some(vec!["romania".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["parliament".into(), "psd".into(), "pnl".into()]),
    }.get_region_vec(), "Romania".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "RS"),
        names: Some(vec!["serbia".into()]), //montenegro
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["national assembly".into(), "sns".into(), "sps".into()]),
    }.get_region_vec(), "Serbia".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "RU"),
        names: Some(vec!["russia".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["state duma".into(), "united russia".into()]),
    }.get_region_vec(), "Russia".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "RW"),
        names: Some(vec!["rwanda".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["parliament".into(), "rpf".into(), "fdlr".into()]),
    }.get_region_vec(), "Rwanda".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "SA"),
        names: Some(vec!["saudi arabia".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["consultative council".into(), "shura council".into(), "al saud".into()]),
    }.get_region_vec(), "Saudi Arabia".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "SB"),
        names: Some(vec!["solomon island".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["parliament".into(), "sdp".into(), "sipa".into()]),
    }.get_region_vec(), "Solomon Islands".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "SC"),
        names: Some(vec!["seycheles".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["national assembly".into(), "ppm".into(), "us".into()]),
    }.get_region_vec(), "Seychelles".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "SD"),
        names: Some(vec!["sudan".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["national assembly".into(), "ncp".into(), "splm".into()]),
    }.get_region_vec(), "Sudan".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "SE"),
        names: Some(vec!["sweden".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["riksdag".into(), "s".into(), "m".into()]),
    }.get_region_vec(), "Sweden".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "SG"),
        names: Some(vec!["singapore".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["parliament".into(), "pap".into(), "wp".into()]),
    }.get_region_vec(), "Singapore".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "SH"),
        names: Some(vec!["saint helena".into()]),
        demonyms: None,
        enterprises: None,
        misc: None,
    }.get_region_vec(), "Saint Helena".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "SI"),
        names: Some(vec!["slovenia".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["national assembly".into(), "sds".into(), "sdl".into()]),
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
        names: Some(vec!["slovakia".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["national council".into(), "smer".into(), "sdku".into()]),
    }.get_region_vec(), "Slovakia".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "SL"),
        names: Some(vec!["sieera leone".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["parliament".into(), "apc".into(), "slpp".into()]),
    }.get_region_vec(), "Sierra Leone".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "SM"),
        names: Some(vec!["san marino".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["grand and general council".into(), "pdc".into(), "psd".into()]),
    }.get_region_vec(), "San Marino".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "SN"),
        names: Some(vec!["senegal".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["national assembly".into(), "apc".into(), "pds".into()]),
    }.get_region_vec(), "Senegal".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "SO"),
        names: Some(vec!["somalia".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["parliament".into(), "tfg".into(), "tnc".into()]),
    }.get_region_vec(), "Somalia".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "SR"),
        names: Some(vec!["suriname".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["national assembly".into(), "ndp".into(), "vhp".into()]),
    }.get_region_vec(), "Suriname".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "SS"),
        names: Some(vec!["south sudan".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["national assembly".into(), "splm".into(), "ssla".into()]),
    }.get_region_vec(), "South Sudan".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "ST"),
        names: Some(vec!["sao tome".into(), "principe".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["national assembly".into(), "adil".into(), "mlstp".into()]),
    }.get_region_vec(), "Sao Tome and Principe".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "SV"),
        names: Some(vec!["el salvador".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["legislative assembly".into(), "arena".into(), "fmln".into()]),
    }.get_region_vec(), "El Salvador".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "SX"),
        names: Some(vec!["sint maarten".into()]),
        demonyms: None,
        enterprises: None,
        misc: None,
    }.get_region_vec(), "Sint Maarten".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "SY"),
        names: Some(vec!["syria".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["people's council".into(), "ba'ath".into(), "snc".into()]),
    }.get_region_vec(), "Syria".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "SZ"),
        names: Some(vec!["swaziland".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["parliament".into(), "tinkhundla".into(), "sdp".into()]),
    }.get_region_vec(), "Swaziland".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "TC"),
        names: Some(vec!["turks".into(), "caicos".into()]),
        demonyms: None,
        enterprises: None,
        misc: None,
    }.get_region_vec(), "Turks and Caicos Islands".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "TD"),
        names: Some(vec!["chad".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["national transitional council".into()]),
    }.get_region_vec(), "Chad".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "TF"),
        names: Some(vec!["french southern territories".into()]),
        demonyms: None,
        enterprises: None,
        misc: None,
    }.get_region_vec(), "French Southern Territories".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "TG"),
        names: Some(vec!["togo".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["national assembly".into(), "unir".into(), "ufc".into()]),
    }.get_region_vec(), "Togo".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "TH"),
        names: Some(vec!["thailand".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["national assembly".into(), "pdp".into(), "dpt".into()]),
    }.get_region_vec(), "Thailand".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "TJ"),
        names: Some(vec!["tajikistan".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["supreme assembly".into(), "pdpt".into(), "cprf".into()]),
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
        names: Some(vec!["timor-leste".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["national parliament".into(), "cnrt".into(), "fretilin".into()]),
    }.get_region_vec(), "Timor-Leste".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "TM"),
        names: Some(vec!["turkmenistan".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["assembly".into(), "tdp".into(), "dpt".into()]),
    }.get_region_vec(), "Turkmenistan".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "TN"),
        names: Some(vec!["tunisia".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["assembly of the representatives of the people".into(), "nidaa tounes".into(), "ennahda".into()]),
    }.get_region_vec(), "Tunisia".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "TO"),
        names: Some(vec!["tonga".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["parliament".into(), "dpfi".into(), "dpt".into()]),
    }.get_region_vec(), "Tonga".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "TR"),
        names: Some(vec!["turkey".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["grand national assembly".into(), "akp".into(), "chp".into()]),
    }.get_region_vec(), "Turkey".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "TT"),
        names: Some(vec!["trinidad".into(), "tobago".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["parliament".into(), "pnm".into(), "unc".into()]),
    }.get_region_vec(), "Trinidad and Tobago".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "TV"),
        names: Some(vec!["tuvalu".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["parliament".into(), "tpp".into(), "tva".into()]),
    }.get_region_vec(), "Tuvalu".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "TW"),
        names: Some(vec!["taiwan".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["legislative yuan".into(), "kmt".into(), "dpp".into()]),
    }.get_region_vec(), "Taiwan".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "TZ"),
        names: Some(vec!["tanzania".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["national assembly".into(), "ccm".into(), "chadema".into()]),
    }.get_region_vec(), "Tanzania".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "UA"),
        names: Some(vec!["ukraine".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["verkhovna rada".into(), "servant of the people".into(), "opposition platform".into()]),
    }.get_region_vec(), "Ukraine".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "UG"),
        names: Some(vec!["uganda".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["parliament".into(), "nrm".into(), "fdc".into()]),
    }.get_region_vec(), "Uganda".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "UM"),
        names: Some(vec!["united states minor outlying islands".into()]),
        demonyms: None,
        enterprises: None,
        misc: None,
    }.get_region_vec(), "United States Minor Outlying Islands".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "US"),
        names: Some(vec!["united states".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["congress".into(), "democrats".into(), "republicans".into()]),
    }.get_region_vec(), "United States".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "UY"),
        names: Some(vec!["uruguay".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["general assembly".into(), "frente amplio".into(), "pn".into()]),
    }.get_region_vec(), "Uruguay".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "UZ"),
        names: Some(vec!["uzbekistan".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["supreme assembly".into(), "ldp".into(), "adolat".into()]),
    }.get_region_vec(), "Uzbekistan".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "VA"),
        names: Some(vec!["vatican city".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["college of cardinals".into(), "pope".into(), "vatican".into()]),
    }.get_region_vec(), "Vatican City".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "VC"),
        names: Some(vec!["saint vincent".into(), "grenadines".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["parliament".into(), "ulp".into(), "ndp".into()]),
    }.get_region_vec(), "Saint Vincent and the Grenadines".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "VE"),
        names: Some(vec!["venezuela".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["national assembly".into(), "psuv".into(), "mud".into()]),
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
        names: Some(vec!["united states virgin islands".into()]),
        demonyms: None,
        enterprises: None,
        misc: None,
    }.get_region_vec(), "United States Virgin Islands".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "VN"),
        names: Some(vec!["vietnam".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["national assembly".into(), "cpv".into(), "vdp".into()]),
    }.get_region_vec(), "Vietnam".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "VU"),
        names: Some(vec!["vanuatu".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["parliament".into(), "nup".into(), "vp".into()]),
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
        names: Some(vec!["samoa".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["parliament".into(), "hrpp".into(), "tsp".into()]),
    }.get_region_vec(), "Samoa".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "YE"),
        names: Some(vec!["yemen".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["house of representatives".into(), "gpc".into(), "houthi".into()]),
    }.get_region_vec(), "Yemen".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "YT"),
        names: Some(vec!["mayotte".into()]),
        demonyms: None,
        enterprises: None,
        misc: None,
    }.get_region_vec(), "Mayotte".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "XK"),
        names: Some(vec!["kosovo".into()]),
        demonyms: None,
        enterprises: None,
        misc: None,
    }.get_region_vec(), "Kosovo".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "ZA"),
        names: Some(vec!["south africa".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["national assembly".into(), "anc".into(), "da".into()]),
    }.get_region_vec(), "South Africa".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "ZM"),
        names: Some(vec!["zambia".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["national assembly".into(), "pf".into(), "upnd".into()]),
    }.get_region_vec(), "Zambia".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "ZW"),
        names: Some(vec!["zimbabwe".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["parliament".into(), "zanu-pf".into(), "mdc".into()]),
    }.get_region_vec(), "Zimbabwe".into()));

    remove_ambiguities(map, vec!["chad".into(), "georgia".into(), "jordan".into(), "turkey".into()].into_par_iter().collect()) //TODO: look at sqlite db for more
});