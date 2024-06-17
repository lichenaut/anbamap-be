use super::region::KEYPHRASE_REGION_MAP;
use crate::{prelude::*, service::var_service::get_docker_volume};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use regex::Regex;
use std::{
    collections::HashMap,
    fmt::Display,
    process::Command,
    str::from_utf8,
    sync::{Arc, Mutex},
};
use unidecode::unidecode;
use url::Url;

pub(super) async fn get_regions(text: &[&str]) -> Result<Vec<String>> {
    let text = strip_content(text.join(" "))?;
    let identified_regions = get_flashgeotext_regions(&text).await?;
    let has_eu: bool = text.contains("EU");
    let text = &text.replace("\\'", "'").to_lowercase(); //TODO check for correct behavior
    let identified_regions = Arc::new(Mutex::new(identified_regions));
    KEYPHRASE_REGION_MAP
        .par_iter()
        .for_each(|(keyphrases, region)| {
            let identified_regions = Arc::clone(&identified_regions);
            let mut identified_regions_locked = match identified_regions.lock() {
                Ok(regions) => regions,
                Err(poisoned) => {
                    tracing::error!("Poisoned lock: {:?}", poisoned);
                    return;
                }
            };

            if identified_regions_locked.contains(region) {
                return;
            }

            for keyphrase in keyphrases.iter() {
                if text.contains(keyphrase) {
                    //println!("{}: {}", keyphrase, region);
                    identified_regions_locked.push(region);
                    break;
                }
            }
        });

    let mut regions = match identified_regions.lock() {
        Ok(regions) => regions.clone(),
        Err(poisoned) => {
            tracing::error!("Poisoned lock: {:?}", poisoned);
            Vec::new()
        }
    };

    if text.contains("georgia") && !regions.contains(&"us") && !regions.contains(&"ge") {
        regions.push("ge");
    }
    if text.contains("ireland") && !text.contains("northern ireland") && !regions.contains(&"ie") {
        regions.push("ie");
    }
    if text.contains("mexico") && !text.contains("new mexico") && !regions.contains(&"mx") {
        regions.push("mx");
    }
    if text.contains("sudan") && !text.contains("south sudan") && !regions.contains(&"sd") {
        regions.push("sd");
    }
    if regions.is_empty() && !text.contains("europe") && !has_eu {
        regions.push("us");
    }

    Ok(regions.iter().map(|s| s.to_string()).collect())
}

pub fn notify_parse_fail<T: Display>(msg: &str, item: T) {
    tracing::error!("Unexpected value while parsing HTML: {} at {}", msg, item);
}

pub fn get_base_url(url: &str) -> Result<String> {
    let parsed_url = Url::parse(url)?;
    Ok(format!(
        "{}://{}",
        parsed_url.scheme(),
        parsed_url.host_str().unwrap_or("")
    ))
}

pub fn look_between(text: &str, this: String, that: String) -> Result<Option<String>> {
    match text.splitn(2, &this).last() {
        Some(text) => Ok(text.split(&that).next().map(|text| text.to_string())),
        None => Ok(None),
    }
}

pub fn strip_html<T: ToString>(input: T) -> Result<String> {
    let mut replacements = HashMap::new();
    replacements.insert("&amp;", "&");
    replacements.insert("&hellip;", "...");
    replacements.insert("&nbsp;", " ");
    replacements.insert("&quot;", "\"");
    replacements.insert("&#39;", "'");
    replacements.insert("&#039;", "'");
    replacements.insert("&#8220;", "\"");
    replacements.insert("&#8221;", "\"");
    replacements.insert("&#8217;", "'");

    let input = Regex::new(r"<[^>]*>")?
        .replace_all(&input.to_string(), "")
        .to_string();
    let mut result = input;
    for (key, value) in replacements {
        result = result.replace(key, value);
    }

    Ok(result)
}

pub fn truncate_string(input: String) -> Result<String> {
    let mut words: Vec<&str> = input.split_whitespace().collect();
    while words.join(" ").len() > 130 {
        words.pop();
    }
    if words.len() < input.len() {
        words.push("...");
    }
    Ok(words.join(" "))
}

