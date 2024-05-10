use lazy_static::lazy_static;
use std::collections::HashMap;

#[derive(Debug, Eq, PartialEq, Hash)]
pub enum Region { // https://en.wikipedia.org/wiki/List_of_alternative_country_names
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
    DemocraticRepublicOfCongo,
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
//use python to count words and look for any duplicates
//convert accent markers and non-english words in input texts to work with these
//test substrings with apostrophes and periods
lazy_static! {//once it finds a hit within a vec, dont bother with checking the rest within the vec and move onto next region
    pub static ref REGION_MAP = { // name, demonym, capital (largest cities too), relevant figure(s), subregions. redundancies removed.
        let mut map = HashMap::new();
        map.insert(// georgia: "ruslan abashidze", "jemal gamakharia",
                vec!["abkhaz", "sukhumi", "aslan bzhania", "alexander ankvab", "ruslan abashidze", "jemal gamakharia", "gagra", "gali", "gudauta", "gulripshi", "ochamchira", "tkvarcheli"],
                Region::Abkhazia);
        map.insert(
                vec!["afghan", "kabul", "hibatullah akhundzada", "haibatullah akhunzada", "hasan akhund", "abdul hakim", "kandahar", "herat", "mazar-i-sharif", "mazar-e-sharif", "jalalabad", "kunduz", "lashkargah"],
                Region::Afghanistan);
        map.insert(
                vec!["albania", "tirana", "bajram begaj", "edi rama", "lindita nikolla", "berat", "dibër", "durrës", "elbasan", "fier", "gjirokastër", "korce", "kukes", "lezhe", "shkoder", "tirana", "vlore"],
                Region::Albania);
        map.insert(
                vec!["algeria", "algiers", "abdelmadjid tebboune", "nadir larbaoui", "salah goudjil", "ibrahim boughali", "adrar", "chlef", "laghouat", "oum el bouaghi", "batna", "bejaia", "biskra", "bechar", "blida", "bouira", "tamanrasset", "tebessa", "tlemcen", "tiaret", "tizi ouzou", "algiers", "djelfa", "jijel", "setif", "saida", "skikda", "sidi bel abbes", "annaba", "guelma", "constantine", "medea", "mostaganem", "m'sila", "mascara", "ouargla", "oran", "el bayadh", "illizi", "bordj bou arreridj", "boumerdes", "el taref", "tindouf", "tissemsilt", "el oued", "khenchela", "souk ahras", "tipaza", "mila", "ain defla", "naama", "ain timouchent", "ghardaia", "relizane", "el m'ghair", "el menia", "ouled djellal", "bordj baji mokhtar", "beni abbes", "timimoun", "touggourt", "djanet", "in salah", "in guezzam"],
                Region::Algeria);
        map.insert(
                vec!["andorra", "xavier espot zamora", "joan enric vives", "emmanuel macron", "macron", "josep maria mauri", "patrick strzoda", "carles ensenat reig", "canillo", "encamp ", "escaldes-engordany", "la massana", "ordino", "sant julia de loria"],
                Region::Andorra);
        map.insert(
                vec!["angola", "luanda", "joao lourenco", "esperanca da costa", "bengo", "benguela", "bie", "cabinda", "cuando cubango", "cuanza norte", "cuanza sul", "cunene", "huambo", "huila", "luanda", "lunda norte", "lunda sul", "malanje", "moxico", "namibe", "uíge", "zaire"],
                Region::Angola);
        map.insert(
                vec!["antigua", "barbuda", "saint john's", "st. john's", "charles iii", "king charles", "gaston browne", "sir rodney williams", "codrington", "all saints", "piggotts", "liberta"],
                Region::AntiguaAndBarbuda);
        map.insert(
                vec!["argentina", "argentine" "argentinian", "argentinean", "buenos aires", "javier milei", "victoria villarruel", "nicolas posse", "martin menem", "horacio rosatti", "catamarca", "chaco", "chubut", "cordoba", "corrientes", "entre rios", "formosa", "jujuy", "la pampa", "la rioja", "mendoza", "misiones", "neuquen", "rio negro", "salta", "san juan", "san luis", "santa cruz", "santa fe", "santiago del estero", "tierra del fuego", "tucuman"],
                Region::Argentina);
        map.insert(
                vec!["armenia", "yerevan", "vahagn khachaturyan", "nikol pashinyan", "alen simonyan", "aragatsotn", "ararat", "armavir", "gegharkunik", "kotayk", "lori", "shirak", "syunik", "tavush", "vayots dzor", "yerevan"],
                Region::Armenia);
        map.insert(
                vec!["australia", "aussie", "canberra", "charles iii", "king charles", "david hurley", "anthony albanese", "sydney", "melbourne", "queensland", "new south wales", "tasmania", "jervis bay", "coral sea islands", "norfolk island", "northern territory", "ashmore and cartier", "christmas island", "cocos islands", "keeling islands", "heard island", "mcdonald islands"],
                Region::Australia);
        map.insert(
                vec!["austria", "vienna", "alexander van der bellen", "karl nehammer", "burgenland", "carinthia", "lower austria", "upper austria", "salzburg", "styria", "tyrol", "vorarlberg"],
                Region::Austria);
        map.insert(
                vec!["azerbaijan", "baku", "ilham aliyev", "mehriban aliyeva", "sahiba gafarova", "ali asadov", "absheron", "khizi", "sumgait", "aghdash", "goychay", "kurdamir", "ujar", "yevlakh", "zardab", "mingachevir", "beylagan", "imishli", "saatly", "sabirabad", "bilasuvar", "hajigabul", "neftchala", "salyan", "shirvan", "aghsu", "gobustan", "ismayilli", "shamakhi", "dashkasan", "goranboy", "goygol", "samukh", "ganja", "naftalan", "aghstafa", "gadabay", "gazakh", "shamkir", "tovuz", "guba", "gusar", "khachmaz", "shabran", "siyazan", "gubadly", "jabrayil", "kalbajar", "lachin", "zangilan", "astara", "jalilabad", "lankaran", "lerik", "masally", "yardimli", "nakhchivan", "babek", "julfa", "kangarli", "ordubad", "sadarak", "shahbuz", "sharur", "shaki", "zagatala", "balakan", "gabala", "gakh", "oghuz", "shaki", "karabakh", "aghjabadi", "barda", "aghdam", "fuzuli", "khojaly", "khojavend", "shusha", "tartar", "khankendi"],
                Region::Azerbaijan);
        map.insert(
                vec!["bahamas", "bahamian", "nassau", "charles iii", "king charles", "philip davis", "new providence", "acklins", "berry islands", "bimini", "black point", "cat island", "central abaco", "central andros", "central eleuthera", "city of freeport", "crooked island", "east grand bahama", "exuma", "grand cay", "harbour island", "hope town", "inagua", "long island", "mangrove cay", "mayaguana", "moore's island", "north abaco", "north andros", "north eleuthera", "ragged island", "rum cay", "san salvador", "south abaco", "south andros", "south eleuthera", "spanish wells", "west grand bahama"],
                Region::TheBahamas);
        map.insert(
                vec!["bahrain", "manama", "hamad bin isa al khalifa", "salman bin hamad al khalifa", "khalifa bin salman al khalifa", "muharraq", "capital governorate", "northern governorate", "southern governorate"],
                Region::Bahrain);
        map.insert(
                vec!["bangladesh", "dhaka", "mohammed shahabuddin", "sheikh hasina", "shirin sharmin chaudhury", "obaidul hassan", "barisal", "chittagong", "dhaka", "khulna", "mymensingh", "rajshahi", "rangpur", "sylhet"],
                Region::Bangladesh);
        map.insert(
                vec!["barbados", "barbadian", "bajan", "bridgetown", "sandra mason", "mia mottley", "christ church", "saint andrew", "saint george", "saint james", "saint john", "saint joseph", "saint lucy", "saint michael", "saint peter", "saint philip", "saint thomas"],
                Region::Barbados);
        map.insert(
                vec!["belarus", "lukashenko", "roman golovchenko", "minsk oblast", "brest", "gomel", "grodno", "mogilev", "vitebsk"],
                Region::Belarus);
        map.insert(
                vec!["belgium", "belgian", "brussels", "philippe" "alexander de croo", "flemish", "flanders", "walloon", "wallonia", "east cantons"],
                Region::Belgium);
        map.insert(
                vec!["belize", "belmopan", "charles iii", "king charles", "dame froyla tzalam", "johnny briceno", "cayo district", "corozal district", "orange walk district", "stann creek district", "toledo district"],
                Region::Belize);
        map.insert(
                vec!["benin", "porto-novo", "patrice talon", "mariam chabi talata", "cotonou", "alibori", "atakora", "atlantique", "borgou", "collines", "kouffo", "donga", "littoral", "mono department", "oueme", "plateau department", "zou"],
                Region::Benin);
        map.insert(
                vec!["bhutan", "thimphu", "wangchuck", "tshering tobgay", "bumthang", "chukha", "dagana", "gasa", "haa", "lhuntse", "mongar", "paro", "pemagatshel", "punakha", "samdrup jongkhar", "samtse", "sarpang", "thimphu", "trashigang", "trashiyangtse", "trongsa", "tsirang", "wangdue phodrang", "zhemgang"],
                Region::Bhutan);
        map.insert(
                vec!["bolivia", "sucre", "david choquehuanca", "luis arce", "lucho", "andronico rodriguez" "santa cruz de la sierra", "beni", "chuquisaca", "cochabamba", "la paz", "oruro", "pando", "potosi", "santa cruz", "tarija"],
                Region::Bolivia);
        map.insert(
                vec!["bosnia", "herzegovina", "herzegovinian", "sarajevo", "christian schmidt", "denis becirovic", "zeljka cvijanovic", "zeljko komsic", "borjana kristo", "banja luka", "tuzla", "zenica", "bijeljina", "mostar", "prijedor", "brcko", "doboj", "cazin", "zvornik", "zivinice", "bihac", "travnik", "gradiska", "gracanica", "lukavac", "tesanj", "sanski most", "velika kladusa"],
                Region::BosniaAndHerzegovina);
        map.insert(
                vec!["botswana", "batswana", "motswana", "gaborone", "mokgweetsi masisi", "slumber tsogwane", "phandu skelemani", "kweneng", "kgatleng", "ngamiland", "kgalagadi", "chobe", "ghanzi"],
                Region::Botswana);
        map.insert(
                vec!["brazil", "brasilia", "lula", "geraldo alckmin", "arthur lira", "rodrigo pacheco", "luis roberto barroso", "sao paulo", "alagoas", "amapa", "amazonas", "bahia", "ceara", "distrito federal", "espirito santo", "goias", "maranhao", "mato grosso", "mato grosso do sul", "minas gerais", "para", "paraiba", "parana", "pernambuco", "piaui", "rio de janeiro", "rio grande do norte", "rio grande do sul", "rondonia", "roraima", "santa catarina", "sergipe", "tocantins"],
                Region::Brazil);
        map.insert(
                vec!["brunei", "bandar seri begawan", "hassanal bolkiah", "al-muhtadee billah", "abdul aziz juned", "belait", "tutong", "temburong"],
                Region::Brunei);
        map.insert(
                vec!["bulgaria", "sofia", "rumen radev", "iliana iotova", "kiril petkov", "dimitar glavchev", "blagoevgrad", "burgas", "dobrich", "gabrovo", "haskovo", "kardzhali", "kyustendil", "lovech", "pazardzhik", "pernik", "pleven", "plovdiv", "razgrad", "shumen", "silistra", "sliven", "smolyan", "sofia province", "stara zagora", "targovishte", "varna", "veliko tarnovo", "vidin", "vratsa", "yambol"],
                Region::Bulgaria);
        map.insert(
                vec!["burkina faso", "ouagadougou", "ibrahim traore", "apollinaire", "tambela", "dedougou", "banfora", "tenkodogo", "kaya", "koudougou", "manga", "fada n'gourma", "bobo dioulasso", "ouahigouya", "ziniare", "dori", "gaoua"],
                Region::BurkinaFaso);
        map.insert(
                vec!["burundi", "bujumbura", "gitega", "bujumbura", "evariste ndayishimiye", "prosper bazombanza", "gervais ndirakobuca", "cankuzo", "gitega", "rutana", "ruyigi", "karuzi", "kayanza", "kirundo", "muyinga", "ngozi", "bururi", "makamba", "rumonge", "bubanza", "bujumbura mairie", "bujumbura rural", "cibitoke", "muramvya", "mwaro"],
                Region::Burundi);








        map
    }
}