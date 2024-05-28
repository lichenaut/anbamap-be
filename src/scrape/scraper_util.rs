use super::regions::KEYPHRASE_REGION_MAP;
use super::scrapers::youtube::scrape_youtube_channel;
use crate::util::var_service::{get_youtube_api_key, get_youtube_channel_ids};
use crate::{db::redis::update_db, util::path_service::get_parent_dir};
use rayon::prelude::*;
use std::{
    error::Error,
    process::Command,
    str,
    sync::{Arc, Mutex},
};
use unidecode::unidecode;

pub async fn run_scrapers() -> Result<(), Box<dyn Error>> {
    let mut media = Vec::new();
    scrape_youtube(&mut media).await?;
    update_db(media).await?;

    Ok(())
}

async fn scrape_youtube(
    media: &mut Vec<(String, String, String, Vec<String>)>,
) -> Result<(), Box<dyn Error>> {
    let youtube_api_key = match get_youtube_api_key().await? {
        Some(api_key) => api_key,
        None => return Ok(()),
    };

    let youtube_channel_ids = match get_youtube_channel_ids().await? {
        Some(channel_ids) => channel_ids,
        None => return Ok(()),
    };

    let youtube_channel_ids = youtube_channel_ids
        .split(",")
        .filter(|&s| !s.is_empty())
        .collect::<Vec<&str>>();
    for youtube_channel_id in youtube_channel_ids {
        media.extend(scrape_youtube_channel(&youtube_api_key, &youtube_channel_id).await?);
    }

    Ok(())
}

pub async fn get_regions(text: &[&str]) -> Result<Vec<String>, Box<dyn Error>> {
    let text = text
        .join(" ")
        .replace("'", r"\'")
        .replace("&#39;", r"\'")
        .replace("\"", "")
        .replace("`", "")
        .replace("‘", "")
        .replace("’", "")
        .replace("–", "")
        .replace("'s ", " ")
        .replace("s' ", " ");
    let regions = get_flashgeotext_regions(&text).await?;

    let text = unidecode(&text.replace(r"\'", "'").to_lowercase());
    let regions = Arc::new(Mutex::new(regions));
    KEYPHRASE_REGION_MAP
        .par_iter()
        .for_each(|(keyphrases, region)| {
            let regions = Arc::clone(&regions);
            let mut regions_locked = match regions.lock() {
                Ok(regions) => regions,
                Err(poisoned) => {
                    tracing::error!("Poisoned lock: {:?}", poisoned);
                    return;
                }
            };

            if regions_locked.contains(&region.to_string()) {
                return;
            }

            for keyphrase in keyphrases.iter() {
                if text.contains(keyphrase) {
                    regions_locked.push(region.to_string());
                    break;
                }
            }
        });

    let mut regions = match regions.lock() {
        Ok(regions) => regions.clone(),
        Err(poisoned) => {
            tracing::error!("Poisoned lock: {:?}", poisoned);
            Vec::new()
        }
    };

    if text.contains("georgia") && !regions.contains(&"United States".into()) {
        regions.push("Georgia".into());
    }
    if text.contains("ireland") && !text.contains("northern ireland") {
        regions.push("Ireland".into());
    }
    if text.contains("mexico") && !text.contains("new mexico") {
        regions.push("Mexico".into());
    }

    Ok(regions)
}