fn strip_content<T: ToString>(input: T) -> Result<String> {
    let mut replacements = HashMap::new();
    replacements.insert("\"", r"\'");
    replacements.insert("`", r"\'");
    replacements.insert("‘", r"\'");
    replacements.insert("’", r"\'");
    replacements.insert("'", r"\'");
    replacements.insert("–", "");

    let input = input.to_string();
    let mut result = String::with_capacity(input.len());
    let chars = input.chars();
    for ch in chars {
        match replacements.get(&*ch.to_string()) {
            Some(replacement) => result.push_str(replacement),
            None => result.push(ch),
        }
    }

    Ok(unidecode(&result))
}

async fn get_flashgeotext_regions(text: &str) -> Result<Vec<&'static str>> {
    let docker_volume = get_docker_volume().await?;
    let regions = Command::new(format!("{}/p3venv/bin/python", docker_volume))
        .arg("-c")
        .arg(format!(
            "import sys; sys.path.append('{}'); from media_to_regions import get_regions; print(get_regions('{}'))",
            docker_volume, text
        ))
        .output()?;
    let output = from_utf8(&regions.stdout)?;
    if output.is_empty() || output == "[]\n" {
        return Ok(Vec::new());
    }

    let output: Vec<&'static str> = output
        .trim()
        .replace(['[', ']', '\''], "")
        .split(", ")
        .map(|s| &*Box::leak(s.to_string().into_boxed_str()))
        .collect();
    let regions = output
        .into_iter()
        .filter(|s| {
            !matches!(
                *s,
                "Chad" | "Georgia" | "Guinea-Bissau" | "Jordan" | "Republic of Congo"
            )
        })
        .filter_map(|s| get_iso_from_name(s))
        .collect();

    Ok(regions)
}

