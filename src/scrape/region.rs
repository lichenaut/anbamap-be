use std::vec;
use rayon::prelude::*;
use lazy_static::lazy_static;
use std::sync::{Arc, Mutex};
use serde_json::json;
use reqwest::{blocking::Response, blocking::Client};
use std::{error::Error, fs, str};

#[derive(Debug, Eq, PartialEq, Hash)]
pub enum RegionType {
    ExtraRegional,
    Abkhazia,
    Afghanistan,
    Albania,
    Algeria,
    Andorra,
    Angola,
    AntiguaAndBarbuda,
    Argentina,
    Armenia,
    Australia,
    Austria,
    Azerbaijan,
    TheBahamas,
    Bahrain,
    Bangladesh,
    Barbados,
    Belarus,
    Belgium,
    Belize,
    Benin,
    Bhutan,
    Bolivia,
    BosniaAndHerzegovina,
    Botswana,
    Brazil,
    Brunei,
    Bulgaria,
    BurkinaFaso,
    Burundi,
    Cambodia,
    Cameroon,
    Canada,
    CapeVerde,
    CentralAfricanRepublic,
    Chad,
    Chile,
    China,
    Colombia,
    Comoros,
    Congo,
    DemocraticRepublicOfTheCongo,
    CostaRica,
    IvoryCoast,
    Croatia,
    Cuba,
    Cyprus,
    CzechRepublic,
    Denmark,
    Djibouti,
    Dominica,
    DominicanRepublic,
    EastTimor,
    Ecuador,
    Egypt,
    ElSalvador,
    EquatorialGuinea,
    Eritrea,
    Estonia,
    Eswatini,
    Ethiopia,
    Fiji,
    Finland,
    France,
    Gabon,
    TheGambia,
    Georgia,
    Germany,
    Ghana,
    Greece,
    Grenada,
    Guatemala,
    Guinea,
    GuineaBissau,
    Guyana,
    Haiti,
    Honduras,
    Hungary,
    Iceland,
    India,
    Indonesia,
    Iran,
    Iraq,
    Ireland,
    Israel,
    Italy,
    Jamaica,
    Japan,
    Jordan,
    Kazakhstan,
    Kenya,
    Kiribati,
    NorthKorea,
    SouthKorea,
    Kosovo,
    Kuwait,
    Kyrgyzstan,
    Laos,
    Latvia,
    Lebanon,
    Lesotho,
    Liberia,
    Libya,
    Liechtenstein,
    Lithuania,
    Luxembourg,
    Madagascar,
    Malawi,
    Malaysia,
    Maldives,
    Mali,
    Malta,
    MarshallIslands,
    Mauritania,
    Mauritius,
    Mexico,
    Micronesia,
    Moldova,
    Monaco,
    Mongolia,
    Montenegro,
    Morocco,
    Mozambique,
    Myanmar,
    Namibia,
    Nauru,
    Nepal,
    Netherlands,
    NewZealand,
    Nicaragua,
    Niger,
    Nigeria,
    NorthMacedonia,
    Norway,
    Oman,
    Pakistan,
    Palau,
    Palestine,
    Panama,
    PapuaNewGuinea,
    Paraguay,
    Peru,
    Philippines,
    Poland,
    Portugal,
    PuertoRico,
    Qatar,
    Romania,
    Russia,
    Rwanda,
    SaintLucia,
    SaintKitssAndNevis,
    SaintVincentAndTheGrenadines,
    SaoTomeAndPrincipe,
    Samoa,
    SanMarino,
    SaudiArabia,
    Senegal,
    Serbia,
    Seychelles,
    SierraLeone,
    Singapore,
    Slovakia,
    Slovenia,
    SolomonIslands,
    Somalia,
    SouthAfrica,
    SouthSudan,
    Spain,
    SriLanka,
    Sudan,
    Suriname,
    Sweden,
    Switzerland,
    Syria,
    Tajikistan,
    Tanzania,
    Togo,
    Tonga,
    Thailand,
    Transnistria,
    TrinidadAndTobago,
    Tunisia,
    Turkey,
    Turkmenistan,
    Tuvalu,
    Uganda,
    Ukraine,
    UnitedArabEmirates,
    UnitedKingdom,
    UnitedStates,
    Uruguay,
    Uzbekistan,
    Vanuatu,
    VaticanCity,
    Venezuela,
    Vietnam,
    WesternSahara,
    Yemen,
    Zambia,
    Zimbabwe,
}

struct RegionKeyphrases {
    pub names: Option<Vec<String>>,
    pub demonyms: Option<Vec<String>>,
    pub figures: Option<Vec<String>>,
    pub geo: Option<Vec<String>>,
    pub enterprises: Option<Vec<String>>, // https://companiesmarketcap.com/all-countries/
    pub misc: Option<Vec<String>>,
}

impl RegionKeyphrases {
    pub fn get_region_vec(self) -> Vec<String> {
        let mut region_vec = Vec::new();
        if let Some(names) = self.names { region_vec.extend(names); }
        if let Some(demonyms) = self.demonyms { region_vec.extend(demonyms); }
        if let Some(figures) = self.figures { region_vec.extend(figures); } // At least the names on a region's Wikipedia infobox.
        if let Some(geo) = self.geo { region_vec.extend(geo); } // subregions ≥ 490k population, capitals, cities ≥ 290k population
        if let Some(enterprises) = self.enterprises { region_vec.extend(enterprises); } // ≥ 9.9B market cap USD
        // Positions of power, legislative bodies, institutions, buildings, political groups, ideologies, ethnic groups, cultural regions, etc.
        if let Some(misc) = self.misc { region_vec.extend(misc); }
        region_vec
    }
}

// TODO: subregions, capital, cities keyphrases gen. keyphrase blacklist

pub fn get_geo_map(client: &Client, api_username: &String, country_info_json: &serde_json::Value) -> Result<Vec<(String, Vec<String>)>, Box<dyn Error>> {
    let country_tuples = country_info_json["geonames"].as_array().unwrap().iter().map(|country| {
        (
            country["countryName"].as_str().unwrap().to_string(),
            vec![country["geonameId"].as_str().unwrap().to_string()],
        )
    }).collect::<Vec<(String, Vec<String>)>>();


    let mut map = Arc::new(Mutex::new(Vec::new()));
    let chunks: Vec<_> = country_tuples.chunks(16).collect();
    chunks.into_par_iter().for_each(|chunk| {
        for country in chunk.iter() {
            let (country_name, geoname_id) = country;
            let url = format!("https://secure.geonames.org/childrenJSON?geonameId={:?}&username={}", geoname_id, api_username);
            let response = client.get(&url).send().unwrap();
            let country_json: serde_json::Value = response.json().unwrap();
            let (lat, lng) = (country_json["lat"].as_str().unwrap(), country_json["lng"].as_str().unwrap());


            // if feature class is A and feature code starts with A and doesn't contain H and population is 490k or above
            
        }
    });

    Ok(country_tuples)
}