async fn get_flashgeotext_regions(text: &String) -> Result<Vec<String>, Box<dyn Error>> {
    let exe_parent = get_parent_dir().await?;
    let regions = Command::new(format!("{}/p3venv/bin/python", exe_parent))
        .arg("-c")
        .arg(format!(
            "import sys; sys.path.append('{}'); from media_to_regions import get_regions; print(get_regions('{}'))",
            exe_parent, text
        ))
        .output()?;

    let output = str::from_utf8(&regions.stdout)?;
    if output.is_empty() {
        tracing::error!("Flashgeotext error from body: {}", text);
        return Ok(Vec::new());
    }

    if output == "[]\n" {
        return Ok(Vec::new());
    }

    let output: Vec<String> = output
        .trim()
        .replace("[", "")
        .replace("]", "")
        .replace("'", "")
        .split(", ")
        .map(|s| s.to_string())
        .collect();
    let regions = output
        .into_iter()
        .filter(|s| match s.as_str() {
            "Chad" | "Georgia" | "Guinea-Bissau" | "Jordan" | "Republic of Congo" => false,
            _ => true,
        })
        .collect::<Vec<String>>();

    Ok(regions)
}

pub fn get_iso_from_name(name: &str) -> Option<&str> {
    match name {
        "Andorra" => Some("AD"),
        "United Arab Emirates" => Some("AE"),
        "Afghanistan" => Some("AF"),
        "Antigua and Barbuda" => Some("AG"),
        "Anguilla" => Some("AI"),
        "Albania" => Some("AL"),
        "Armenia" => Some("AM"),
        "Angola" => Some("AO"),
        "Antarctica" => Some("AQ"),
        "Argentina" => Some("AR"),
        "American Samoa" => Some("AS"),
        "Austria" => Some("AT"),
        "Australia" => Some("AU"),
        "Aruba" => Some("AW"),
        "Aland Islands" => Some("AX"),
        "Azerbaijan" => Some("AZ"),
        "Bosnia and Herzegovina" => Some("BA"),
        "Barbados" => Some("BB"),
        "Bangladesh" => Some("BD"),
        "Belgium" => Some("BE"),
        "Burkina Faso" => Some("BF"),
        "Bulgaria" => Some("BG"),
        "Bahrain" => Some("BH"),
        "Burundi" => Some("BI"),
        "Benin" => Some("BJ"),
        "Saint Barthelemy" => Some("BL"),
        "Bermuda" => Some("BM"),
        "Brunei" => Some("BN"),
        "Bolivia" => Some("BO"),
        "Bonaire, Sint Eustatius, and Saba" => Some("BQ"),
        "Brazil" => Some("BR"),
        "Bahamas" => Some("BS"),
        "Bhutan" => Some("BT"),
        "Bouvet Island" => Some("BV"),
        "Botswana" => Some("BW"),
        "Belarus" => Some("BY"),
        "Belize" => Some("BZ"),
        "Canada" => Some("CA"),
        "Cocos (Keeling) Islands" => Some("CC"),
        "Democratic Republic of the Congo" => Some("CD"),
        "Central African Republic" => Some("CF"),
        "Republic of the Congo" => Some("CG"),
        "Switzerland" => Some("CH"),
        "Ivory Coast" => Some("CI"),
        "Cook Islands" => Some("CK"),
        "Chile" => Some("CL"),
        "Cameroon" => Some("CM"),
        "China" => Some("CN"),
        "Colombia" => Some("CO"),
        "Costa Rica" => Some("CR"),
        "Cuba" => Some("CU"),
        "Cape Verde" => Some("CV"),
        "Curacao" => Some("CW"),
        "Christmas Island" => Some("CX"),
        "Cyprus" => Some("CY"),
        "Czech Republic" => Some("CZ"),
        "Germany" => Some("DE"),
        "Djibouti" => Some("DJ"),
        "Denmark" => Some("DK"),
        "Dominica" => Some("DM"),
        "Dominican Republic" => Some("DO"),
        "Algeria" => Some("DZ"),
        "Ecuador" => Some("EC"),
        "Estonia" => Some("EE"),
        "Egypt" => Some("EG"),
        "Western Sahara" => Some("EH"),
        "Eritrea" => Some("ER"),
        "Spain" => Some("ES"),
        "Ethiopia" => Some("ET"),
        "Finland" => Some("FI"),
        "Fiji" => Some("FJ"),
        "Falkland Islands" => Some("FK"),
        "Micronesia" => Some("FM"),
        "Faroe Islands" => Some("FO"),
        "France" => Some("FR"),
        "Gabon" => Some("GA"),
        "United Kingdom" => Some("GB"),
        "Grenada" => Some("GD"),
        "Georgia" => Some("GE"),
        "French Guiana" => Some("GF"),
        "Guernsey" => Some("GG"),
        "Ghana" => Some("GH"),
        "Gibraltar" => Some("GI"),
        "Greenland" => Some("GL"),
        "Gambia" => Some("GM"),
        "Guinea" => Some("GN"),
        "Guadeloupe" => Some("GP"),
        "Equatorial Guinea" => Some("GQ"),
        "Greece" => Some("GR"),
        "South Georgia and the South Sandwich Islands" => Some("GS"),
        "Guatemala" => Some("GT"),
        "Guam" => Some("GU"),
        "Guinea-Bissau" => Some("GW"),
        "Guyana" => Some("GY"),
        "Hong Kong" => Some("HK"),
        "Heard Island and McDonald Islands" => Some("HM"),
        "Honduras" => Some("HN"),
        "Croatia" => Some("HR"),
        "Haiti" => Some("HT"),
        "Hungary" => Some("HU"),
        "Indonesia" => Some("ID"),
        "Ireland" => Some("IE"),
        "Israel" => Some("IL"),
        "Isle of Man" => Some("IM"),
        "India" => Some("IN"),
        "British Indian Ocean Territory" => Some("IO"),
        "Iraq" => Some("IQ"),
        "Iran" => Some("IR"),
        "Iceland" => Some("IS"),
        "Italy" => Some("IT"),
        "Jersey" => Some("JE"),
        "Jamaica" => Some("JM"),
        "Jordan" => Some("JO"),
        "Japan" => Some("JP"),
        "Kenya" => Some("KE"),
        "Kyrgyzstan" => Some("KG"),
        "Cambodia" => Some("KH"),
        "Kiribati" => Some("KI"),
        "Comoros" => Some("KM"),
        "Saint Kitts and Nevis" => Some("KN"),
        "North Korea" => Some("KP"),
        "South Korea" => Some("KR"),
        "Kuwait" => Some("KW"),
        "Cayman Islands" => Some("KY"),
        "Kazakhstan" => Some("KZ"),
        "Laos" => Some("LA"),
        "Lebanon" => Some("LB"),
        "Saint Lucia" => Some("LC"),
        "Liechtenstein" => Some("LI"),
        "Sri Lanka" => Some("LK"),
        "Liberia" => Some("LR"),
        "Lesotho" => Some("LS"),
        "Lithuania" => Some("LT"),
        "Luxembourg" => Some("LU"),
        "Latvia" => Some("LV"),
        "Libya" => Some("LY"),
        "Morocco" => Some("MA"),
        "Monaco" => Some("MC"),
        "Moldova" => Some("MD"),
        "Montenegro" => Some("ME"),
        "Saint Martin" => Some("MF"),
        "Madagascar" => Some("MG"),
        "Marshall Islands" => Some("MH"),
        "North Macedonia" => Some("MK"),
        "Mali" => Some("ML"),
        "Myanmar" => Some("MM"),
        "Mongolia" => Some("MN"),
        "Macau" => Some("MO"),
        "Northern Mariana Islands" => Some("MP"),
        "Martinique" => Some("MQ"),
        "Mauritania" => Some("MR"),
        "Montserrat" => Some("MS"),
        "Malta" => Some("MT"),
        "Mauritius" => Some("MU"),
        "Maldives" => Some("MV"),
        "Malawi" => Some("MW"),
        "Mexico" => Some("MX"),
        "Malaysia" => Some("MY"),
        "Mozambique" => Some("MZ"),
        "Namibia" => Some("NA"),
        "New Caledonia" => Some("NC"),
        "Niger" => Some("NE"),
        "Norfolk Island" => Some("NF"),
        "Nigeria" => Some("NG"),
        "Nicaragua" => Some("NI"),
        "Netherlands" => Some("NL"),
        "Norway" => Some("NO"),
        "Nepal" => Some("NP"),
        "Nauru" => Some("NR"),
        "Niue" => Some("NU"),
        "New Zealand" => Some("NZ"),
        "Oman" => Some("OM"),
        "Panama" => Some("PA"),
        "Peru" => Some("PE"),
        "French Polynesia" => Some("PF"),
        "Papua New Guinea" => Some("PG"),
        "Philippines" => Some("PH"),
        "Pakistan" => Some("PK"),
        "Poland" => Some("PL"),
        "Saint Pierre and Miquelon" => Some("PM"),
        "Pitcairn Islands" => Some("PN"),
        "Puerto Rico" => Some("PR"),
        "Palestine" => Some("PS"),
        "Portugal" => Some("PT"),
        "Palau" => Some("PW"),
        "Paraguay" => Some("PY"),
        "Qatar" => Some("QA"),
        "Reunion" => Some("RE"),
        "Romania" => Some("RO"),
        "Serbia" => Some("RS"),
        "Russia" => Some("RU"),
        "Rwanda" => Some("RW"),
        "Saudi Arabia" => Some("SA"),
        "Solomon Islands" => Some("SB"),
        "Seychelles" => Some("SC"),
        "Sudan" => Some("SD"),
        "Sweden" => Some("SE"),
        "Singapore" => Some("SG"),
        "Saint Helena" => Some("SH"),
        "Slovenia" => Some("SI"),
        "Svalbard and Jan Mayen" => Some("SJ"),
        "Slovakia" => Some("SK"),
        "Sierra Leone" => Some("SL"),
        "San Marino" => Some("SM"),
        "Senegal" => Some("SN"),
        "Somalia" => Some("SO"),
        "Suriname" => Some("SR"),
        "South Sudan" => Some("SS"),
        "Sao Tome and Principe" => Some("ST"),
        "El Salvador" => Some("SV"),
        "Sint Maarten" => Some("SX"),
        "Syria" => Some("SY"),
        "Eswatini" => Some("SZ"),
        "Turks and Caicos Islands" => Some("TC"),
        "Chad" => Some("TD"),
        "French Southern Territories" => Some("TF"),
        "Togo" => Some("TG"),
        "Thailand" => Some("TH"),
        "Tajikistan" => Some("TJ"),
        "Tokelau" => Some("TK"),
        "East Timor" => Some("TL"),
        "Turkmenistan" => Some("TM"),
        "Tunisia" => Some("TN"),
        "Tonga" => Some("TO"),
        "Turkey" => Some("TR"),
        "Trinidad and Tobago" => Some("TT"),
        "Tuvalu" => Some("TV"),
        "Taiwan" => Some("TW"),
        "Tanzania" => Some("TZ"),
        "Ukraine" => Some("UA"),
        "Uganda" => Some("UG"),
        "United States Minor Outlying Islands" => Some("UM"),
        "United States" => Some("US"),
        "Uruguay" => Some("UY"),
        "Uzbekistan" => Some("UZ"),
        "Vatican City" => Some("VA"),
        "Saint Vincent and the Grenadines" => Some("VC"),
        "Venezuela" => Some("VE"),
        "British Virgin Islands" => Some("VG"),
        "United States Virgin Islands" => Some("VI"),
        "Vietnam" => Some("VN"),
        "Vanuatu" => Some("VU"),
        "Wallis and Futuna" => Some("WF"),
        "Samoa" => Some("WS"),
        "Yemen" => Some("YE"),
        "Mayotte" => Some("YT"),
        "Kosovo" => Some("XK"),
        "South Africa" => Some("ZA"),
        "Zambia" => Some("ZM"),
        "Zimbabwe" => Some("ZW"),
        _ => None,
    }
}