pub fn get_iso_from_name(name: &str) -> Option<&str> {
    match name {
        "Andorra" => Some("ad"),
        "United Arab Emirates" => Some("ae"),
        "Afghanistan" => Some("af"),
        "Antigua and Barbuda" => Some("ag"),
        "Anguilla" => Some("ai"),
        "Albania" => Some("al"),
        "Armenia" => Some("am"),
        "Angola" => Some("ao"),
        "Antarctica" => Some("aq"),
        "Argentina" => Some("ar"),
        "American Samoa" => Some("as"),
        "Austria" => Some("at"),
        "Australia" => Some("au"),
        "Aruba" => Some("aw"),
        "Aland Islands" => Some("ax"),
        "Azerbaijan" => Some("az"),
        "Bosnia and Herzegovina" => Some("ba"),
        "Barbados" => Some("bb"),
        "Bangladesh" => Some("bd"),
        "Belgium" => Some("be"),
        "Burkina Faso" => Some("bf"),
        "Bulgaria" => Some("bg"),
        "Bahrain" => Some("bh"),
        "Burundi" => Some("bi"),
        "Benin" => Some("bj"),
        "Saint Barthelemy" => Some("bl"),
        "Bermuda" => Some("bm"),
        "Brunei" => Some("bn"),
        "Bolivia" => Some("bo"),
        "Bonaire, Sint Eustatius, and Saba" => Some("bq"),
        "Brazil" => Some("br"),
        "Bahamas" => Some("bs"),
        "Bhutan" => Some("bt"),
        "Bouvet Island" => Some("bv"),
        "Botswana" => Some("bw"),
        "Belarus" => Some("by"),
        "Belize" => Some("bz"),
        "Canada" => Some("ca"),
        "Cocos (Keeling) Islands" => Some("cc"),
        "Democratic Republic of the Congo" => Some("cd"),
        "Central African Republic" => Some("cf"),
        "Republic of the Congo" => Some("cg"),
        "Switzerland" => Some("ch"),
        "Ivory Coast" => Some("ci"),
        "Cook Islands" => Some("ck"),
        "Chile" => Some("cl"),
        "Cameroon" => Some("cm"),
        "China" => Some("cn"),
        "Colombia" => Some("co"),
        "Costa Rica" => Some("cr"),
        "Cuba" => Some("cu"),
        "Cape Verde" => Some("cv"),
        "Curacao" => Some("cw"),
        "Christmas Island" => Some("cx"),
        "Cyprus" => Some("cy"),
        "Czech Republic" => Some("cz"),
        "Germany" => Some("de"),
        "Djibouti" => Some("dj"),
        "Denmark" => Some("dk"),
        "Dominica" => Some("dm"),
        "Dominican Republic" => Some("do"),
        "Algeria" => Some("dz"),
        "Ecuador" => Some("ec"),
        "Estonia" => Some("ee"),
        "Egypt" => Some("eg"),
        "Western Sahara" => Some("eh"),
        "Eritrea" => Some("er"),
        "Spain" => Some("es"),
        "Ethiopia" => Some("et"),
        "Finland" => Some("fi"),
        "Fiji" => Some("fj"),
        "Falkland Islands" => Some("fk"),
        "Micronesia" => Some("fm"),
        "Faroe Islands" => Some("fo"),
        "France" => Some("fr"),
        "Gabon" => Some("ga"),
        "United Kingdom" => Some("gb"),
        "Grenada" => Some("gd"),
        "Georgia" => Some("ge"),
        "French Guiana" => Some("gf"),
        "Guernsey" => Some("gg"),
        "Ghana" => Some("gh"),
        "Gibraltar" => Some("gi"),
        "Greenland" => Some("gl"),
        "Gambia" => Some("gm"),
        "Guinea" => Some("gn"),
        "Guadeloupe" => Some("gp"),
        "Equatorial Guinea" => Some("gq"),
        "Greece" => Some("gr"),
        "South Georgia and the South Sandwich Islands" => Some("gs"),
        "Guatemala" => Some("gt"),
        "Guam" => Some("gu"),
        "Guinea-Bissau" => Some("gw"),
        "Guyana" => Some("gy"),
        "Hong Kong" => Some("hk"),
        "Heard Island and McDonald Islands" => Some("hm"),
        "Honduras" => Some("hn"),
        "Croatia" => Some("hr"),
        "Haiti" => Some("ht"),
        "Hungary" => Some("hu"),
        "Indonesia" => Some("id"),
        "Ireland" => Some("ie"),
        "Israel" => Some("il"),
        "Isle of Man" => Some("im"),
        "India" => Some("in"),
        "British Indian Ocean Territory" => Some("io"),
        "Iraq" => Some("iq"),
        "Iran" => Some("ir"),
        "Iceland" => Some("is"),
        "Italy" => Some("it"),
        "Jersey" => Some("je"),
        "Jamaica" => Some("jm"),
        "Jordan" => Some("jo"),
        "Japan" => Some("jp"),
        "Kenya" => Some("ke"),
        "Kyrgyzstan" => Some("kg"),
        "Cambodia" => Some("kh"),
        "Kiribati" => Some("ki"),
        "Comoros" => Some("km"),
        "Saint Kitts and Nevis" => Some("kn"),
        "North Korea" => Some("kp"),
        "South Korea" => Some("kr"),
        "Kuwait" => Some("kw"),
        "Cayman Islands" => Some("ky"),
        "Kazakhstan" => Some("kz"),
        "Laos" => Some("la"),
        "Lebanon" => Some("lb"),
        "Saint Lucia" => Some("lc"),
        "Liechtenstein" => Some("li"),
        "Sri Lanka" => Some("lk"),
        "Liberia" => Some("lr"),
        "Lesotho" => Some("ls"),
        "Lithuania" => Some("lt"),
        "Luxembourg" => Some("lu"),
        "Latvia" => Some("lv"),
        "Libya" => Some("ly"),
        "Morocco" => Some("ma"),
        "Monaco" => Some("mc"),
        "Moldova" => Some("md"),
        "Montenegro" => Some("me"),
        "Saint Martin" => Some("mf"),
        "Madagascar" => Some("mg"),
        "Marshall Islands" => Some("mh"),
        "North Macedonia" => Some("mk"),
        "Mali" => Some("ml"),
        "Myanmar" => Some("mm"),
        "Mongolia" => Some("mn"),
        "Macau" => Some("mo"),
        "Northern Mariana Islands" => Some("mp"),
        "Martinique" => Some("mq"),
        "Mauritania" => Some("mr"),
        "Montserrat" => Some("ms"),
        "Malta" => Some("mt"),
        "Mauritius" => Some("mu"),
        "Maldives" => Some("mv"),
        "Malawi" => Some("mw"),
        "Mexico" => Some("mx"),
        "Malaysia" => Some("my"),
        "Mozambique" => Some("mz"),
        "Namibia" => Some("na"),
        "New Caledonia" => Some("nc"),
        "Niger" => Some("ne"),
        "Norfolk Island" => Some("nf"),
        "Nigeria" => Some("ng"),
        "Nicaragua" => Some("ni"),
        "Netherlands" => Some("nl"),
        "Norway" => Some("no"),
        "Nepal" => Some("np"),
        "Nauru" => Some("nr"),
        "Niue" => Some("nu"),
        "New Zealand" => Some("nz"),
        "Oman" => Some("om"),
        "Panama" => Some("pa"),
        "Peru" => Some("pe"),
        "French Polynesia" => Some("pf"),
        "Papua New Guinea" => Some("pg"),
        "Philippines" => Some("ph"),
        "Pakistan" => Some("pk"),
        "Poland" => Some("pl"),
        "Saint Pierre and Miquelon" => Some("pm"),
        "Pitcairn Islands" => Some("pn"),
        "Puerto Rico" => Some("pr"),
        "Palestine" => Some("ps"),
        "Portugal" => Some("pt"),
        "Palau" => Some("pw"),
        "Paraguay" => Some("py"),
        "Qatar" => Some("qa"),
        "Reunion" => Some("re"),
        "Romania" => Some("ro"),
        "Serbia" => Some("rs"),
        "Russia" => Some("ru"),
        "Rwanda" => Some("rw"),
        "Saudi Arabia" => Some("sa"),
        "Solomon Islands" => Some("sb"),
        "Seychelles" => Some("sc"),
        "Sudan" => Some("sd"),
        "Sweden" => Some("se"),
        "Singapore" => Some("sg"),
        "Saint Helena" => Some("sh"),
        "Slovenia" => Some("si"),
        "Svalbard and Jan Mayen" => Some("sj"),
        "Slovakia" => Some("sk"),
        "Sierra Leone" => Some("sl"),
        "San Marino" => Some("sm"),
        "Senegal" => Some("sn"),
        "Somalia" => Some("so"),
        "Suriname" => Some("sr"),
        "South Sudan" => Some("ss"),
        "Sao Tome and Principe" => Some("st"),
        "El Salvador" => Some("sv"),
        "Sint Maarten" => Some("sx"),
        "Syria" => Some("sy"),
        "Eswatini" => Some("sz"),
        "Turks and Caicos Islands" => Some("tc"),
        "Chad" => Some("td"),
        "French Southern Territories" => Some("tf"),
        "Togo" => Some("tg"),
        "Thailand" => Some("th"),
        "Tajikistan" => Some("tj"),
        "Tokelau" => Some("tk"),
        "East Timor" => Some("tl"),
        "Turkmenistan" => Some("tm"),
        "Tunisia" => Some("tn"),
        "Tonga" => Some("to"),
        "Turkey" => Some("tr"),
        "Trinidad and Tobago" => Some("tt"),
        "Tuvalu" => Some("tv"),
        "Taiwan" => Some("tw"),
        "Tanzania" => Some("tz"),
        "Ukraine" => Some("ua"),
        "Uganda" => Some("ug"),
        "United States Minor Outlying Islands" => Some("um"),
        "United States" => Some("us"),
        "Uruguay" => Some("uy"),
        "Uzbekistan" => Some("uz"),
        "Vatican City" => Some("va"),
        "Saint Vincent and the Grenadines" => Some("vc"),
        "Venezuela" => Some("ve"),
        "British Virgin Islands" => Some("vg"),
        "United States Virgin Islands" => Some("vi"),
        "Vietnam" => Some("vn"),
        "Vanuatu" => Some("vu"),
        "Wallis and Futuna" => Some("wf"),
        "Samoa" => Some("ws"),
        "Kosovo" => Some("xk"),
        "Yemen" => Some("ye"),
        "Mayotte" => Some("yt"),
        "South Africa" => Some("za"),
        "Zambia" => Some("zm"),
        "Zimbabwe" => Some("zw"),
        _ => None,
    }
}

