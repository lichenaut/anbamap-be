use crate::prelude::*;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use reqwest::Client;
use serde_json::Value;

pub async fn region_code_to_figures(client: &Client, iso_code: &str) -> Result<Vec<String>> {
    let mut figures = Vec::new();
    let property = match get_property_from_iso(iso_code) {
        Some(property) => format!("Q{}", property),
        None => {
            tracing::error!("No Wikidata property found for ISO code: {}", iso_code);
            return Ok(figures);
        }
    };

    let url = format!(
        "https://www.wikidata.org/w/api.php?action=wbgetentities&ids={}&props=claims&format=json",
        property
    );
    let response = client.get(&url).send().await?;
    if !response.status().is_success() {
        tracing::debug!("Non-success response from Wikidata: {}", response.status());
        return Ok(figures);
    }

    let json: Value = response.json().await?;
    let claims = &json["entities"][&property]["claims"]["P6"];
    let last_claim = match claims.as_array() {
        Some(claims) => match claims.last() {
            Some(last_claim) => last_claim,
            None => return Ok(figures),
        },
        None => return Ok(figures),
    };

    let figure_id = match last_claim["mainsnak"]["datavalue"]["value"]["id"].as_str() {
        Some(figure_id) => figure_id,
        None => return Ok(figures),
    };

    let url = format!("https://www.wikidata.org/w/api.php?action=wbgetentities&ids={}&props=labels&languages=en&format=json", figure_id);
    let response = client.get(&url).send().await?;
    let json2: Value = response.json().await?;
    match json2["entities"][figure_id]["labels"]["en"]["value"].as_str() {
        Some(figure_name) => push_to_figures(figure_name, &mut figures)?,
        None => return Ok(figures),
    };

    let claims = &json["entities"][property]["claims"]["P35"];
    let last_claim = match claims.as_array() {
        Some(claims) => match claims.last() {
            Some(last_claim) => last_claim,
            None => return Ok(figures),
        },
        None => return Ok(figures),
    };

    let figure_id = match last_claim["mainsnak"]["datavalue"]["value"]["id"].as_str() {
        Some(figure_id) => figure_id,
        None => return Ok(figures),
    };

    let url = format!("https://www.wikidata.org/w/api.php?action=wbgetentities&ids={}&props=labels&languages=en&format=json", figure_id);
    let response = client.get(&url).send().await?;
    let json2: Value = response.json().await?;
    match json2["entities"][figure_id]["labels"]["en"]["value"].as_str() {
        Some(figure_name) => push_to_figures(figure_name, &mut figures)?,
        None => return Ok(figures),
    };

    Ok(figures
        .par_iter()
        .filter(|figure| {
            !figure.to_lowercase().contains("chad")
                && !figure.to_lowercase().contains("israel")
                && !figure.to_lowercase().contains("jordan")
                && !figure.to_lowercase().contains("denmark")
        })
        .cloned()
        .collect())
}

fn push_to_figures(mut figure_name: &str, figures: &mut Vec<String>) -> Result<()> {
    if figure_name == "Frederik X of Denmark" {
        figure_name = "frederik x";
    } else if figure_name == "Willem-Alexander of the Netherlands" {
        figure_name = "willem-alexander";
    } else if figure_name == "Charles III of the United Kingdom" {
        figure_name = "charles iii,king charles";
    }

    figures.push(figure_name.to_string());

    Ok(())
}