lazy_static! {
    pub static ref REGION_MAP: Vec<(Vec<String>, String)> = {
        let mut map = Vec::new();
        let client = Client::new();
        let api_username = fs::read_to_string("keys/geonames.txt").unwrap();
        let country_info_url = format!("https://secure.geonames.org/countryInfoJSON?&username={}", api_username);
        let country_info_response = client.get(&country_info_url).send().unwrap();
        let country_info_json: serde_json::Value = country_info_response.json().unwrap();
        let geo_map = get_geo_map(&client, &country_info_json);
        

























        map.push((RegionKeyphrases {
            names: None,
            demonyms: Some(vec!["afghan".into()]),
            figures: Some(vec!["hibatullah akhundzada".into(), "haibatullah akhunzada".into(), "hasan akhund".into(), "abdul hakim haqqani".into(), "abdul hakim ishaqzai".into()]),
            enterprises: None,
            geo: None,
            misc: Some(vec!["taliban".into()]),
        }.get_region_vec(), "Afghanistan".into()));
        map.insert(RegionKeyphrases {
            names: Some(vec!["albania".into()]),
            demonyms: None,
            figures: Some(vec!["bajram begaj".into(), "edi rama".into(), "lindita nikolla".into()]),
            enterprises: None,
            subregions: None,
            capitals: Some(vec!["tirana".into()]),
            cities: None,
            misc: Some(vec!["kuvendi".into()]),
        }.get_region_vec(), RegionType::Albania);
        map.insert(RegionKeyphrases {
            names: Some(vec!["algeria".into()]),
            demonyms: None,
            figures: Some(vec!["abdelmadjid tebboune".into(), "nadir larbaoui".into(), "salah goudjil".into(), "ibrahim boughali".into()]),
            enterprises: None,
            subregions: Some(vec!["oran".into(), "setif".into(), "tizi ouzou".into(), "batna".into(), "djelfa".into(), "blida".into(), "chlef".into(), "m'sila".into(), "tlemcen".into(), "bejaia".into(), "skikda".into(), "tiaret".into(), "medea".into(), "boumerdes".into(), "mila".into(), "ain defla".into(), "mostaganem".into(), "relizane".into(), "bouira".into(), "tebessa".into(), "el oued".into(), "jijel".into(), "bordj bou arreridj".into(), "oum el bouaghi".into(), "annaba".into(), "sidi bel abbes".into(), "tipaza".into(), "biskra".into()]),
            capitals: Some(vec!["algiers".into()]),
            cities: Some(vec!["bel abbes".into()]),
            misc: Some(vec!["algerie".into(), "fln".into()]),
        }.get_region_vec(), RegionType::Algeria);
        map.insert(RegionKeyphrases {
            names: Some(vec!["andorra".into()]),
            demonyms: None,
            figures: Some(vec!["joan enric vives i sicilia".into(), "macron".into(), "josep maria mauri".into(), "patrick strzoda".into(), "xavier espot zamora".into(), "carles ensenyat reig".into()]),
            enterprises: None,
            subregions: None,
            capitals: None,
            cities: None,
            misc: Some(vec!["general syndic".into(), "council of the valleys".into()]),
        }.get_region_vec(), RegionType::Andorra);
        map.insert(RegionKeyphrases {
            names: Some(vec!["angola".into()]),
            demonyms: None,
            figures: Some(vec!["joao lourenco".into(), "esperanca da costa".into()]),
            enterprises: None,
            subregions: Some(vec!["huila".into(), "benguela".into(), "huambo".into(), "cuanza sul".into(), "uige".into(), "bie".into(), "cunene".into(), "malanje".into(), "lunda norte".into(), "moxico".into(), "cabinda".into(), "zaire".into(), "lunda sul".into(), "cuando cubango".into(), "namibe".into()]),
            capitals: Some(vec!["luanda".into()]),
            cities: Some(vec!["cabinda".into(), ]),
            misc: Some(vec!["mpla".into(), "unita".into()]),
        }.get_region_vec(), RegionType::Angola);
        map.insert(RegionKeyphrases {
            names: Some(vec!["antigua".into(), "barbuda".into(), "a&b".into()]),
            demonyms: None,
            figures: Some(vec!["king charles".into(), "charles iii".into(), "rodney williams".into(), "gaston browne".into()]),
            enterprises: None,
            subregions: None,
            capitals: None,
            cities: None,
            misc: Some(vec!["ablp".into(), "united progressive party".into()]),
        }.get_region_vec(), RegionType::AntiguaAndBarbuda);
        map.insert(RegionKeyphrases {
            names: None,
            demonyms: Some(vec!["argentin".into()]),
            figures: Some(vec!["milei".into(), "victoria villarruel".into(), "nicolas posse".into(), "martin menem".into(), "horacio rosatti".into()]),
            enterprises: Some(vec!["mercadolibre".into(), "mercado libre".into(), "ypf".into(), "yacimientos petroliferos fiscales".into()]),
            subregions: Some(vec!["cordoba".into(), "mendoza".into(), "tucuman".into(), "salta".into(), "entre rios".into(), "misiones".into(), "corrientes".into(), "chaco".into(), "santiago del estero".into(), "jujuy".into(), "rio negro".into(), "neuquen".into(), "formosa".into(), "chubut".into()]),
            capitals: Some(vec!["buenos aires".into()]),
            cities: Some(vec!["rosario".into(), "la plata".into(), "mar del plata".into(), "quilmes".into(), "salta".into(), "santa fe de la vera cruz".into(), "resistencia".into(), "posadas".into(), "bahia blanca".into()]),
            misc: Some(vec!["casa rosada".into(), "union for the homeland".into(), "juntos por el cambio".into(), "cambiemos".into(), "peronis".into(), "kirchneris".into()]),
        }.get_region_vec(), RegionType::Argentina);
        map.insert(RegionKeyphrases {
            names: Some(vec!["armenia".into()]),
            demonyms: None,
            figures: Some(vec!["vahagn khachaturyan".into(), "nikol pashinyan".into(), "alen simonyan".into()]),
            enterprises: None,
            subregions: None,
            capitals: Some(vec!["yerevan".into()]),
            cities: None,
            misc: Some(vec!["azgayin zhoghov".into()]),
        }.get_region_vec(), RegionType::Armenia);
        map.insert(RegionKeyphrases {
            names: Some(vec!["australia".into()]),
            demonyms: Some(vec!["aussie".into()]),
            figures: Some(vec!["king charles".into(), "charles iii".into(), "david hurley".into(), "anthony albanese".into()]),
            enterprises: Some(vec!["bhp group".into(), "commonwealth bank".into(), "csl".into(), "nab limited".into(), "anz bank".into(), "fortescue".into(), "wesfarmers".into(), "macquarie".into(), "atlassian".into(), "goodman group".into(), "woodside".into(), "telstra".into(), "transurban".into(), "woolworths".into(), "wisetech".into(), "qbe insurance".into(), "santos limited".into(), "aristocrat leisure".into(), "rea group".into(), "coles group".into(), "cochlear".into(), "suncorp".into(), "brambles limited".into(), "reece group".into(), "origin energy".into(), "northern star resources".into(), "scentre group".into(), "south32".into(), "computershare".into()]),
            subregions: Some(vec!["new south wales".into(), "queensland".into(), "tasmania".into(), "jervis bay".into(), "norfolk island".into(), "christmas island".into(), "cocos islands".into(), "keeling islands".into(), "coral sea island".into(), "ashmore and cartier".into(), "heard island".into(), "mcdonald island".into()]),
            capitals: Some(vec!["canberra".into()]),
            cities: Some(vec!["sydney".into(), "melbourne".into(), "brisbane".into(), "perth".into(), "adelaide".into(), "gold coast".into(), "wollongong".into(), "logan city".into()]),
            misc: Some(vec!["aborigin".into()]),
        }.get_region_vec(), RegionType::Australia);
        map.insert(RegionKeyphrases {
            names: Some(vec!["austria".into(), "oesterreich".into()]),
            demonyms: None,
            figures: Some(vec!["van der bellen".into(), "karl nehammer".into()]),
            enterprises: Some(vec!["verbund".into(), "erste group".into(), "omv".into()]),
            subregions: Some(vec!["styria".into(), "tyrol".into(), "carinthia".into(), "salzburg".into()]),
            capitals: Some(vec!["vienna".into()]),
            cities: Some(vec!["graz".into()]),
            misc: None,
        }.get_region_vec(), RegionType::Austria);
        map.insert(RegionKeyphrases {
            names: Some(vec!["azerbaijan".into()]),
            demonyms: Some(vec!["azeri".into()]),
            figures: Some(vec!["aliyev".into(), "ali asadov".into(), "sahiba gafarova".into()]),
            enterprises: None,
            subregions: Some(vec!["sumgait".into()]),
            capitals: Some(vec!["baku".into()]),
            cities: Some(vec!["sumgayit".into(), "sumqayit".into(), "ganja".into()]),
            misc: Some(vec!["milli majlis".into(), "democratic reforms party".into()]),
        }.get_region_vec(), RegionType::Azerbaijan);
        map.insert(RegionKeyphrases {
            names: Some(vec!["bahama".into()]),
            demonyms: Some(vec!["bahamian".into()]),
            figures: Some(vec!["king charles".into(), "charles iii".into(), "cynthia pratt".into(), "philip davis".into()]),
            enterprises: None,
            subregions: None,
            capitals: Some(vec!["nassau".into()]),
            cities: None,
            misc: Some(vec!["progressive liberal party".into(), "free national movement".into()]),
        }.get_region_vec(), RegionType::TheBahamas);
        map.insert(RegionKeyphrases {
            names: Some(vec!["bahrain".into()]),
            demonyms: None,
            figures: Some(vec!["al khalifa".into()]),
            enterprises: Some(vec!["ahli united".into()]),
            subregions: Some(vec!["capital governorate".into()]),
            capitals: Some(vec!["manama".into()]),
            cities: None,
            misc: Some(vec!["shura council".into(), "asalah".into(), "progressive democratic tribune".into(), "bchr".into()]),
        }.get_region_vec(), RegionType::Bahrain);
        map.insert(RegionKeyphrases {
            names: Some(vec!["bangladesh".into()]),
            demonyms: None,
            figures: Some(vec!["mohammed shahabuddin".into(), "sheikh hasina".into(), "shirin sharmin chaudhury".into(), "obaidul hassan".into()]),
            enterprises: None,
            subregions: Some(vec!["barisal".into(), "chittagong".into(), "khulna".into(), "mymensingh".into(), "rajshahi".into(), "rangpur".into(), "sylhet".into()]),
            capitals: Some(vec!["dhaka".into()]),
            cities: Some(vec!["chattogram".into(), "comilla".into(), "cumilla".into(), "shibganj".into(), "natore".into(), "tongi".into(), ]),
            misc: Some(vec!["jatiya sangsad".into(), "awami league".into(), "jatiya party".into(), "bengal".into()]),
        }.get_region_vec(), RegionType::Bangladesh);
        map.insert(RegionKeyphrases {
            names: Some(vec!["barbados".into()]),
            demonyms: None,
            capitals: Some(vec!["bridgetown".into()]),
            relevant_figures: Some(vec!["sandra mason".into(), "mia mottley".into()]),
            relevant_cities: None,
            subregions: None,
            misc: Some(vec!["cibc".into(), "sagicor financial".into()]),
        }.get_region_vec(), RegionType::Barbados);
        map.insert(RegionKeyphrases {
            names: Some(vec!["belarus".into()]),
            demonyms: None,
            capitals: None,
            relevant_figures: Some(vec!["lukashenko".into(), "roman golovchenko".into()]),
            relevant_cities: Some(vec!["brest".into(), "gomel".into(), "grodno".into(), "mogilev".into(), "vitebsk".into()]),
            subregions: None,
            misc: Some(vec!["belaya rus".into(), "ldpb".into(), "belneftekhim".into(), "mozyr oil".into(), "naftan oil".into(), "beltelecom".into(), "belmedpreparaty".into()]),
        }.get_region_vec(), RegionType::Belarus);
        map.insert(RegionKeyphrases {
            names: Some(vec!["belgium".into()]),
            demonyms: Some(vec!["belgian".into()]),
            capitals: Some(vec!["brussels".into()]),
            relevant_figures: Some(vec!["king philippe".into(), "alexander de croo".into()]),
            relevant_cities: Some(vec!["antwerp".into(), "ghent".into(), "charleroi".into()]),
            subregions: Some(vec!["flanders".into(), "wallonia".into()]),
            misc: Some(vec!["flemish".into(), "walloon".into(), "anheuser-busch".into(), "ackermans & van haaren".into(), "reynaers aluminium".into(), "luciad".into(), "groupe bruxelles lambert".into(), "compagnie nationale a portefeuille".into(), "gimv".into(), "sofina".into(), "fluxys".into(), "proximus".into(), "telenet group".into(), "carmeuse".into(), "forrest group".into(), "nyrstar".into(), "umicore".into(), "deme".into(), "metallo-chimique".into(), "janssen pharmaceuticals".into(), "ucb".into(), "ag real estate".into(), "cofinimmo".into(), "bekaert".into(), "solvay".into(), "soudal".into(), "deceuninck".into(), "kbc group".into(), "bdo global".into()]),
        }.get_region_vec(), RegionType::Belgium);
        map.insert(RegionKeyphrases {// continue updating misc fields here
            names: Some(vec!["belize".into()]),
            demonyms: None,
            capitals: Some(vec!["belmopan".into()]),
            relevant_figures: Some(vec!["king charles".into(), "charles iii".into(), "dame froyla tzalam".into(), "john briceno".into(), "johnny briceno".into()]),
            relevant_cities: None,
            subregions: Some(vec!["cayo district".into(), "corozal district".into(), "orange walk district".into(), "stann creek district".into(), "toledo district".into()]),
            misc: Some(vec!["people's united party".into()]),
        }.get_region_vec(), RegionType::Belize);
        map.insert(RegionKeyphrases {
            names: Some(vec!["benin".into()]),
            demonyms: None,
            capitals: Some(vec!["porto-novo".into()]),
            relevant_figures: Some(vec!["patrice talon".into(), "mariam chabi talata".into()]),
            relevant_cities: Some(vec!["cotonou".into(), "parakou".into()]),
            subregions: Some(vec!["alibori".into(), "atakora".into(), "atlantique".into(), "borgou".into(), "collines".into(), "donga".into(), "kouffo".into(), "oueme".into(), "zou".into()]),
            misc: Some(vec!["progressive union for renewal".into()]),
        }.get_region_vec(), RegionType::Benin);
        map.insert(RegionKeyphrases {
            names: Some(vec!["bhutan".into()]),
            demonyms: None,
            capitals: Some(vec!["thimphu".into()]),
            relevant_figures: Some(vec!["wangchuck".into(), "tshering tobgay".into()]),
            relevant_cities: None,
            subregions: Some(vec!["bhumthang".into(), "chukha".into(), "dagana".into(), "gasa".into(), "haa".into(), "lhuntse".into(), "mongar".into(), "paro".into(), "pemagatshel".into(), "punakha".into(), "samdrup jongkhar".into(), "samtse".into(), "sarpang".into(), "thimphu".into(), "trashigang".into(), "trashiyangtse".into(), "trongsa".into(), "tsirang".into(), "wangdue phodrang".into(), "zhemgang".into()]),
            misc: Some(vec!["druk gyalpo".into()]),
        }.get_region_vec(), RegionType::Bhutan);
        map.insert(RegionKeyphrases {
            names: Some(vec!["bolivia".into()]),
            demonyms: None,
            capitals: Some(vec!["sucre".into()]),
            relevant_figures: Some(vec!["luis arce".into(), "lucho".into(), "david choquehuanca".into(), "andronico rodriguez".into(), "israel huaytari".into()]),
            relevant_cities: Some(vec!["la paz".into(), "santa cruz de la sierra".into(), "el alto".into(), "cochabamba".into(), "oruro".into()]),
            subregions: Some(vec!["pando".into(), "beni".into(), "potosi".into(), "chuquisaca".into(), "tarija".into()]),
            misc: Some(vec!["pluritonal".into(), "plaza murillo".into()]),
        }.get_region_vec(), RegionType::Bolivia);
        map.insert(RegionKeyphrases {
            names: Some(vec!["bosnia".into(), "srpska".into(), "brcko".into()]),
            demonyms: Some(vec!["herzegovin".into()]),
            capitals: Some(vec!["sarajevo".into()]),
            relevant_figures: Some(vec!["christian schmidt".into(), "denis becirovic".into(), "zeljka cvijanovic".into(), "zeljko komsic".into(), "borjana kristo".into()]),
            relevant_cities: None,
            subregions: None,
            misc: Some(vec!["alliance of independent social democrats".into(), "party of democratic action".into()]),
        }.get_region_vec(), RegionType::BosniaAndHerzegovina);
        map.insert(RegionKeyphrases {
            names: Some(vec!["botswana".into()]),
            demonyms: Some(vec!["batswana".into(), "motswana".into()]),
            capitals: Some(vec!["gaborone".into()]),
            relevant_figures: Some(vec!["mokgweetsi masisi".into(), "slumber tsogwane".into(), "phandu skelemani".into()]),
            relevant_cities: None,
            subregions: Some(vec!["kweneng".into(), "kgatleng".into(), "ngamiland".into(), "kgalagadi".into(), "chobe".into(), "ghanzi".into()]),
            misc: Some(vec!["umbrella for democratic change".into()]),
        }.get_region_vec(), RegionType::Botswana);
        map.insert(RegionKeyphrases {
            names: Some(vec!["brazil".into(), "brasil".into()]),
            demonyms: None,
            capitals: None,
            relevant_figures: Some(vec!["lula".into(), "geraldo alckmin".into(), "arthur lira".into(), "rodrigo pacheco".into(), "luis roberto barroso".into()]),
            relevant_cities: Some(vec!["sao paulo".into(), "rio de janeiro".into(), "fortaleza".into(), "belo horizonte".into(), "manaus".into(), "curitiba".into(), "recife".into(), "goiania".into(), "porto alegre".into(), "belem".into(), "guarulhos".into(), "campinas".into(), "sao luis".into(), "maceio".into(), "campo grande".into(), "sao goncalo".into(), "teresina".into(), "joao pessoa".into(), "sao bernardo do campo".into(), "duque de caxias".into(), "nova iguacu".into(), "natal".into(), "santo andre".into(), "osasco".into(), "sorocaba".into(), "uberlandia".into(), "ribeirao preto".into(), "sao jose dos campos".into(), "cuiaba".into(), "jaboatao dos guararapes".into(), "contagem".into(), "joinville".into(), "feira de santana".into(), "aracaju".into(), "londrina".into(), "juiz de fora".into(), "florianopolis".into(), "aparecida de goiania".into(), "serra".into(), "campos dos goytacazes".into(), "belford roxo".into(), "niteroi".into(), "sao jose do rio preto".into(), "ananindeua".into(), "vila velha".into(), "caxias do sul".into(), "porto velho".into(), "mogi das cruzes".into(), "jundiai".into(), "macapa".into(), "sao joao de meriti".into(), "piracicaba".into(), "campina grande".into(), "santos".into(), "maua".into(), "montes claros".into(), "boa vista".into(), "betim".into(), "maringa".into(), "anapolis".into(), "diadema".into(), "carapicuiba".into(), "petrolina".into(), "bauru".into(), "caruaru".into(), "vitoria da conquista".into(), "itaquaquecetuba".into(), "rio branco".into(), "blumenau".into(), "ponta grossa".into(), "caucaia".into(), "cariacica".into(), "olinda".into(), "praia grande".into(), "cascavel".into(), "canoas".into(), "paulista".into(), "uberaba".into(), "santarem".into(), "sao vicente".into(), "ribeirao das neves".into(), "sao jose dos pinhais".into(), "pelotas".into(), "vitoria".into(), "barueri".into(), "taubate".into(), "suzano".into(), "palmas".into(), "camacari".into(), "varzea grande".into(), "limeira".into(), "guaruja".into(), "juazeiro do norte".into(), "foz do iguacu".into(), "sumare".into(), "petropolis".into(), "cotia".into(), "taboao da serra".into(), "imperatriz".into(), "santa maria".into(), "sao jose".into(), "maraba".into(), "parauapebas".into(), "gravatai".into(), "mossoro".into(), "itajai".into(), "volta redonda".into(), "governador valadares".into(), "indaiatuba".into(), "sao carlos".into(), "chapeco".into(), "parnamirim".into(), "embu das artes".into(), "macae".into(), "rondonopolis".into(), "sao jose de ribamar".into(), "dourados".into(), "aracatuba".into(), "jacarei".into(), "marilia".into(), "americana".into(), "hortolandia".into(), "juazeiro".into(), "arapiraca".into(), "maracanau".into(), "itapevi".into(), "colombo".into(), "divinopolis".into(), "mage".into(), "novo hamburgo".into(), "ipatinga".into(), "sete lagoas".into(), "rio verde".into(), "aguas lindas de goias".into(), "presidente prudente".into(), "itaborai".into(), "viao".into(), "palhoca".into(), "caucaia".into(), "sobral".into(), "rio claro".into(), "aracatuba".into(), "valparaiso de goias".into(), "marica".into(), "sinop".into()]),
            subregions: Some(vec!["acre".into(), "alagoas".into(), "amapa".into(), "amazonas".into(), "bahia".into(), "ceara".into(), "distrito federal".into(), "espirito santo".into(), "goias".into(), "maranhao".into(), "mato grosso".into(), "mato grosso do sul".into(), "minas gerais".into(), "para".into(), "paraiba".into(), "parana".into(), "pernambuco".into(), "piaui".into(), "rio de janeiro".into(), "rio grande do norte".into(), "rio grande do sul".into(), "rondonia".into(), "roraima".into(), "santa catarina".into(), "sao paulo".into(), "sergipe".into(), "tocantins".into()]),
            misc: Some(vec!["planalto".into()]),
        }.get_region_vec(), RegionType::Brazil);
        map.insert(RegionKeyphrases {
            names: Some(vec!["brunei".into()]),
            demonyms: None,
            capitals: Some(vec!["bandar seri begawan".into()]),
            relevant_figures: Some(vec!["hassanal bolkiah".into(), "muhtadee billah".into(), "abdul aziz".into()]),
            relevant_cities: None,
            subregions: Some(vec!["belait".into(), "seria".into(), "tutong".into(),]),
            misc: None,
        }.get_region_vec(), RegionType::Brunei);
        map.insert(RegionKeyphrases {
            names: Some(vec!["bulgaria".into()]),
            demonyms: None,
            capitals: None,
            relevant_figures: Some(vec!["rumen radev".into(), "iliana iotova".into(), "dimitar glavchev".into()]), // Vacant National Assembly Chairperson
            relevant_cities: Some(vec!["plovdiv".into(), "varna".into()]),
            subregions: Some(vec!["blagoevgrad".into(), "burgas".into(), "dobrich".into(), "gabrovo".into(), "haskovo".into(), "kardzhali".into(), "kyustendil".into(), "lovech".into(), "pazardzhik".into(), "pernik".into(), "pleven".into(), "plovdiv".into(), "razgrad".into(), "rousse".into(), "shumen".into(), "silistra".into(), "sliven".into(), "smolyan".into(), "stara zagora".into(), "targovishte".into(), "varna".into(), "veliko tarnovo".into(), "vidin".into(), "vratsa".into(), "yambol".into()]),
            misc: Some(vec!["narodno sabranie".into(), "gerb".into()]),
        }.get_region_vec(), RegionType::Bulgaria);
        map.insert(RegionKeyphrases {
            names: Some(vec!["burkina faso".into()]),
            demonyms: Some(vec!["burkinabe".into(), "burkinese".into()]),
            capitals: Some(vec!["ouagadougou".into()]),
            relevant_figures: Some(vec!["ibrahim traore".into(), "apollinaire joachim".into(), "kyelem de tambela".into()]),
            relevant_cities: Some(vec!["bobo-dioulasso".into()]),
            subregions: Some(vec!["boucle du mouhoin".into(), "centre-est".into(), "centre-nord".into(), "centre-ouest".into(), "centre-sud".into(), "hauts-bassins".into(), "plateau-central".into(), "sahel".into(), "sud-ouest".into()]),
            misc: Some(vec!["mpsr".into()]),
        }.get_region_vec(), RegionType::BurkinaFaso);
        map.insert(RegionKeyphrases {
            names: Some(vec!["burundi".into()]),
            demonyms: None,
            capitals: Some(vec!["gitega".into(), "bujumbura".into()]),
            relevant_figures: Some(vec!["evariste ndayishimiye".into(), "prosper bazombanza".into(), "gervais ndirakobuca".into()]),
            relevant_cities: None,
            subregions: Some(vec!["bubanza".into(), "bururi".into(), "cankuzo".into(), "cibitoke".into(), "karuzi".into(), "kayanza".into(), "kirundo".into(), "makamba".into(), "muramvya".into(), "muyinga".into(), "mwaro".into(), "ngozi".into(), "rumonge".into(), "rutana".into(), "ruyigi".into()]),
            misc: Some(vec!["cndd".into(), "national congress for liberty".into(), "national congress for freedom".into()]),
        }.get_region_vec(), RegionType::Burundi);
        map.insert(RegionKeyphrases {
            names: Some(vec!["cambodia".into()]),
            demonyms: Some(vec!["khmer".into()]),
            capitals: Some(vec!["phnom penh".into()]),
            relevant_figures: Some(vec!["norodom sihamoni".into(), "hun manet".into(), "hun sen".into(), "khuon sodary".into()]),
            relevant_cities: Some(vec!["siem reap".into()]),
            subregions: Some(vec!["banteay meanchey".into(), "battambang".into(), "kampong cham".into(), "kampong chhnang".into(), "kampong speu".into(), "kampong thom".into(), "kampot".into(), "kandal".into(), "koh kong".into(), "kratie".into(), "mondulkiri".into(), "oddar meancheay".into(), "pailin".into(), "preah sihanouk".into(), "preah vihear".into(), "prey veng".into(), "pursat".into(), "ratanakiri".into(), "stung treng".into(), "svay rieng".into(), "takeo".into(), "tbong khmum".into(), "tboung khmom".into()]),
            misc: Some(vec!["funcinpec".into()]),
        }.get_region_vec(), RegionType::Cambodia);
        map.insert(RegionKeyphrases {
            names: Some(vec!["cameroon".into()]),
            demonyms: None,
            capitals: Some(vec!["yaounde".into()]),
            relevant_figures: Some(vec!["paul biya".into(), "joseph ngute".into(), "marcel niat njifenji".into(), "cavaye yeguie djibril".into()]),
            relevant_cities: Some(vec!["douala".into(), "garoua".into(), "kousseri".into(), "bamenda".into(), "maroua".into(), "bafoussam".into(), "mokolo".into(), "gaoundere".into(), "bertoua".into(), "edea".into()]),
            subregions: Some(vec!["adamawa".into(), "bakassi".into()]),
            misc: Some(vec!["unity palace".into(), "crtv".into(), "rdpc".into(), "ambazonia".into()]),
        }.get_region_vec(), RegionType::Cameroon);
        map.insert(RegionKeyphrases {
            names: Some(vec!["canada".into()]),
            demonyms: Some(vec!["canadian".into()]),
            capitals: Some(vec!["ottawa".into()]),
            relevant_figures: Some(vec!["king charles".into(), "charles iii".into(), "mary simon".into(), "trudeau".into()]),
            relevant_cities: Some(vec!["toronto".into(), "montreal".into(), "vancouver".into(), "calgary".into(), "edmonton".into(), "winnipeg".into(), "hamilton".into(), "kitchener".into(), "saskatoon".into()]),
            subregions: Some(vec!["alberta".into(), "british columbia".into(), "manitoba".into(), "new brunswick".into(), "newfoundland and labrador".into(), "nova scotia".into(), "ontario".into(), "prince edward island".into(), "quebec".into(), "saskatchewan".into(), "northwest territories".into(), "nunavut".into(), "yukon".into()]),
            misc: Some(vec!["parliament hill".into(), "rcmp".into(), "ndp".into(), "quebecois".into(), "metis".into(), "first nations".into()]),
        }.get_region_vec(), RegionType::Canada);
        map.insert(RegionKeyphrases {
            names: Some(vec!["cape verde".into()]),
            demonyms: Some(vec!["cabo verdean".into()]),
            capitals: Some(vec!["praia".into()]),
            relevant_figures: Some(vec!["jose maria neves".into(), "correia e silva".into()]),
            relevant_cities: None,
            subregions: Some(vec!["boavista".into(), "boa vista".into(), "brava".into(), "maio".into(), "fogo".into(), "santa luzia".into(), "santo antao".into(), "sao nicolau".into(), "sao vicente".into()]),
            misc: Some(vec!["paicv".into()]),
        }.get_region_vec(), RegionType::CapeVerde);
        map.insert(RegionKeyphrases {
            names: None,
            demonyms: Some(vec!["central african".into()]),
            capitals: Some(vec!["bangui".into()]),
            relevant_figures: Some(vec!["touadera".into(), "felix moloua".into(), "simplice sarandji".into()]),
            relevant_cities: None,
            subregions: Some(vec!["bamingui-bangoran".into(), "basse-kotto".into(), "haute-kotto".into(), "haut-mbomou".into(), "kemo".into(), "lobaye".into(), "lim-pende".into(), "mambere".into(), "mbomou".into(), "nana-grebizi".into(), "ombella-m'poko".into(), "ouaka".into(), "ouham".into(), "sangha-mbaere".into(), "vakaga".into()]),
            misc: Some(vec!["united hearts movement".into(), "kwa na kwa".into(), "fprc".into(), "anti-balaka".into()]),
        }.get_region_vec(), RegionType::CentralAfricanRepublic);
        map.insert(RegionKeyphrases {
            names: Some(vec!["chad".into()]),
            demonyms: None,
            capitals: Some(vec!["n'djamena".into()]),
            relevant_figures: Some(vec!["mahamat deby".into(), "succes masra".into()]),
            relevant_cities: None,
            subregions: Some(vec!["batha".into(), "bahr el gazel".into(), "borkou".into(), "ouaddai".into(), "wadi fira".into(), "mayo-kebbi est".into(), "logone oriental".into(), "ennedi-est".into(), "guera".into(), "mayo-kebbi ouest".into(), "logone occidental".into(), "ennedi-ouest".into(), "kanem".into(), "mandoul".into(), "salamat".into(), "sila".into(), "moyen-chari".into(), "tandjile".into(), "tibesti".into(), "chari-baguirmi".into(), "hadjer-lamis".into()]),
            misc: Some(vec!["national transitional council".into()]),
        }.get_region_vec(), RegionType::Chad);
        map.insert(RegionKeyphrases {
            names: Some(vec!["chile".into()]),
            demonyms: None,
            capitals: None,
            relevant_figures: Some(vec!["gabriel boric".into(), "juan antonio coloma".into(), "ricardo cifuentes".into(), "juan fuentes belmar".into(),]),
            relevant_cities: Some(vec!["puente alto".into(), "maipu".into(), "la florida".into(), "vina del mar".into(), "antofagasta".into(), "valparaiso".into(), "las condes".into(), "san bernardo".into(), "temuco".into(), "penalolen".into(), "concepcion".into(), "rancagua".into(), "pudahuel".into()]),
            subregions: Some(vec!["aisen".into(), "aysen".into(), "arica y parinacota".into(), "tarapaca".into(), "antofagasta".into(), "atacama".into(), "coquimbo".into(), "valparaiso".into(), "bernardo ohiggins".into(), "bernardo o'higgins".into(), "maule".into(), "nuble".into(), "biobio".into(), "araucania".into(), "los rios".into(), "los lagos".into(), "magallanes".into()]),
            misc: None,
        }.get_region_vec(), RegionType::Chile);
        map.insert(RegionKeyphrases {
            names: Some(vec!["china".into(), "prc".into()]),
            demonyms: Some(vec!["chinese".into()]),
            capitals: Some(vec!["beijing".into()]),
            relevant_figures: Some(vec!["xi jinping".into(), "li qiang".into(), "wang huning".into(), "han zheng".into()]),
            relevant_cities: Some(vec!["hong kong".into(), "taipei".into(), "kaohsiung".into(), "taichung".into(), "taoyuan".into(), "tainan".into(), "hsinchu".into(), "keelung".into(), "chiayi".into(), "changhua".into(), "pingtung".into(), "zhubei".into(), "shanghai".into(), "guangzhou".into(), "chengdu".into(), "chongqing".into(), "shenzhen".into(), "tianjin".into(), "wuhan".into(), "xi'an".into(), "hangzhou".into(), "dongguan".into(), "foshan".into(), "nanjing".into(), "jinan".into(), "shenyang".into(), "qingdao".into(), "harbin".into(), "zhengzhou".into(), "changsha".into(), "kunming".into(), "dalian".into(), "changchun".into(), "xiamen".into(), "ningbo".into(), "taiyuan".into(), "zhongshan".into(), "urumqi".into(), "suzhou".into(), "shantou".into(), "hefei".into(), "shijiazhuang".into(), "fuzhou".into(), "nanning".into(), "wenzhou".into(), "changzhou".into(), "nanchang".into(), "guiyang".into(), "tangshan".into(), "wuxi".into(), "lanzhou".into(), "handan".into(), "hohhot".into(), "weifang".into(), "jiangmen".into(), "zibo".into(), "huai'an".into(), "xuzhou".into(), "maoming".into(), "shaoxing".into(), "yantai".into(), "huizhou".into(), "zhuhai".into(), "luoyang".into(), "linyi".into(), "nantong".into(), "haikou".into(), "baotou".into(), "liuzhou".into(), "datong".into(), "putian".into(), "lianyungang".into(), "baoding".into(), "xining".into(), "zhanjiang".into(), "wuhu".into(), "chaozhou".into(), "qingyuan".into(), "tai'an".into(), "yichang".into(), "yangzhou".into(), "yinchuan".into(), "xiEangyang".into(), "anshan".into(), "jilin city".into(), "yancheng".into(), "taizhou".into(), "qinhuangdao".into(), "ganzhou".into(), "daqing".into(), "guilin".into(), "huzhou".into(), "zhaoqing".into(), "jiaxing".into(), "jining".into(), "jinhua".into(), "changde".into(), "hengyang".into(), "suqian".into(), "baoji".into(), "zhangjiakou".into(), "mianyang".into(), "qiqihar".into(), "heze".into(), "fushun".into(), "yangjiang".into(), "liaocheng".into(), "tianshui".into(), "benxi".into(), "chifeng".into(), "jiujiang".into(), "anyang".into(), "huaibei".into(), "yulin".into(), "xinxiang".into(), "shaoguan".into(), "dongying".into(), "luzhou".into(), "meizhou".into(), "leshan".into(), "dezhou".into(), "xingtai".into(), "chenzhou".into(), "mudanjiang".into(), "tongliao".into(), "chengde".into(), "laiwu".into(), "taishan".into(), "quzhou".into(), "zhoushan".into(), "suihua".into(), "langfang".into(), "hengshui".into(), "yingkou".into(), "panjin".into(), "weihai".into(), "anqing".into(), "liaoyang".into(), "puyang".into(), "fuxin".into(), "jieyang".into(), "yangquan".into(), "jiamusi".into(), "huludao".into(), "zhumadian".into(), "kashgar".into(), "dazhou".into(), "heyuan".into(), "longyan".into(), "aksu city".into(), "ordos city".into(), "hegang".into(), "binzhou".into(), "siping".into(), "sanmenxia".into(), "dandong".into(), "suining".into(), "sanya".into(), "ji'an".into(), "cangzhou".into(), "qitaihe".into(), "yichun".into(), "tonghua".into(), "jixi".into(), "korla".into(), "chaoyang".into(), "dingxi".into(), "shuangyashan".into(), "songyuan".into(), "nanping".into(), "liaoyuan".into(), "lhasa".into(), "karamay".into(), "shanwei".into(), "tieling".into(), "suihua".into(), "ulanqab".into(), "hami".into(), "huangshan city".into(), "hotan".into(), "wuwei".into(), "baishan".into(), "sanming".into(), "yunfu".into(), "hailar".into(), "zhaotong".into(), "ningde".into(), "baicheng".into(), "hunchun".into(), "zhangjiajie".into(), "golmud".into()]),
            subregions: Some(vec!["anhui".into(), "fujian".into(), "gansu".into(), "guangdong".into(), "guizhou".into(), "hainan".into(), "hebei".into(), "heilongjiang".into(), "henan".into(), "hubei".into(), "hunan".into(), "jiangsu".into(), "jiangxi".into(), "jilin".into(), "liaoning".into(), "macao".into(), "macau".into(), "qinghai".into(), "shaanxi".into(), "shandong".into(), "shanxi".into(), "sichuan".into(), "taiwan".into(), "yunnan".into(), "zhejiang".into(), "wolong".into(), "xinjiang".into(), "tibet".into()]),
            misc: Some(vec!["national people's congress".into(), "cppcc".into(), "ccp".into(), "kuomintang".into(), "guomindang".into()]),
        }.get_region_vec(), RegionType::China);
        map.insert(RegionKeyphrases {
            names: Some(vec!["colombia".into()]),
            demonyms: None,
            capitals: Some(vec!["bogota".into()]),
            relevant_figures: Some(vec!["gustavo petro".into(), "francia marquez".into()]),
            relevant_cities: Some(vec!["medellin".into(), "barranquilla".into(), "cartagena".into(), "cucuta".into(), "soledad".into(), "ibague".into(), "soacha".into(), "bucaramanga".into(), "santa marta".into(), "valledupar".into(), "bello".into(), "pereira".into(), "monteria".into(), "san juan de pasto".into(), "buenaventura".into(), "manizales".into(), "neiva".into(), "palmira".into()]),
            subregions: Some(vec!["amazonas".into(), "antioquia".into(), "arauca".into(), "atlantico".into(), "bolivar".into(), "boyaca".into(), "caldas".into(), "caqueta".into(), "casanare".into(), "cauca".into(), "cesar".into(), "choco".into(), "cordoba".into(), "cundinamarca".into(), "guainia".into(), "guaviare".into(), "huila".into(), "la guajira".into(), "magdalena".into(), "meta".into(), "narino".into(), "norte de santander".into(), "putumayo".into(), "quindio".into(), "risaralda".into(), "san andres, providencia, and santa catalina".into(), "santander".into(), "sucre".into(), "tolima".into(), "valle del cauca".into(), "vaupes".into(), "vichada".into()]),
            misc: Some(vec!["casa de narino".into(), "capitolio nacional".into(), "eln".into()]),
        }.get_region_vec(), RegionType::Colombia);
        map.insert(RegionKeyphrases {
            names: Some(vec!["comoros".into()]),
            demonyms: Some(vec!["comorian".into()]),
            capitals: Some(vec!["moroni".into()]),
            relevant_figures: Some(vec!["azali assoumani".into(), "ahemd abdallah ali".into()]),
            relevant_cities: None,
            subregions: Some(vec!["ngazidja".into(), "ndzuwani".into(), "mwali".into()]),
            misc: Some(vec!["orange party".into(), "republican organization for the future of new generations".into()]),
        }.get_region_vec(), RegionType::Comoros);
        map.insert(RegionKeyphrases {
            names: Some(vec!["little congo".into()]),
            demonyms: None,
            capitals: Some(vec!["brazzaville".into()]),
            relevant_figures: Some(vec!["denis sassou nguesso".into(), "anatole collinet makosso".into()]),
            relevant_cities: Some(vec!["pointe-noire".into()]),
            subregions: Some(vec!["bouenza".into(), "cuvette".into(), "cuvette-ouest".into(), "kouilou".into(), "lekoumou".into(), "likouala".into(), "niari".into(), "plateaux".into(), "sangha".into()]),
            misc: Some(vec!["congolese party of labour".into(), "upads".into()]),
        }.get_region_vec(), RegionType::Congo);
        map.insert(RegionKeyphrases {
            names: Some(vec!["democratic republic of the congo".into(), "drc".into(), "big congo".into()]),
            demonyms: None,
            capitals: Some(vec!["kinshasa".into()]),
            relevant_figures: Some(vec!["felix tshisekedi".into(), "sama lukonde".into()]),
            relevant_cities: Some(vec!["lubumbashi".into(), "mbuji-mayi".into(), "bukavu".into(), "kananga".into(), "kisangani".into(), "tshikapa".into(), "kolwezi".into(), "likasi".into(), "goma".into(), "kikwit".into(), "uvira".into(), "bunia".into(), "mbandaka".into(), "matadi".into(), "butembo".into(), "kabinda".into(), "mwene-ditu".into()]),
            subregions: Some(vec!["kongo central".into(), "kwango".into(), "kwilu".into(), "mai-ndombe".into(), "kasai".into(), "lomami".into(), "sankuru".into(), "maniema".into(), "kivu".into(), "ituri".into(), "haut-uele".into(), "tshopo".into(), "bas-uele".into(), "ubangi".into(), "mongala".into(), "equateur".into(), "tshuapa".into(), "tanganyika".into(), "lualaba".into(), "haut-katanga".into()]),
            misc: Some(vec!["udps".into(), "common front for congo".into(), "kabila coalition".into(), "lamuka".into(), "fardc".into()]),
        }.get_region_vec(), RegionType::DemocraticRepublicOfTheCongo);
        map.insert(RegionKeyphrases {
            names: Some(vec!["costa rica".into()]),
            demonyms: None,
            capitals: None,
            relevant_figures: Some(vec!["rodrigo chaves".into(), "stephan brunner".into(), "mary munive".into()]),
            relevant_cities: None,
            subregions: Some(vec!["alajuela".into(), "cartago".into(), "guanacaste".into(), "heredia".into(), "limon".into(), "puntarenas".into()]),
            misc: Some(vec!["inter-american court of human rights".into(), "social democratic progress party".into(), "national liberation party".into(), "verdiblancos".into()]),
        }.get_region_vec(), RegionType::CostaRica);
        map.insert(RegionKeyphrases {
            names: Some(vec!["ivory coast".into(), "cote d'ivoire".into()]),
            demonyms: Some(vec!["ivorian".into()]),
            capitals: Some(vec!["yamoussoukro".into()]),
            relevant_figures: Some(vec!["alassane ouattara".into(), "tiemoko meyliet kone".into(), "robert beugre mambe".into()]),
            relevant_cities: Some(vec!["abidjan".into(), "bouake".into(), "daloa".into()]),
            subregions: Some(vec!["bas-sassandra".into(), "comoe".into(), "denguele".into(), "goh-djiboua".into(), "lacs".into(), "lagunes".into(), "montagnes".into(), "sassandra-marahoue".into(), "savanes".into(), "vallee du bandama".into(), "woroba".into(), "zanzan".into()]),
            misc: Some(vec!["compagnie ivoirienne".into()]),
        }.get_region_vec(), RegionType::IvoryCoast);
        map.insert(RegionKeyphrases {
            names: Some(vec!["croatia".into()]),
            demonyms: None,
            capitals: Some(vec!["zagreb".into()]),
            relevant_figures: Some(vec!["zoran milanovic".into(), "andrej plenkovic".into()]),
            relevant_cities: None,
            subregions: Some(vec!["bjelovar-bilogora".into(), "brod-posavina".into(), "dubrovnik-neretva".into(), "istria".into(), "karlovac".into(), "koprivnica-krizevci".into(), "krapina-zagorje".into(), "lika-senj".into(), "medimurje".into(), "osijek-baranja".into(), "pozega-slavonia".into(), "primorje-gorski kotar".into(), "sibenik-knin".into(), "varazdin".into(), "virovitica-podravina".into(), "vukovar-syrmia".into(), "zadar".into()]),
            misc: Some(vec!["sabor".into(), "banski dvori".into(), "hdz".into()]),
        }.get_region_vec(), RegionType::Croatia);
        map.insert(RegionKeyphrases {
            names: Some(vec!["cuba".into()]),
            demonyms: None,
            capitals: Some(vec!["havana".into()]),
            relevant_figures: Some(vec!["miguel diaz-canel".into(), "salvador valdes".into(), "manuel marrero cruz".into(), "esteban lazo hernandez".into()]),
            relevant_cities: Some(vec!["santiago de cuba".into(), "camaguey".into(), "holguin".into(), "guantanamo".into()]),
            subregions: Some(vec!["artemisa".into(), "camaguey".into(), "ciego de avila".into(), "cienfuegos".into(), "granma".into(), "holguin".into(), "isla de la juventud".into(), "la habana".into(), "las tunas".into(), "matanzas".into(), "mayabeque".into(), "pinar del rio".into(), "sancti spiritus".into(), "santiago de cuba".into(), "villa clara".into()]),
            misc: Some(vec!["national assembly of people's power".into()]),
        }.get_region_vec(), RegionType::Cuba);
        map.insert(RegionKeyphrases {
            names: Some(vec!["cyprus".into()]),
            demonyms: Some(vec!["cypriot".into()]),
            capitals: Some(vec!["nicosia".into()]),
            relevant_figures: Some(vec!["nikos christodoulides".into(), "annita demetriou".into()]),
            relevant_cities: None,
            subregions: Some(vec!["kyrenia".into(), "limassol".into(), "larnaca".into(), "paphos".into(), "famagusta".into()]),
            misc: Some(vec!["akel".into()]),
        }.get_region_vec(), RegionType::Cyprus);
        map.insert(RegionKeyphrases {
            names: None,
            demonyms: Some(vec!["czech".into()]),
            capitals: Some(vec!["prague".into()]),
            relevant_figures: Some(vec!["petr pavel".into(), "petr fiala".into()]),
            relevant_cities: Some(vec!["brno".into(), "ostrava".into()]),
            subregions: Some(vec!["bohemia".into(), "plzen".into(), "karlovy vary".into(), "carlsbad region".into(), "usti nad labem".into(), "ustecky".into(), "liberec".into(), "hradec kralove".into(), "pardubice".into(), "vysocina".into(), "south moravian".into(), "olomouc".into(), "zlin".into(), "moravian-silesian".into()]),
            misc: Some(vec!["spolu".into(), "ano 2011".into()]),
        }.get_region_vec(), RegionType::CzechRepublic);
        map
    };
}