// pub fn get_name_from_iso(iso_code: &str) -> Option<&str> {
//     match iso_code {
//         "ad" => Some("Andorra"),
//         "ae" => Some("United Arab Emirates"),
//         "af" => Some("Afghanistan"),
//         "ag" => Some("Antigua and Barbuda"),
//         "ai" => Some("Anguilla"),
//         "al" => Some("Albania"),
//         "am" => Some("Armenia"),
//         "ao" => Some("Angola"),
//         "aq" => Some("Antarctica"),
//         "ar" => Some("Argentina"),
//         "as" => Some("American Samoa"),
//         "at" => Some("Austria"),
//         "au" => Some("Australia"),
//         "aw" => Some("Aruba"),
//         "ax" => Some("Aland Islands"),
//         "az" => Some("Azerbaijan"),
//         "ba" => Some("Bosnia and Herzegovina"),
//         "bb" => Some("Barbados"),
//         "bd" => Some("Bangladesh"),
//         "be" => Some("Belgium"),
//         "bf" => Some("Burkina Faso"),
//         "bg" => Some("Bulgaria"),
//         "bh" => Some("Bahrain"),
//         "bi" => Some("Burundi"),
//         "bj" => Some("Benin"),
//         "bl" => Some("Saint Barthelemy"),
//         "bm" => Some("Bermuda"),
//         "bn" => Some("Brunei"),
//         "bo" => Some("Bolivia"),
//         "bq" => Some("Bonaire, Sint Eustatius, and Saba"),
//         "br" => Some("Brazil"),
//         "bs" => Some("Bahamas"),
//         "bt" => Some("Bhutan"),
//         "bv" => Some("Bouvet Island"),
//         "bw" => Some("Botswana"),
//         "by" => Some("Belarus"),
//         "bz" => Some("Belize"),
//         "ca" => Some("Canada"),
//         "cc" => Some("Cocos (Keeling) Islands"),
//         "cd" => Some("Democratic Republic of the Congo"),
//         "cf" => Some("Central African Republic"),
//         "cg" => Some("Republic of the Congo"),
//         "ch" => Some("Switzerland"),
//         "ci" => Some("Ivory Coast"),
//         "ck" => Some("Cook Islands"),
//         "cl" => Some("Chile"),
//         "cm" => Some("Cameroon"),
//         "cn" => Some("China"),
//         "co" => Some("Colombia"),
//         "cr" => Some("Costa Rica"),
//         "cu" => Some("Cuba"),
//         "cv" => Some("Cape Verde"),
//         "cw" => Some("Curacao"),
//         "cx" => Some("Christmas Island"),
//         "cy" => Some("Cyprus"),
//         "cz" => Some("Czech Republic"),
//         "de" => Some("Germany"),
//         "dj" => Some("Djibouti"),
//         "dk" => Some("Denmark"),
//         "dm" => Some("Dominica"),
//         "do" => Some("Dominican Republic"),
//         "dz" => Some("Algeria"),
//         "ec" => Some("Ecuador"),
//         "ee" => Some("Estonia"),
//         "eg" => Some("Egypt"),
//         "eh" => Some("Western Sahara"),
//         "er" => Some("Eritrea"),
//         "es" => Some("Spain"),
//         "et" => Some("Ethiopia"),
//         "fi" => Some("Finland"),
//         "fj" => Some("Fiji"),
//         "fk" => Some("Falkland Islands"),
//         "fm" => Some("Micronesia"),
//         "fo" => Some("Faroe Islands"),
//         "fr" => Some("France"),
//         "ga" => Some("Gabon"),
//         "gb" => Some("United Kingdom"),
//         "gd" => Some("Grenada"),
//         "ge" => Some("Georgia"),
//         "gf" => Some("French Guiana"),
//         "gg" => Some("Guernsey"),
//         "gh" => Some("Ghana"),
//         "gi" => Some("Gibraltar"),
//         "gl" => Some("Greenland"),
//         "gm" => Some("Gambia"),
//         "gn" => Some("Guinea"),
//         "gp" => Some("Guadeloupe"),
//         "gq" => Some("Equatorial Guinea"),
//         "gr" => Some("Greece"),
//         "gs" => Some("South Georgia and the South Sandwich Islands"),
//         "gt" => Some("Guatemala"),
//         "gu" => Some("Guam"),
//         "gw" => Some("Guinea-Bissau"),
//         "gy" => Some("Guyana"),
//         "hk" => Some("Hong Kong"),
//         "hm" => Some("Heard Island and McDonald Islands"),
//         "hn" => Some("Honduras"),
//         "hr" => Some("Croatia"),
//         "ht" => Some("Haiti"),
//         "hu" => Some("Hungary"),
//         "id" => Some("Indonesia"),
//         "ie" => Some("Ireland"),
//         "il" => Some("Israel"),
//         "im" => Some("Isle of Man"),
//         "in" => Some("India"),
//         "io" => Some("British Indian Ocean Territory"),
//         "iq" => Some("Iraq"),
//         "ir" => Some("Iran"),
//         "is" => Some("Iceland"),
//         "it" => Some("Italy"),
//         "je" => Some("Jersey"),
//         "jm" => Some("Jamaica"),
//         "jo" => Some("Jordan"),
//         "jp" => Some("Japan"),
//         "ke" => Some("Kenya"),
//         "kg" => Some("Kyrgyzstan"),
//         "kh" => Some("Cambodia"),
//         "ki" => Some("Kiribati"),
//         "km" => Some("Comoros"),
//         "kn" => Some("Saint Kitts and Nevis"),
//         "kp" => Some("North Korea"),
//         "kr" => Some("South Korea"),
//         "kw" => Some("Kuwait"),
//         "ky" => Some("Cayman Islands"),
//         "kz" => Some("Kazakhstan"),
//         "la" => Some("Laos"),
//         "lb" => Some("Lebanon"),
//         "lc" => Some("Saint Lucia"),
//         "li" => Some("Liechtenstein"),
//         "lk" => Some("Sri Lanka"),
//         "lr" => Some("Liberia"),
//         "ls" => Some("Lesotho"),
//         "lt" => Some("Lithuania"),
//         "lu" => Some("Luxembourg"),
//         "lv" => Some("Latvia"),
//         "ly" => Some("Libya"),
//         "ma" => Some("Morocco"),
//         "mc" => Some("Monaco"),
//         "md" => Some("Moldova"),
//         "me" => Some("Montenegro"),
//         "mf" => Some("Saint Martin"),
//         "mg" => Some("Madagascar"),
//         "mh" => Some("Marshall Islands"),
//         "mk" => Some("North Macedonia"),
//         "ml" => Some("Mali"),
//         "mm" => Some("Myanmar"),
//         "mn" => Some("Mongolia"),
//         "mo" => Some("Macau"),
//         "mp" => Some("Northern Mariana Islands"),
//         "mq" => Some("Martinique"),
//         "mr" => Some("Mauritania"),
//         "ms" => Some("Montserrat"),
//         "mt" => Some("Malta"),
//         "mu" => Some("Mauritius"),
//         "mv" => Some("Maldives"),
//         "mw" => Some("Malawi"),
//         "mx" => Some("Mexico"),
//         "my" => Some("Malaysia"),
//         "mz" => Some("Mozambique"),
//         "na" => Some("Namibia"),
//         "nc" => Some("New Caledonia"),
//         "ne" => Some("Niger"),
//         "nf" => Some("Norfolk Island"),
//         "ng" => Some("Nigeria"),
//         "ni" => Some("Nicaragua"),
//         "nl" => Some("Netherlands"),
//         "no" => Some("Norway"),
//         "np" => Some("Nepal"),
//         "nr" => Some("Nauru"),
//         "nu" => Some("Niue"),
//         "nz" => Some("New Zealand"),
//         "om" => Some("Oman"),
//         "pa" => Some("Panama"),
//         "pe" => Some("Peru"),
//         "pf" => Some("French Polynesia"),
//         "pg" => Some("Papua New Guinea"),
//         "ph" => Some("Philippines"),
//         "pk" => Some("Pakistan"),
//         "pl" => Some("Poland"),
//         "pm" => Some("Saint Pierre and Miquelon"),
//         "pn" => Some("Pitcairn Islands"),
//         "pr" => Some("Puerto Rico"),
//         "ps" => Some("Palestine"),
//         "pt" => Some("Portugal"),
//         "pw" => Some("Palau"),
//         "py" => Some("Paraguay"),
//         "qa" => Some("Qatar"),
//         "re" => Some("Reunion"),
//         "ro" => Some("Romania"),
//         "rs" => Some("Serbia"),
//         "ru" => Some("Russia"),
//         "rw" => Some("Rwanda"),
//         "sa" => Some("Saudi Arabia"),
//         "sb" => Some("Solomon Islands"),
//         "sc" => Some("Seychelles"),
//         "sd" => Some("Sudan"),
//         "se" => Some("Sweden"),
//         "sg" => Some("Singapore"),
//         "sh" => Some("Saint Helena"),
//         "si" => Some("Slovenia"),
//         "sj" => Some("Svalbard and Jan Mayen"),
//         "sk" => Some("Slovakia"),
//         "sl" => Some("Sierra Leone"),
//         "sm" => Some("San Marino"),
//         "sn" => Some("Senegal"),
//         "so" => Some("Somalia"),
//         "sr" => Some("Suriname"),
//         "ss" => Some("South Sudan"),
//         "st" => Some("Sao Tome and Principe"),
//         "sv" => Some("El Salvador"),
//         "sx" => Some("Sint Maarten"),
//         "sy" => Some("Syria"),
//         "sz" => Some("Eswatini"),
//         "tc" => Some("Turks and Caicos Islands"),
//         "td" => Some("Chad"),
//         "tf" => Some("French Southern Territories"),
//         "tg" => Some("Togo"),
//         "th" => Some("Thailand"),
//         "tj" => Some("Tajikistan"),
//         "tk" => Some("Tokelau"),
//         "tl" => Some("East Timor"),
//         "tm" => Some("Turkmenistan"),
//         "tn" => Some("Tunisia"),
//         "to" => Some("Tonga"),
//         "tr" => Some("Turkey"),
//         "tt" => Some("Trinidad and Tobago"),
//         "tv" => Some("Tuvalu"),
//         "tw" => Some("Taiwan"),
//         "tz" => Some("Tanzania"),
//         "ua" => Some("Ukraine"),
//         "ug" => Some("Uganda"),
//         "um" => Some("United States Minor Outlying Islands"),
//         "us" => Some("United States"),
//         "uy" => Some("Uruguay"),
//         "uz" => Some("Uzbekistan"),
//         "va" => Some("Vatican City"),
//         "vc" => Some("Saint Vincent and the Grenadines"),
//         "ve" => Some("Venezuela"),
//         "vg" => Some("British Virgin Islands"),
//         "vi" => Some("United States Virgin Islands"),
//         "vn" => Some("Vietnam"),
//         "vu" => Some("Vanuatu"),
//         "wf" => Some("Wallis and Futuna"),
//         "ws" => Some("Samoa"),
//         "xk" => Some("Kosovo"),
//         "ye" => Some("Yemen"),
//         "yt" => Some("Mayotte"),
//         "za" => Some("South Africa"),
//         "zm" => Some("Zambia"),
//         "zw" => Some("Zimbabwe"),
//         _ => None,
//     }
// }