fn get_property_from_iso(iso_code: &str) -> Option<&str> {
    match iso_code {
        "ad" => Some("228"),
        "ae" => Some("878"),
        "af" => Some("889"),
        "ag" => Some("781"),
        "ai" => Some("25228"),
        "al" => Some("222"),
        "am" => Some("399"),
        "ao" => Some("916"),
        "aq" => Some("51"),
        "ar" => Some("414"),
        "as" => Some("16641"),
        "at" => Some("40"),
        "au" => Some("408"),
        "aw" => Some("21203"),
        "ax" => Some("5689"),
        "az" => Some("227"),
        "ba" => Some("225"),
        "bb" => Some("244"),
        "bd" => Some("902"),
        "be" => Some("31"),
        "bf" => Some("965"),
        "bg" => Some("219"),
        "bh" => Some("398"),
        "bi" => Some("967"),
        "bj" => Some("962"),
        "bl" => Some("25362"),
        "bm" => Some("23635"),
        "bn" => Some("921"),
        "bo" => Some("750"),
        "bq" => Some("25396"),
        "br" => Some("155"),
        "bs" => Some("778"),
        "bt" => Some("917"),
        "bv" => Some("23408"),
        "bw" => Some("963"),
        "by" => Some("184"),
        "bz" => Some("242"),
        "ca" => Some("16"),
        "cc" => Some("36004"),
        "cd" => Some("974"),
        "cf" => Some("929"),
        "cg" => Some("971"),
        "ch" => Some("39"),
        "ci" => Some("1008"),
        "ck" => Some("26988"),
        "cl" => Some("298"),
        "cm" => Some("1009"),
        "cn" => Some("148"),
        "co" => Some("739"),
        "cr" => Some("800"),
        "cu" => Some("241"),
        "cv" => Some("1011"),
        "cw" => Some("25279"),
        "cx" => Some("31063"),
        "cy" => Some("229"),
        "cz" => Some("213"),
        "de" => Some("183"),
        "dj" => Some("977"),
        "dk" => Some("35"),
        "dm" => Some("784"),
        "do" => Some("786"),
        "dz" => Some("262"),
        "ec" => Some("736"),
        "ee" => Some("191"),
        "eg" => Some("79"),
        "eh" => Some("6250"),
        "er" => Some("986"),
        "es" => Some("29"),
        "et" => Some("115"),
        "fi" => Some("33"),
        "fj" => Some("712"),
        "fk" => Some("9648"),
        "fm" => Some("3359409"),
        "fo" => Some("4628"),
        "fr" => Some("142"),
        "ga" => Some("1000"),
        "gb" => Some("145"),
        "gd" => Some("769"),
        "ge" => Some("230"),
        "gf" => Some("3769"),
        "gg" => Some("3311985"),
        "gh" => Some("117"),
        "gi" => Some("1410"),
        "gl" => Some("223"),
        "gm" => Some("1005"),
        "gn" => Some("1006"),
        "gp" => Some("17012"),
        "gq" => Some("983"),
        "gr" => Some("41"),
        "gs" => Some("35086"),
        "gt" => Some("774"),
        "gu" => Some("16635"),
        "gw" => Some("1007"),
        "gy" => Some("734"),
        "hk" => Some("8646"),
        "hm" => Some("131198"),
        "hn" => Some("783"),
        "hr" => Some("224"),
        "ht" => Some("790"),
        "hu" => Some("28"),
        "id" => Some("252"),
        "ie" => Some("22890"),
        "il" => Some("801"),
        "im" => Some("9676"),
        "in" => Some("668"),
        "io" => Some("43448"),
        "iq" => Some("796"),
        "ir" => Some("794"),
        "is" => Some("189"),
        "it" => Some("38"),
        "je" => Some("785"),
        "jm" => Some("766"),
        "jo" => Some("810"),
        "jp" => Some("17"),
        "ke" => Some("114"),
        "kg" => Some("813"),
        "kh" => Some("424"),
        "ki" => Some("710"),
        "km" => Some("970"),
        "kn" => Some("763"),
        "kp" => Some("423"),
        "kr" => Some("884"),
        "kw" => Some("817"),
        "ky" => Some("5785"),
        "kz" => Some("232"),
        "la" => Some("819"),
        "lb" => Some("822"),
        "lc" => Some("760"),
        "li" => Some("347"),
        "lk" => Some("854"),
        "lr" => Some("1014"),
        "ls" => Some("1013"),
        "lt" => Some("37"),
        "lu" => Some("32"),
        "lv" => Some("211"),
        "ly" => Some("1016"),
        "ma" => Some("1028"),
        "mc" => Some("235"),
        "md" => Some("217"),
        "me" => Some("236"),
        "mf" => Some("25596"),
        "mg" => Some("1019"),
        "mh" => Some("709"),
        "mk" => Some("221"),
        "ml" => Some("912"),
        "mm" => Some("836"),
        "mn" => Some("711"),
        "mo" => Some("14773"),
        "mp" => Some("16644"),
        "mq" => Some("17054"),
        "mr" => Some("1025"),
        "ms" => Some("732115"),
        "mt" => Some("233"),
        "mu" => Some("1027"),
        "mv" => Some("826"),
        "mw" => Some("1020"),
        "mx" => Some("96"),
        "my" => Some("833"),
        "mz" => Some("1029"),
        "na" => Some("1030"),
        "nc" => Some("33788"),
        "ne" => Some("1032"),
        "nf" => Some("31057"),
        "ng" => Some("1033"),
        "ni" => Some("811"),
        "nl" => Some("55"),
        "no" => Some("20"),
        "np" => Some("837"),
        "nr" => Some("697"),
        "nu" => Some("34020"),
        "nz" => Some("664"),
        "om" => Some("842"),
        "pa" => Some("804"),
        "pe" => Some("419"),
        "pf" => Some("30971"),
        "pg" => Some("691"),
        "ph" => Some("928"),
        "pk" => Some("843"),
        "pl" => Some("36"),
        "pm" => Some("34617"),
        "pn" => Some("35672"),
        "pr" => Some("1183"),
        "ps" => Some("219060"),
        "pt" => Some("45"),
        "pw" => Some("695"),
        "py" => Some("733"),
        "qa" => Some("846"),
        "re" => Some("17070"),
        "ro" => Some("218"),
        "rs" => Some("403"),
        "ru" => Some("159"),
        "rw" => Some("1037"),
        "sa" => Some("851"),
        "sb" => Some("685"),
        "sc" => Some("1042"),
        "sd" => Some("1049"),
        "se" => Some("34"),
        "sg" => Some("334"),
        "sh" => Some("34497"),
        "si" => Some("215"),
        "sj" => Some("842829"),
        "sk" => Some("214"),
        "sl" => Some("1044"),
        "sm" => Some("238"),
        "sn" => Some("1041"),
        "so" => Some("1045"),
        "sr" => Some("730"),
        "ss" => Some("958"),
        "st" => Some("1039"),
        "sv" => Some("792"),
        "sx" => Some("26273"),
        "sy" => Some("858"),
        "sz" => Some("1050"),
        "tc" => Some("18221"),
        "td" => Some("657"),
        "tf" => Some("129003"),
        "tg" => Some("945"),
        "th" => Some("869"),
        "tj" => Some("863"),
        "tk" => Some("36823"),
        "tl" => Some("574"),
        "tm" => Some("874"),
        "tn" => Some("948"),
        "to" => Some("678"),
        "tr" => Some("43"),
        "tt" => Some("754"),
        "tv" => Some("672"),
        "tw" => Some("865"),
        "tz" => Some("924"),
        "ua" => Some("212"),
        "ug" => Some("1036"),
        "um" => Some("16645"),
        "us" => Some("30"),
        "uy" => Some("77"),
        "uz" => Some("265"),
        "va" => Some("237"),
        "vc" => Some("757"),
        "ve" => Some("717"),
        "vg" => Some("25305"),
        "vi" => Some("11703"),
        "vn" => Some("881"),
        "vu" => Some("686"),
        "wf" => Some("35555"),
        "ws" => Some("683"),
        "xk" => Some("1246"),
        "ye" => Some("805"),
        "yt" => Some("17063"),
        "za" => Some("258"),
        "zm" => Some("953"),
        "zw" => Some("954"),
        _ => None,
    }
}

// You are seeing false values for regions because Anbamap cannot correctly match region ISO codes with Wikidata properties.
// Perhaps the property number and/or region name has changed?
async fn print_result(name: &str, code: &str, result: &bool) {
    if !*result {
        tracing::error!(
            "A Wikidata region code's correctness is false! {}",
            format!("{}, {}: {}", name, code, result)
        );
    }
}