// pub fn get_name_from_iso(iso_code: &str) -> Option<&str> {
//     match iso_code {
//         "AD" => Some("Andorra"),
//         "AE" => Some("United Arab Emirates"),
//         "AF" => Some("Afghanistan"),
//         "AG" => Some("Antigua and Barbuda"),
//         "AI" => Some("Anguilla"),
//         "AL" => Some("Albania"),
//         "AM" => Some("Armenia"),
//         "AO" => Some("Angola"),
//         "AQ" => Some("Antarctica"),
//         "AR" => Some("Argentina"),
//         "AS" => Some("American Samoa"),
//         "AT" => Some("Austria"),
//         "AU" => Some("Australia"),
//         "AW" => Some("Aruba"),
//         "AX" => Some("Aland Islands"),
//         "AZ" => Some("Azerbaijan"),
//         "BA" => Some("Bosnia and Herzegovina"),
//         "BB" => Some("Barbados"),
//         "BD" => Some("Bangladesh"),
//         "BE" => Some("Belgium"),
//         "BF" => Some("Burkina Faso"),
//         "BG" => Some("Bulgaria"),
//         "BH" => Some("Bahrain"),
//         "BI" => Some("Burundi"),
//         "BJ" => Some("Benin"),
//         "BL" => Some("Saint Barthelemy"),
//         "BM" => Some("Bermuda"),
//         "BN" => Some("Brunei"),
//         "BO" => Some("Bolivia"),
//         "BQ" => Some("Bonaire, Sint Eustatius, and Saba"),
//         "BR" => Some("Brazil"),
//         "BS" => Some("Bahamas"),
//         "BT" => Some("Bhutan"),
//         "BV" => Some("Bouvet Island"),
//         "BW" => Some("Botswana"),
//         "BY" => Some("Belarus"),
//         "BZ" => Some("Belize"),
//         "CA" => Some("Canada"),
//         "CC" => Some("Cocos (Keeling) Islands"),
//         "CD" => Some("Democratic Republic of the Congo"),
//         "CF" => Some("Central African Republic"),
//         "CG" => Some("Republic of the Congo"),
//         "CH" => Some("Switzerland"),
//         "CI" => Some("Ivory Coast"),
//         "CK" => Some("Cook Islands"),
//         "CL" => Some("Chile"),
//         "CM" => Some("Cameroon"),
//         "CN" => Some("China"),
//         "CO" => Some("Colombia"),
//         "CR" => Some("Costa Rica"),
//         "CU" => Some("Cuba"),
//         "CV" => Some("Cape Verde"),
//         "CW" => Some("Curacao"),
//         "CX" => Some("Christmas Island"),
//         "CY" => Some("Cyprus"),
//         "CZ" => Some("Czech Republic"),
//         "DE" => Some("Germany"),
//         "DJ" => Some("Djibouti"),
//         "DK" => Some("Denmark"),
//         "DM" => Some("Dominica"),
//         "DO" => Some("Dominican Republic"),
//         "DZ" => Some("Algeria"),
//         "EC" => Some("Ecuador"),
//         "EE" => Some("Estonia"),
//         "EG" => Some("Egypt"),
//         "EH" => Some("Western Sahara"),
//         "ER" => Some("Eritrea"),
//         "ES" => Some("Spain"),
//         "ET" => Some("Ethiopia"),
//         "FI" => Some("Finland"),
//         "FJ" => Some("Fiji"),
//         "FK" => Some("Falkland Islands"),
//         "FM" => Some("Micronesia"),
//         "FO" => Some("Faroe Islands"),
//         "FR" => Some("France"),
//         "GA" => Some("Gabon"),
//         "GB" => Some("United Kingdom"),
//         "GD" => Some("Grenada"),
//         "GE" => Some("Georgia"),
//         "GF" => Some("French Guiana"),
//         "GG" => Some("Guernsey"),
//         "GH" => Some("Ghana"),
//         "GI" => Some("Gibraltar"),
//         "GL" => Some("Greenland"),
//         "GM" => Some("Gambia"),
//         "GN" => Some("Guinea"),
//         "GP" => Some("Guadeloupe"),
//         "GQ" => Some("Equatorial Guinea"),
//         "GR" => Some("Greece"),
//         "GS" => Some("South Georgia and the South Sandwich Islands"),
//         "GT" => Some("Guatemala"),
//         "GU" => Some("Guam"),
//         "GW" => Some("Guinea-Bissau"),
//         "GY" => Some("Guyana"),
//         "HK" => Some("Hong Kong"),
//         "HM" => Some("Heard Island and McDonald Islands"),
//         "HN" => Some("Honduras"),
//         "HR" => Some("Croatia"),
//         "HT" => Some("Haiti"),
//         "HU" => Some("Hungary"),
//         "ID" => Some("Indonesia"),
//         "IE" => Some("Ireland"),
//         "IL" => Some("Israel"),
//         "IM" => Some("Isle of Man"),
//         "IN" => Some("India"),
//         "IO" => Some("British Indian Ocean Territory"),
//         "IQ" => Some("Iraq"),
//         "IR" => Some("Iran"),
//         "IS" => Some("Iceland"),
//         "IT" => Some("Italy"),
//         "JE" => Some("Jersey"),
//         "JM" => Some("Jamaica"),
//         "JO" => Some("Jordan"),
//         "JP" => Some("Japan"),
//         "KE" => Some("Kenya"),
//         "KG" => Some("Kyrgyzstan"),
//         "KH" => Some("Cambodia"),
//         "KI" => Some("Kiribati"),
//         "KM" => Some("Comoros"),
//         "KN" => Some("Saint Kitts and Nevis"),
//         "KP" => Some("North Korea"),
//         "KR" => Some("South Korea"),
//         "KW" => Some("Kuwait"),
//         "KY" => Some("Cayman Islands"),
//         "KZ" => Some("Kazakhstan"),
//         "LA" => Some("Laos"),
//         "LB" => Some("Lebanon"),
//         "LC" => Some("Saint Lucia"),
//         "LI" => Some("Liechtenstein"),
//         "LK" => Some("Sri Lanka"),
//         "LR" => Some("Liberia"),
//         "LS" => Some("Lesotho"),
//         "LT" => Some("Lithuania"),
//         "LU" => Some("Luxembourg"),
//         "LV" => Some("Latvia"),
//         "LY" => Some("Libya"),
//         "MA" => Some("Morocco"),
//         "MC" => Some("Monaco"),
//         "MD" => Some("Moldova"),
//         "ME" => Some("Montenegro"),
//         "MF" => Some("Saint Martin"),
//         "MG" => Some("Madagascar"),
//         "MH" => Some("Marshall Islands"),
//         "MK" => Some("North Macedonia"),
//         "ML" => Some("Mali"),
//         "MM" => Some("Myanmar"),
//         "MN" => Some("Mongolia"),
//         "MO" => Some("Macau"),
//         "MP" => Some("Northern Mariana Islands"),
//         "MQ" => Some("Martinique"),
//         "MR" => Some("Mauritania"),
//         "MS" => Some("Montserrat"),
//         "MT" => Some("Malta"),
//         "MU" => Some("Mauritius"),
//         "MV" => Some("Maldives"),
//         "MW" => Some("Malawi"),
//         "MX" => Some("Mexico"),
//         "MY" => Some("Malaysia"),
//         "MZ" => Some("Mozambique"),
//         "NA" => Some("Namibia"),
//         "NC" => Some("New Caledonia"),
//         "NE" => Some("Niger"),
//         "NF" => Some("Norfolk Island"),
//         "NG" => Some("Nigeria"),
//         "NI" => Some("Nicaragua"),
//         "NL" => Some("Netherlands"),
//         "NO" => Some("Norway"),
//         "NP" => Some("Nepal"),
//         "NR" => Some("Nauru"),
//         "NU" => Some("Niue"),
//         "NZ" => Some("New Zealand"),
//         "OM" => Some("Oman"),
//         "PA" => Some("Panama"),
//         "PE" => Some("Peru"),
//         "PF" => Some("French Polynesia"),
//         "PG" => Some("Papua New Guinea"),
//         "PH" => Some("Philippines"),
//         "PK" => Some("Pakistan"),
//         "PL" => Some("Poland"),
//         "PM" => Some("Saint Pierre and Miquelon"),
//         "PN" => Some("Pitcairn Islands"),
//         "PR" => Some("Puerto Rico"),
//         "PS" => Some("Palestine"),
//         "PT" => Some("Portugal"),
//         "PW" => Some("Palau"),
//         "PY" => Some("Paraguay"),
//         "QA" => Some("Qatar"),
//         "RE" => Some("Reunion"),
//         "RO" => Some("Romania"),
//         "RS" => Some("Serbia"),
//         "RU" => Some("Russia"),
//         "RW" => Some("Rwanda"),
//         "SA" => Some("Saudi Arabia"),
//         "SB" => Some("Solomon Islands"),
//         "SC" => Some("Seychelles"),
//         "SD" => Some("Sudan"),
//         "SE" => Some("Sweden"),
//         "SG" => Some("Singapore"),
//         "SH" => Some("Saint Helena"),
//         "SI" => Some("Slovenia"),
//         "SJ" => Some("Svalbard and Jan Mayen"),
//         "SK" => Some("Slovakia"),
//         "SL" => Some("Sierra Leone"),
//         "SM" => Some("San Marino"),
//         "SN" => Some("Senegal"),
//         "SO" => Some("Somalia"),
//         "SR" => Some("Suriname"),
//         "SS" => Some("South Sudan"),
//         "ST" => Some("Sao Tome and Principe"),
//         "SV" => Some("El Salvador"),
//         "SX" => Some("Sint Maarten"),
//         "SY" => Some("Syria"),
//         "SZ" => Some("Eswatini"),
//         "TC" => Some("Turks and Caicos Islands"),
//         "TD" => Some("Chad"),
//         "TF" => Some("French Southern Territories"),
//         "TG" => Some("Togo"),
//         "TH" => Some("Thailand"),
//         "TJ" => Some("Tajikistan"),
//         "TK" => Some("Tokelau"),
//         "TL" => Some("East Timor"),
//         "TM" => Some("Turkmenistan"),
//         "TN" => Some("Tunisia"),
//         "TO" => Some("Tonga"),
//         "TR" => Some("Turkey"),
//         "TT" => Some("Trinidad and Tobago"),
//         "TV" => Some("Tuvalu"),
//         "TW" => Some("Taiwan"),
//         "TZ" => Some("Tanzania"),
//         "UA" => Some("Ukraine"),
//         "UG" => Some("Uganda"),
//         "UM" => Some("United States Minor Outlying Islands"),
//         "US" => Some("United States"),
//         "UY" => Some("Uruguay"),
//         "UZ" => Some("Uzbekistan"),
//         "VA" => Some("Vatican City"),
//         "VC" => Some("Saint Vincent and the Grenadines"),
//         "VE" => Some("Venezuela"),
//         "VG" => Some("British Virgin Islands"),
//         "VI" => Some("United States Virgin Islands"),
//         "VN" => Some("Vietnam"),
//         "VU" => Some("Vanuatu"),
//         "WF" => Some("Wallis and Futuna"),
//         "WS" => Some("Samoa"),
//         "YE" => Some("Yemen"),
//         "YT" => Some("Mayotte"),
//         "XK" => Some("Kosovo"),
//         "ZA" => Some("South Africa"),
//         "ZM" => Some("Zambia"),
//         "ZW" => Some("Zimbabwe"),
//         _ => None,
//     }
// }