async fn verify_iso_match(client: &Client, wikidata_id: &str, name: &str) -> bool {
    let property = get_property_from_iso(wikidata_id);
    let property = match property {
        Some(property) => property,
        None => return false,
    };

    let url = format!(
        "https://www.wikidata.org/w/api.php?action=wbgetentities&ids=Q{}&format=json",
        property
    );
    let response = client.get(&url).send().await;
    let response = match response {
        Ok(response) => response,
        Err(_) => return false,
    };

    let json = response.json().await;
    let json: serde_json::Value = match json {
        Ok(json) => json,
        Err(_) => return false,
    };

    let label = json["entities"][format!("Q{}", property)]["labels"]["en"]["value"].as_str();
    let label = match label {
        Some(label) => label,
        None => return false,
    };

    label == name
}

#[allow(unused)]
pub async fn verify_codes() {
    // Uses Geonames names by default, but this is overrided with Wikidata names when there is a difference. This occurence is tagged with '//tag'.
    let client = reqwest::Client::new();
    print_result(
        "Andorra",
        "ad",
        &verify_iso_match(&client, "ad", "Andorra").await,
    )
    .await;
    print_result(
        "United Arab Emirates",
        "ae",
        &verify_iso_match(&client, "ae", "United Arab Emirates").await,
    )
    .await;
    print_result(
        "Afghanistan",
        "af",
        &verify_iso_match(&client, "af", "Afghanistan").await,
    )
    .await;
    print_result(
        "Antigua and Barbuda",
        "ag",
        &verify_iso_match(&client, "ag", "Antigua and Barbuda").await,
    )
    .await;
    print_result(
        "Anguilla",
        "ai",
        &verify_iso_match(&client, "ai", "Anguilla").await,
    )
    .await;
    print_result(
        "Albania",
        "al",
        &verify_iso_match(&client, "al", "Albania").await,
    )
    .await;
    print_result(
        "Armenia",
        "am",
        &verify_iso_match(&client, "am", "Armenia").await,
    )
    .await;
    print_result(
        "Angola",
        "ao",
        &verify_iso_match(&client, "ao", "Angola").await,
    )
    .await;
    print_result(
        "Antarctica",
        "aq",
        &verify_iso_match(&client, "aq", "Antarctica").await,
    )
    .await;
    print_result(
        "Argentina",
        "ar",
        &verify_iso_match(&client, "ar", "Argentina").await,
    )
    .await;
    print_result(
        "American Samoa",
        "as",
        &verify_iso_match(&client, "as", "American Samoa").await,
    )
    .await;
    print_result(
        "Austria",
        "at",
        &verify_iso_match(&client, "at", "Austria").await,
    )
    .await;
    print_result(
        "Australia",
        "au",
        &verify_iso_match(&client, "au", "Australia").await,
    )
    .await;
    print_result(
        "Aruba",
        "aw",
        &verify_iso_match(&client, "aw", "Aruba").await,
    )
    .await;
    print_result(
        "Åland Islands",
        "ax",
        &verify_iso_match(&client, "ax", "Åland").await,
    )
    .await; //tag
    print_result(
        "Azerbaijan",
        "az",
        &verify_iso_match(&client, "az", "Azerbaijan").await,
    )
    .await;
    print_result(
        "Bosnia and Herzegovina",
        "ba",
        &verify_iso_match(&client, "ba", "Bosnia and Herzegovina").await,
    )
    .await;
    print_result(
        "Barbados",
        "bb",
        &verify_iso_match(&client, "bb", "Barbados").await,
    )
    .await;
    print_result(
        "Bangladesh",
        "bd",
        &verify_iso_match(&client, "bd", "Bangladesh").await,
    )
    .await;
    print_result(
        "Belgium",
        "be",
        &verify_iso_match(&client, "be", "Belgium").await,
    )
    .await;
    print_result(
        "Burkina Faso",
        "bf",
        &verify_iso_match(&client, "bf", "Burkina Faso").await,
    )
    .await;
    print_result(
        "Bulgaria",
        "bg",
        &verify_iso_match(&client, "bg", "Bulgaria").await,
    )
    .await;
    print_result(
        "Bahrain",
        "bh",
        &verify_iso_match(&client, "bh", "Bahrain").await,
    )
    .await;
    print_result(
        "Burundi",
        "bi",
        &verify_iso_match(&client, "bi", "Burundi").await,
    )
    .await;
    print_result(
        "Benin",
        "bj",
        &verify_iso_match(&client, "bj", "Benin").await,
    )
    .await;
    print_result(
        "Saint Barthélemy",
        "bl",
        &verify_iso_match(&client, "bl", "Saint Barthélemy").await,
    )
    .await;
    print_result(
        "Bermuda",
        "bm",
        &verify_iso_match(&client, "bm", "Bermuda").await,
    )
    .await;
    print_result(
        "Brunei",
        "bn",
        &verify_iso_match(&client, "bn", "Brunei").await,
    )
    .await; //tag
    print_result(
        "Bolivia",
        "bo",
        &verify_iso_match(&client, "bo", "Bolivia").await,
    )
    .await;
    print_result(
        "Bonaire",
        "bq",
        &verify_iso_match(&client, "bq", "Bonaire").await,
    )
    .await;
    print_result(
        "Brazil",
        "br",
        &verify_iso_match(&client, "br", "Brazil").await,
    )
    .await;
    print_result(
        "Bahamas",
        "bs",
        &verify_iso_match(&client, "bs", "The Bahamas").await,
    )
    .await; //tag
    print_result(
        "Bhutan",
        "bt",
        &verify_iso_match(&client, "bt", "Bhutan").await,
    )
    .await;
    print_result(
        "Bouvet Island",
        "bv",
        &verify_iso_match(&client, "bv", "Bouvet Island").await,
    )
    .await;
    print_result(
        "Botswana",
        "bw",
        &verify_iso_match(&client, "bw", "Botswana").await,
    )
    .await;
    print_result(
        "Belarus",
        "by",
        &verify_iso_match(&client, "by", "Belarus").await,
    )
    .await;
    print_result(
        "Belize",
        "bz",
        &verify_iso_match(&client, "bz", "Belize").await,
    )
    .await;
    print_result(
        "Canada",
        "ca",
        &verify_iso_match(&client, "ca", "Canada").await,
    )
    .await;
    print_result(
        "Cocos (Keeling) Islands",
        "cc",
        &verify_iso_match(&client, "cc", "Cocos (Keeling) Islands").await,
    )
    .await;
    print_result(
        "Democratic Republic of the Congo",
        "cd",
        &verify_iso_match(&client, "cd", "Democratic Republic of the Congo").await,
    )
    .await;
    print_result(
        "Central African Republic",
        "cf",
        &verify_iso_match(&client, "cf", "Central African Republic").await,
    )
    .await;
    print_result(
        "Republic of the Congo",
        "cg",
        &verify_iso_match(&client, "cg", "Republic of the Congo").await,
    )
    .await;
    print_result(
        "Switzerland",
        "ch",
        &verify_iso_match(&client, "ch", "Switzerland").await,
    )
    .await;
    print_result(
        "Ivory Coast",
        "ci",
        &verify_iso_match(&client, "ci", "Ivory Coast").await,
    )
    .await;
    print_result(
        "Cook Islands",
        "ck",
        &verify_iso_match(&client, "ck", "Cook Islands").await,
    )
    .await;
    print_result(
        "Chile",
        "cl",
        &verify_iso_match(&client, "cl", "Chile").await,
    )
    .await;
    print_result(
        "Cameroon",
        "cm",
        &verify_iso_match(&client, "cm", "Cameroon").await,
    )
    .await;
    print_result(
        "China",
        "cn",
        &verify_iso_match(&client, "cn", "People's Republic of China").await,
    )
    .await; //tag
    print_result(
        "Colombia",
        "co",
        &verify_iso_match(&client, "co", "Colombia").await,
    )
    .await;
    print_result(
        "Costa Rica",
        "cr",
        &verify_iso_match(&client, "cr", "Costa Rica").await,
    )
    .await;
    print_result("Cuba", "cu", &verify_iso_match(&client, "cu", "Cuba").await).await;
    print_result(
        "Cape Verde",
        "cv",
        &verify_iso_match(&client, "cv", "Cape Verde").await,
    )
    .await;
    print_result(
        "Curaçao",
        "cw",
        &verify_iso_match(&client, "cw", "Curaçao").await,
    )
    .await;
    print_result(
        "Christmas Island",
        "cx",
        &verify_iso_match(&client, "cx", "Christmas Island").await,
    )
    .await;
    print_result(
        "Cyprus",
        "cy",
        &verify_iso_match(&client, "cy", "Cyprus").await,
    )
    .await;
    print_result(
        "Czech Republic",
        "cz",
        &verify_iso_match(&client, "cz", "Czech Republic").await,
    )
    .await;
    print_result(
        "Germany",
        "de",
        &verify_iso_match(&client, "de", "Germany").await,
    )
    .await;
    print_result(
        "Djibouti",
        "dj",
        &verify_iso_match(&client, "dj", "Djibouti").await,
    )
    .await;
    print_result(
        "Denmark",
        "dk",
        &verify_iso_match(&client, "dk", "Denmark").await,
    )
    .await;
    print_result(
        "Dominica",
        "dm",
        &verify_iso_match(&client, "dm", "Dominica").await,
    )
    .await;
    print_result(
        "Dominican Republic",
        "do",
        &verify_iso_match(&client, "do", "Dominican Republic").await,
    )
    .await;
    print_result(
        "Algeria",
        "dz",
        &verify_iso_match(&client, "dz", "Algeria").await,
    )
    .await;
    print_result(
        "Ecuador",
        "ec",
        &verify_iso_match(&client, "ec", "Ecuador").await,
    )
    .await;
    print_result(
        "Estonia",
        "ee",
        &verify_iso_match(&client, "ee", "Estonia").await,
    )
    .await;
    print_result(
        "Egypt",
        "eg",
        &verify_iso_match(&client, "eg", "Egypt").await,
    )
    .await;
    print_result(
        "Western Sahara",
        "eh",
        &verify_iso_match(&client, "eh", "Western Sahara").await,
    )
    .await;
    print_result(
        "Eritrea",
        "er",
        &verify_iso_match(&client, "er", "Eritrea").await,
    )
    .await;
    print_result(
        "Spain",
        "es",
        &verify_iso_match(&client, "es", "Spain").await,
    )
    .await;
    print_result(
        "Ethiopia",
        "et",
        &verify_iso_match(&client, "et", "Ethiopia").await,
    )
    .await;
    print_result(
        "Finland",
        "fi",
        &verify_iso_match(&client, "fi", "Finland").await,
    )
    .await;
    print_result("Fiji", "fj", &verify_iso_match(&client, "fj", "Fiji").await).await;
    print_result(
        "Falkland Islands",
        "fk",
        &verify_iso_match(&client, "fk", "Falkland Islands").await,
    )
    .await;
    print_result(
        "Micronesia",
        "fm",
        &verify_iso_match(&client, "fm", "Micronesia").await,
    )
    .await;
    print_result(
        "Faroe Islands",
        "fo",
        &verify_iso_match(&client, "fo", "Faroe Islands").await,
    )
    .await;
    print_result(
        "France",
        "fr",
        &verify_iso_match(&client, "fr", "France").await,
    )
    .await;
    print_result(
        "Gabon",
        "ga",
        &verify_iso_match(&client, "ga", "Gabon").await,
    )
    .await;
    print_result(
        "United Kingdom",
        "gb",
        &verify_iso_match(&client, "gb", "United Kingdom").await,
    )
    .await;
    print_result(
        "Grenada",
        "gd",
        &verify_iso_match(&client, "gd", "Grenada").await,
    )
    .await;
    print_result(
        "Georgia",
        "ge",
        &verify_iso_match(&client, "ge", "Georgia").await,
    )
    .await;
    print_result(
        "French Guiana",
        "gf",
        &verify_iso_match(&client, "gf", "French Guiana").await,
    )
    .await;
    print_result(
        "Guernsey",
        "gg",
        &verify_iso_match(&client, "gg", "Guernsey").await,
    )
    .await;
    print_result(
        "Ghana",
        "gh",
        &verify_iso_match(&client, "gh", "Ghana").await,
    )
    .await;
    print_result(
        "Gibraltar",
        "gi",
        &verify_iso_match(&client, "gi", "Gibraltar").await,
    )
    .await;
    print_result(
        "Greenland",
        "gl",
        &verify_iso_match(&client, "gl", "Greenland").await,
    )
    .await;
    print_result(
        "Gambia",
        "gm",
        &verify_iso_match(&client, "gm", "The Gambia").await,
    )
    .await; //tag
    print_result(
        "Guinea",
        "gn",
        &verify_iso_match(&client, "gn", "Guinea").await,
    )
    .await;
    print_result(
        "Guadeloupe",
        "gp",
        &verify_iso_match(&client, "gp", "Guadeloupe").await,
    )
    .await;
    print_result(
        "Equatorial Guinea",
        "gq",
        &verify_iso_match(&client, "gq", "Equatorial Guinea").await,
    )
    .await;
    print_result(
        "Greece",
        "gr",
        &verify_iso_match(&client, "gr", "Greece").await,
    )
    .await;
    print_result(
        "South Georgia and the South Sandwich Islands",
        "gs",
        &verify_iso_match(
            &client,
            "gs",
            "South Georgia and the South Sandwich Islands",
        )
        .await,
    )
    .await;
    print_result(
        "Guatemala",
        "gt",
        &verify_iso_match(&client, "gt", "Guatemala").await,
    )
    .await;
    print_result("Guam", "gu", &verify_iso_match(&client, "gu", "Guam").await).await;
    print_result(
        "Guinea-Bissau",
        "gw",
        &verify_iso_match(&client, "gw", "Guinea-Bissau").await,
    )
    .await;
    print_result(
        "Guyana",
        "gy",
        &verify_iso_match(&client, "gy", "Guyana").await,
    )
    .await;
    print_result(
        "Hong Kong",
        "hk",
        &verify_iso_match(&client, "hk", "Hong Kong").await,
    )
    .await;
    print_result(
        "Heard Island and McDonald Islands",
        "hm",
        &verify_iso_match(&client, "hm", "Heard Island and McDonald Islands").await,
    )
    .await;
    print_result(
        "Honduras",
        "hn",
        &verify_iso_match(&client, "hn", "Honduras").await,
    )
    .await;
    print_result(
        "Croatia",
        "hr",
        &verify_iso_match(&client, "hr", "Croatia").await,
    )
    .await;
    print_result(
        "Haiti",
        "ht",
        &verify_iso_match(&client, "ht", "Haiti").await,
    )
    .await;
    print_result(
        "Hungary",
        "hu",
        &verify_iso_match(&client, "hu", "Hungary").await,
    )
    .await;
    print_result(
        "Indonesia",
        "id",
        &verify_iso_match(&client, "id", "Indonesia").await,
    )
    .await;
    print_result(
        "Ireland",
        "ie",
        &verify_iso_match(&client, "ie", "Ireland").await,
    )
    .await;
    print_result(
        "Israel",
        "il",
        &verify_iso_match(&client, "il", "Israel").await,
    )
    .await;
    print_result(
        "Isle of Man",
        "im",
        &verify_iso_match(&client, "im", "Isle of Man").await,
    )
    .await;
    print_result(
        "India",
        "in",
        &verify_iso_match(&client, "in", "India").await,
    )
    .await;
    print_result(
        "British Indian Ocean Territory",
        "io",
        &verify_iso_match(&client, "io", "British Indian Ocean Territory").await,
    )
    .await;
    print_result("Iraq", "iq", &verify_iso_match(&client, "iq", "Iraq").await).await;
    print_result("Iran", "ir", &verify_iso_match(&client, "ir", "Iran").await).await;
    print_result(
        "Iceland",
        "is",
        &verify_iso_match(&client, "is", "Iceland").await,
    )
    .await;
    print_result(
        "Italy",
        "it",
        &verify_iso_match(&client, "it", "Italy").await,
    )
    .await;
    print_result(
        "Jersey",
        "je",
        &verify_iso_match(&client, "je", "Jersey").await,
    )
    .await;
    print_result(
        "Jamaica",
        "jm",
        &verify_iso_match(&client, "jm", "Jamaica").await,
    )
    .await;
    print_result(
        "Jordan",
        "jo",
        &verify_iso_match(&client, "jo", "Jordan").await,
    )
    .await;
    print_result(
        "Japan",
        "jp",
        &verify_iso_match(&client, "jp", "Japan").await,
    )
    .await;
    print_result(
        "Kenya",
        "ke",
        &verify_iso_match(&client, "ke", "Kenya").await,
    )
    .await;
    print_result(
        "Kyrgyzstan",
        "kg",
        &verify_iso_match(&client, "kg", "Kyrgyzstan").await,
    )
    .await;
    print_result(
        "Cambodia",
        "kh",
        &verify_iso_match(&client, "kh", "Cambodia").await,
    )
    .await;
    print_result(
        "Kiribati",
        "ki",
        &verify_iso_match(&client, "ki", "Kiribati").await,
    )
    .await;
    print_result(
        "Comoros",
        "km",
        &verify_iso_match(&client, "km", "Comoros").await,
    )
    .await;
    print_result(
        "Saint Kitts and Nevis",
        "kn",
        &verify_iso_match(&client, "kn", "Saint Kitts and Nevis").await,
    )
    .await;
    print_result(
        "North Korea",
        "kp",
        &verify_iso_match(&client, "kp", "North Korea").await,
    )
    .await;
    print_result(
        "South Korea",
        "kr",
        &verify_iso_match(&client, "kr", "South Korea").await,
    )
    .await;
    print_result(
        "Kuwait",
        "kw",
        &verify_iso_match(&client, "kw", "Kuwait").await,
    )
    .await;
    print_result(
        "Cayman Islands",
        "ky",
        &verify_iso_match(&client, "ky", "Cayman Islands").await,
    )
    .await;
    print_result(
        "Kazakhstan",
        "kz",
        &verify_iso_match(&client, "kz", "Kazakhstan").await,
    )
    .await;
    print_result("Laos", "la", &verify_iso_match(&client, "la", "Laos").await).await;
    print_result(
        "Lebanon",
        "lb",
        &verify_iso_match(&client, "lb", "Lebanon").await,
    )
    .await;
    print_result(
        "Saint Lucia",
        "lc",
        &verify_iso_match(&client, "lc", "Saint Lucia").await,
    )
    .await;
    print_result(
        "Liechtenstein",
        "li",
        &verify_iso_match(&client, "li", "Liechtenstein").await,
    )
    .await;
    print_result(
        "Sri Lanka",
        "lk",
        &verify_iso_match(&client, "lk", "Sri Lanka").await,
    )
    .await;
    print_result(
        "Liberia",
        "lr",
        &verify_iso_match(&client, "lr", "Liberia").await,
    )
    .await;
    print_result(
        "Lesotho",
        "ls",
        &verify_iso_match(&client, "ls", "Lesotho").await,
    )
    .await;
    print_result(
        "Lithuania",
        "lt",
        &verify_iso_match(&client, "lt", "Lithuania").await,
    )
    .await;
    print_result(
        "Luxembourg",
        "lu",
        &verify_iso_match(&client, "lu", "Luxembourg").await,
    )
    .await;
    print_result(
        "Latvia",
        "lv",
        &verify_iso_match(&client, "lv", "Latvia").await,
    )
    .await;
    print_result(
        "Libya",
        "ly",
        &verify_iso_match(&client, "ly", "Libya").await,
    )
    .await;
    print_result(
        "Morocco",
        "ma",
        &verify_iso_match(&client, "ma", "Morocco").await,
    )
    .await;
    print_result(
        "Monaco",
        "mc",
        &verify_iso_match(&client, "mc", "Monaco").await,
    )
    .await;
    print_result(
        "Moldova",
        "md",
        &verify_iso_match(&client, "md", "Moldova").await,
    )
    .await;
    print_result(
        "Montenegro",
        "me",
        &verify_iso_match(&client, "me", "Montenegro").await,
    )
    .await;
    print_result(
        "Saint Martin",
        "mf",
        &verify_iso_match(&client, "mf", "Saint Martin").await,
    )
    .await;
    print_result(
        "Madagascar",
        "mg",
        &verify_iso_match(&client, "mg", "Madagascar").await,
    )
    .await;
    print_result(
        "Marshall Islands",
        "mh",
        &verify_iso_match(&client, "mh", "Marshall Islands").await,
    )
    .await;
    print_result(
        "North Macedonia",
        "mk",
        &verify_iso_match(&client, "mk", "North Macedonia").await,
    )
    .await;
    print_result("Mali", "ml", &verify_iso_match(&client, "ml", "Mali").await).await;
    print_result(
        "Myanmar",
        "mm",
        &verify_iso_match(&client, "mm", "Myanmar").await,
    )
    .await;
    print_result(
        "Mongolia",
        "mn",
        &verify_iso_match(&client, "mn", "Mongolia").await,
    )
    .await;
    print_result(
        "Macao",
        "mo",
        &verify_iso_match(&client, "mo", "Macau").await,
    )
    .await; //tag
    print_result(
        "Northern Mariana Islands",
        "mp",
        &verify_iso_match(&client, "mp", "Northern Mariana Islands").await,
    )
    .await;
    print_result(
        "Martinique",
        "mq",
        &verify_iso_match(&client, "mq", "Martinique").await,
    )
    .await;
    print_result(
        "Mauritania",
        "mr",
        &verify_iso_match(&client, "mr", "Mauritania").await,
    )
    .await;
    print_result(
        "Montserrat",
        "ms",
        &verify_iso_match(&client, "ms", "Montserrat").await,
    )
    .await;
    print_result(
        "Malta",
        "mt",
        &verify_iso_match(&client, "mt", "Malta").await,
    )
    .await;
    print_result(
        "Mauritius",
        "mu",
        &verify_iso_match(&client, "mu", "Mauritius").await,
    )
    .await;
    print_result(
        "Maldives",
        "mv",
        &verify_iso_match(&client, "mv", "Maldives").await,
    )
    .await;
    print_result(
        "Malawi",
        "mw",
        &verify_iso_match(&client, "mw", "Malawi").await,
    )
    .await;
    print_result(
        "Mexico",
        "mx",
        &verify_iso_match(&client, "mx", "Mexico").await,
    )
    .await;
    print_result(
        "Malaysia",
        "my",
        &verify_iso_match(&client, "my", "Malaysia").await,
    )
    .await;
    print_result(
        "Mozambique",
        "mz",
        &verify_iso_match(&client, "mz", "Mozambique").await,
    )
    .await;
    print_result(
        "Namibia",
        "na",
        &verify_iso_match(&client, "na", "Namibia").await,
    )
    .await;
    print_result(
        "New Caledonia",
        "nc",
        &verify_iso_match(&client, "nc", "New Caledonia").await,
    )
    .await;
    print_result(
        "Niger",
        "ne",
        &verify_iso_match(&client, "ne", "Niger").await,
    )
    .await;
    print_result(
        "Norfolk Island",
        "nf",
        &verify_iso_match(&client, "nf", "Norfolk Island").await,
    )
    .await;
    print_result(
        "Nigeria",
        "ng",
        &verify_iso_match(&client, "ng", "Nigeria").await,
    )
    .await;
    print_result(
        "Nicaragua",
        "ni",
        &verify_iso_match(&client, "ni", "Nicaragua").await,
    )
    .await;
    print_result(
        "Netherlands",
        "nl",
        &verify_iso_match(&client, "nl", "Netherlands").await,
    )
    .await;
    print_result(
        "Norway",
        "no",
        &verify_iso_match(&client, "no", "Norway").await,
    )
    .await;
    print_result(
        "Nepal",
        "np",
        &verify_iso_match(&client, "np", "Nepal").await,
    )
    .await;
    print_result(
        "Nauru",
        "nr",
        &verify_iso_match(&client, "nr", "Nauru").await,
    )
    .await;
    print_result("Niue", "nu", &verify_iso_match(&client, "nu", "Niue").await).await;
    print_result(
        "New Zealand",
        "nz",
        &verify_iso_match(&client, "nz", "New Zealand").await,
    )
    .await;
    print_result("Oman", "om", &verify_iso_match(&client, "om", "Oman").await).await;
    print_result(
        "Panama",
        "pa",
        &verify_iso_match(&client, "pa", "Panama").await,
    )
    .await;
    print_result("Peru", "pe", &verify_iso_match(&client, "pe", "Peru").await).await;
    print_result(
        "French Polynesia",
        "pf",
        &verify_iso_match(&client, "pf", "French Polynesia").await,
    )
    .await;
    print_result(
        "Papua New Guinea",
        "pg",
        &verify_iso_match(&client, "pg", "Papua New Guinea").await,
    )
    .await;
    print_result(
        "Philippines",
        "ph",
        &verify_iso_match(&client, "ph", "Philippines").await,
    )
    .await;
    print_result(
        "Pakistan",
        "pk",
        &verify_iso_match(&client, "pk", "Pakistan").await,
    )
    .await;
    print_result(
        "Poland",
        "pl",
        &verify_iso_match(&client, "pl", "Poland").await,
    )
    .await;
    print_result(
        "Saint Pierre and Miquelon",
        "pm",
        &verify_iso_match(&client, "pm", "Saint Pierre and Miquelon").await,
    )
    .await;
    print_result(
        "Pitcairn Islands",
        "pn",
        &verify_iso_match(&client, "pn", "Pitcairn Islands").await,
    )
    .await;
    print_result(
        "Puerto Rico",
        "pr",
        &verify_iso_match(&client, "pr", "Puerto Rico").await,
    )
    .await;
    print_result(
        "Palestinian Territory",
        "ps",
        &verify_iso_match(&client, "ps", "State of Palestine").await,
    )
    .await;
    print_result(
        "Portugal",
        "pt",
        &verify_iso_match(&client, "pt", "Portugal").await,
    )
    .await;
    print_result(
        "Palau",
        "pw",
        &verify_iso_match(&client, "pw", "Palau").await,
    )
    .await;
    print_result(
        "Paraguay",
        "py",
        &verify_iso_match(&client, "py", "Paraguay").await,
    )
    .await;
    print_result(
        "Qatar",
        "qa",
        &verify_iso_match(&client, "qa", "Qatar").await,
    )
    .await;
    print_result(
        "Réunion",
        "re",
        &verify_iso_match(&client, "re", "Réunion").await,
    )
    .await;
    print_result(
        "Romania",
        "ro",
        &verify_iso_match(&client, "ro", "Romania").await,
    )
    .await;
    print_result(
        "Serbia",
        "rs",
        &verify_iso_match(&client, "rs", "Serbia").await,
    )
    .await;
    print_result(
        "Russia",
        "ru",
        &verify_iso_match(&client, "ru", "Russia").await,
    )
    .await;
    print_result(
        "Rwanda",
        "rw",
        &verify_iso_match(&client, "rw", "Rwanda").await,
    )
    .await;
    print_result(
        "Saudi Arabia",
        "sa",
        &verify_iso_match(&client, "sa", "Saudi Arabia").await,
    )
    .await;
    print_result(
        "Solomon Islands",
        "sb",
        &verify_iso_match(&client, "sb", "Solomon Islands").await,
    )
    .await;
    print_result(
        "Seychelles",
        "sc",
        &verify_iso_match(&client, "sc", "Seychelles").await,
    )
    .await;
    print_result(
        "Sudan",
        "sd",
        &verify_iso_match(&client, "sd", "Sudan").await,
    )
    .await;
    print_result(
        "Sweden",
        "se",
        &verify_iso_match(&client, "se", "Sweden").await,
    )
    .await;
    print_result(
        "Singapore",
        "sg",
        &verify_iso_match(&client, "sg", "Singapore").await,
    )
    .await;
    print_result(
        "Saint Helena",
        "sh",
        &verify_iso_match(&client, "sh", "Saint Helena").await,
    )
    .await;
    print_result(
        "Slovenia",
        "si",
        &verify_iso_match(&client, "si", "Slovenia").await,
    )
    .await;
    print_result(
        "Svalbard and Jan Mayen",
        "sj",
        &verify_iso_match(&client, "sj", "Svalbard and Jan Mayen").await,
    )
    .await;
    print_result(
        "Slovakia",
        "sk",
        &verify_iso_match(&client, "sk", "Slovakia").await,
    )
    .await;
    print_result(
        "Sierra Leone",
        "sl",
        &verify_iso_match(&client, "sl", "Sierra Leone").await,
    )
    .await;
    print_result(
        "San Marino",
        "sm",
        &verify_iso_match(&client, "sm", "San Marino").await,
    )
    .await;
    print_result(
        "Senegal",
        "sn",
        &verify_iso_match(&client, "sn", "Senegal").await,
    )
    .await;
    print_result(
        "Somalia",
        "so",
        &verify_iso_match(&client, "so", "Somalia").await,
    )
    .await;
    print_result(
        "Suriname",
        "sr",
        &verify_iso_match(&client, "sr", "Suriname").await,
    )
    .await;
    print_result(
        "South Sudan",
        "ss",
        &verify_iso_match(&client, "ss", "South Sudan").await,
    )
    .await;
    print_result(
        "São Tomé and Príncipe",
        "st",
        &verify_iso_match(&client, "st", "São Tomé and Príncipe").await,
    )
    .await;
    print_result(
        "El Salvador",
        "sv",
        &verify_iso_match(&client, "sv", "El Salvador").await,
    )
    .await;
    print_result(
        "Sint Maarten",
        "sx",
        &verify_iso_match(&client, "sx", "Sint Maarten").await,
    )
    .await;
    print_result(
        "Syria",
        "sy",
        &verify_iso_match(&client, "sy", "Syria").await,
    )
    .await;
    print_result(
        "Swaziland",
        "sz",
        &verify_iso_match(&client, "sz", "Eswatini").await,
    )
    .await;
    print_result(
        "Turks and Caicos Islands",
        "tc",
        &verify_iso_match(&client, "tc", "Turks and Caicos Islands").await,
    )
    .await;
    print_result("Chad", "td", &verify_iso_match(&client, "td", "Chad").await).await;
    print_result(
        "French Southern Territories",
        "tf",
        &verify_iso_match(&client, "tf", "French Southern and Antarctic Lands").await,
    )
    .await; //tag
    print_result("Togo", "tg", &verify_iso_match(&client, "tg", "Togo").await).await;
    print_result(
        "Thailand",
        "th",
        &verify_iso_match(&client, "th", "Thailand").await,
    )
    .await;
    print_result(
        "Tajikistan",
        "tj",
        &verify_iso_match(&client, "tj", "Tajikistan").await,
    )
    .await;
    print_result(
        "Tokelau",
        "tk",
        &verify_iso_match(&client, "tk", "Tokelau").await,
    )
    .await;
    print_result(
        "Timor-Leste",
        "tl",
        &verify_iso_match(&client, "tl", "East Timor").await,
    )
    .await; //tag
    print_result(
        "Turkmenistan",
        "tm",
        &verify_iso_match(&client, "tm", "Turkmenistan").await,
    )
    .await;
    print_result(
        "Tunisia",
        "tn",
        &verify_iso_match(&client, "tn", "Tunisia").await,
    )
    .await;
    print_result(
        "Tonga",
        "to",
        &verify_iso_match(&client, "to", "Tonga").await,
    )
    .await;
    print_result(
        "Turkey",
        "tr",
        &verify_iso_match(&client, "tr", "Turkey").await,
    )
    .await;
    print_result(
        "Trinidad and Tobago",
        "tt",
        &verify_iso_match(&client, "tt", "Trinidad and Tobago").await,
    )
    .await;
    print_result(
        "Tuvalu",
        "tv",
        &verify_iso_match(&client, "tv", "Tuvalu").await,
    )
    .await;
    print_result(
        "Taiwan",
        "tw",
        &verify_iso_match(&client, "tw", "Taiwan").await,
    )
    .await;
    print_result(
        "Tanzania",
        "tz",
        &verify_iso_match(&client, "tz", "Tanzania").await,
    )
    .await;
    print_result(
        "Ukraine",
        "ua",
        &verify_iso_match(&client, "ua", "Ukraine").await,
    )
    .await;
    print_result(
        "Uganda",
        "ug",
        &verify_iso_match(&client, "ug", "Uganda").await,
    )
    .await;
    print_result(
        "United States Minor Outlying Islands",
        "um",
        &verify_iso_match(&client, "um", "United States Minor Outlying Islands").await,
    )
    .await;
    print_result(
        "United States",
        "us",
        &verify_iso_match(&client, "us", "United States of America").await,
    )
    .await; //tag
    print_result(
        "Uruguay",
        "uy",
        &verify_iso_match(&client, "uy", "Uruguay").await,
    )
    .await;
    print_result(
        "Uzbekistan",
        "uz",
        &verify_iso_match(&client, "uz", "Uzbekistan").await,
    )
    .await;
    print_result(
        "Vatican City",
        "va",
        &verify_iso_match(&client, "va", "Vatican City").await,
    )
    .await;
    print_result(
        "Saint Vincent and the Grenadines",
        "vc",
        &verify_iso_match(&client, "vc", "Saint Vincent and the Grenadines").await,
    )
    .await;
    print_result(
        "Venezuela",
        "ve",
        &verify_iso_match(&client, "ve", "Venezuela").await,
    )
    .await;
    print_result(
        "British Virgin Islands",
        "vg",
        &verify_iso_match(&client, "vg", "British Virgin Islands").await,
    )
    .await;
    print_result(
        "United States Virgin Islands",
        "vi",
        &verify_iso_match(&client, "vi", "United States Virgin Islands").await,
    )
    .await;
    print_result(
        "Vietnam",
        "vn",
        &verify_iso_match(&client, "vn", "Vietnam").await,
    )
    .await;
    print_result(
        "Vanuatu",
        "vu",
        &verify_iso_match(&client, "vu", "Vanuatu").await,
    )
    .await;
    print_result(
        "Wallis and Futuna",
        "wf",
        &verify_iso_match(&client, "wf", "Wallis and Futuna").await,
    )
    .await;
    print_result(
        "Samoa",
        "ws",
        &verify_iso_match(&client, "ws", "Samoa").await,
    )
    .await;
    print_result(
        "Kosovo",
        "xk",
        &verify_iso_match(&client, "xk", "Kosovo").await,
    )
    .await;
    print_result(
        "Yemen",
        "ye",
        &verify_iso_match(&client, "ye", "Yemen").await,
    )
    .await;
    print_result(
        "Mayotte",
        "yt",
        &verify_iso_match(&client, "yt", "Mayotte").await,
    )
    .await;
    print_result(
        "South Africa",
        "za",
        &verify_iso_match(&client, "za", "South Africa").await,
    )
    .await;
    print_result(
        "Zambia",
        "zm",
        &verify_iso_match(&client, "zm", "Zambia").await,
    )
    .await;
    print_result(
        "Zimbabwe",
        "zw",
        &verify_iso_match(&client, "zw", "Zimbabwe").await,
    )
    .await;
}
