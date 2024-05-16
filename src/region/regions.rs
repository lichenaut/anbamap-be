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

// EC	ECU	218	EC	Ecuador	Quito	283560	17084357	SA	.ec	USD	Dollar	593	@####@	^([a-zA-Z]\d{4}[a-zA-Z])$	es-EC	3658394	PE,CO	
// EE	EST	233	EN	Estonia	Tallinn	45226	1320884	EU	.ee	EUR	Euro	372	#####	^(\d{5})$	et,ru	453733	RU,LV	
// EG	EGY	818	EG	Egypt	Cairo	1001450	98423595	AF	.eg	EGP	Pound	20	#####	^(\d{5})$	ar-EG,en,fr	357994	LY,SD,IL,PS	
// EH	ESH	732	WI	Western Sahara	El-Aaiun	266000	273008	AF	.eh	MAD	Dirham	212			ar,mey	2461445	DZ,MR,MA	
// ER	ERI	232	ER	Eritrea	Asmara	121320	6209262	AF	.er	ERN	Nakfa	291			aa-ER,ar,tig,kun,ti-ER	338010	ET,SD,DJ	
// ES	ESP	724	SP	Spain	Madrid	504782	46723749	EU	.es	EUR	Euro	34	#####	^(\d{5})$	es-ES,ca,gl,eu,oc	2510769	AD,PT,GI,FR,MA	
// ET	ETH	231	ET	Ethiopia	Addis Ababa	1127127	109224559	AF	.et	ETB	Birr	251	####	^(\d{4})$	am,en-ET,om-ET,ti-ET,so-ET,sid	337996	ER,KE,SD,SS,SO,DJ	
// FI	FIN	246	FI	Finland	Helsinki	337030	5518050	EU	.fi	EUR	Euro	358	#####	^(?:FI)*(\d{5})$	fi-FI,sv-FI,smn	660013	NO,RU,SE	
// FJ	FJI	242	FJ	Fiji	Suva	18270	883483	OC	.fj	FJD	Dollar	679			en-FJ,fj	2205218		
// FK	FLK	238	FK	Falkland Islands	Stanley	12173	2638	SA	.fk	FKP	Pound	500			en-FK	3474414		
// FM	FSM	583	FM	Micronesia	Palikir	702	112640	OC	.fm	USD	Dollar	691	#####	^(\d{5})$	en-FM,chk,pon,yap,kos,uli,woe,nkr,kpg	2081918		
// FO	FRO	234	FO	Faroe Islands	Torshavn	1399	48497	EU	.fo	DKK	Krone	298	###	^(?:FO)*(\d{3})$	fo,da-FO	2622320		
// FR	FRA	250	FR	France	Paris	547030	66987244	EU	.fr	EUR	Euro	33	#####	^(\d{5})$	fr-FR,frp,br,co,ca,eu,oc	3017382	CH,DE,BE,LU,IT,AD,MC,ES	
// GA	GAB	266	GB	Gabon	Libreville	267667	2119275	AF	.ga	XAF	Franc	241			fr-GA	2400553	CM,GQ,CG	
// GB	GBR	826	UK	United Kingdom	London	244820	66488991	EU	.uk	GBP	Pound	44	@# #@@|@## #@@|@@# #@@|@@## #@@|@#@ #@@|@@#@ #@@|GIR0AA	^([Gg][Ii][Rr]\s?0[Aa]{2})|((([A-Za-z][0-9]{1,2})|(([A-Za-z][A-Ha-hJ-Yj-y][0-9]{1,2})|(([A-Za-z][0-9][A-Za-z])|([A-Za-z][A-Ha-hJ-Yj-y][0-9]?[A-Za-z]))))\s?[0-9][A-Za-z]{2})$	en-GB,cy-GB,gd	2635167	IE	
// GD	GRD	308	GJ	Grenada	St. George's	344	111454	NA	.gd	XCD	Dollar	+1-473			en-GD	3580239		
// GE	GEO	268	GG	Georgia	Tbilisi	69700	3731000	AS	.ge	GEL	Lari	995	####	^(\d{4})$	ka,ru,hy,az	614540	AM,AZ,TR,RU	
// GF	GUF	254	FG	French Guiana	Cayenne	91000	195506	SA	.gf	EUR	Euro	594	#####	^((97|98)3\d{2})$	fr-GF	3381670	SR,BR	
// GG	GGY	831	GK	Guernsey	St Peter Port	78	65228	EU	.gg	GBP	Pound	+44-1481	@# #@@|@## #@@|@@# #@@|@@## #@@|@#@ #@@|@@#@ #@@|GIR0AA	^((?:(?:[A-PR-UWYZ][A-HK-Y]\d[ABEHMNPRV-Y0-9]|[A-PR-UWYZ]\d[A-HJKPS-UW0-9])\s\d[ABD-HJLNP-UW-Z]{2})|GIR\s?0AA)$	en,nrf	3042362		
// GH	GHA	288	GH	Ghana	Accra	239460	29767108	AF	.gh	GHS	Cedi	233			en-GH,ak,ee,tw	2300660	CI,TG,BF	
// GI	GIB	292	GI	Gibraltar	Gibraltar	6.5	33718	EU	.gi	GIP	Pound	350			en-GI,es,it,pt	2411586	ES	
// GL	GRL	304	GL	Greenland	Nuuk	2166086	56025	NA	.gl	DKK	Krone	299	####	^(\d{4})$	kl,da-GL,en	3425505		
// GM	GMB	270	GA	Gambia	Banjul	11300	2280102	AF	.gm	GMD	Dalasi	220			en-GM,mnk,wof,wo,ff	2413451	SN	
// GN	GIN	324	GV	Guinea	Conakry	245857	12414318	AF	.gn	GNF	Franc	224			fr-GN	2420477	LR,SN,SL,CI,GW,ML	
// GP	GLP	312	GP	Guadeloupe	Basse-Terre	1780	443000	NA	.gp	EUR	Euro	590	#####	^((97|98)\d{3})$	fr-GP	3579143		
// GQ	GNQ	226	EK	Equatorial Guinea	Malabo	28051	1308974	AF	.gq	XAF	Franc	240			es-GQ,fr,pt	2309096	GA,CM	
// GR	GRC	300	GR	Greece	Athens	131940	10727668	EU	.gr	EUR	Euro	30	### ##	^(\d{5})$	el-GR,en,fr	390903	AL,MK,TR,BG	
// GS	SGS	239	SX	South Georgia and the South Sandwich Islands	Grytviken	3903	30	AN	.gs	GBP	Pound				en	3474415		
// GT	GTM	320	GT	Guatemala	Guatemala City	108890	17247807	NA	.gt	GTQ	Quetzal	502	#####	^(\d{5})$	es-GT	3595528	MX,HN,BZ,SV	
// GU	GUM	316	GQ	Guam	Hagatna	549	165768	OC	.gu	USD	Dollar	+1-671	969##	^(969\d{2})$	en-GU,ch-GU	4043988		
// GW	GNB	624	PU	Guinea-Bissau	Bissau	36120	1874309	AF	.gw	XOF	Franc	245	####	^(\d{4})$	pt-GW,pov	2372248	SN,GN	
// GY	GUY	328	GY	Guyana	Georgetown	214970	779004	SA	.gy	GYD	Dollar	592			en-GY	3378535	SR,BR,VE	
// HK	HKG	344	HK	Hong Kong	Hong Kong	1092	7491609	AS	.hk	HKD	Dollar	852			zh-HK,yue,zh,en	1819730		
// HM	HMD	334	HM	Heard Island and McDonald Islands		412	0	AN	.hm	AUD	Dollar	 				1547314		
// HN	HND	340	HO	Honduras	Tegucigalpa	112090	9587522	NA	.hn	HNL	Lempira	504	@@####	^([A-Z]{2}\d{4})$	es-HN,cab,miq	3608932	GT,NI,SV	
// HR	HRV	191	HR	Croatia	Zagreb	56542	3871833	EU	.hr	EUR	Euro	385	#####	^(?:HR)*(\d{5})$	hr-HR,sr	3202326	HU,SI,BA,ME,RS	
// HT	HTI	332	HA	Haiti	Port-au-Prince	27750	11123176	NA	.ht	HTG	Gourde	509	HT####	^(?:HT)*(\d{4})$	ht,fr-HT	3723988	DO	
// HU	HUN	348	HU	Hungary	Budapest	93030	9768785	EU	.hu	HUF	Forint	36	####	^(\d{4})$	hu-HU	719819	SK,SI,RO,UA,HR,AT,RS	
// ID	IDN	360	ID	Indonesia	Jakarta	1919440	267663435	AS	.id	IDR	Rupiah	62	#####	^(\d{5})$	id,en,nl,jv	1643084	PG,TL,MY	
// IE	IRL	372	EI	Ireland	Dublin	70280	4853506	EU	.ie	EUR	Euro	353	@@@ @@@@	^(D6W|[AC-FHKNPRTV-Y][0-9]{2})\s?([AC-FHKNPRTV-Y0-9]{4})	en-IE,ga-IE	2963597	GB	
// IL	ISR	376	IS	Israel	Jerusalem	20770	8883800	AS	.il	ILS	Shekel	972	#######	^(\d{7}|\d{5})$	he,ar-IL,en-IL,	294640	SY,JO,LB,EG,PS	
// IM	IMN	833	IM	Isle of Man	Douglas	572	84077	EU	.im	GBP	Pound	+44-1624	@# #@@|@## #@@|@@# #@@|@@## #@@|@#@ #@@|@@#@ #@@|GIR0AA	^((?:(?:[A-PR-UWYZ][A-HK-Y]\d[ABEHMNPRV-Y0-9]|[A-PR-UWYZ]\d[A-HJKPS-UW0-9])\s\d[ABD-HJLNP-UW-Z]{2})|GIR\s?0AA)$	en,gv	3042225		
// IN	IND	356	IN	India	New Delhi	3287590	1352617328	AS	.in	INR	Rupee	91	######	^(\d{6})$	en-IN,hi,bn,te,mr,ta,ur,gu,kn,ml,or,pa,as,bh,sat,ks,ne,sd,kok,doi,mni,sit,sa,fr,lus,inc	1269750	CN,NP,MM,BT,PK,BD	
// IO	IOT	086	IO	British Indian Ocean Territory	Diego Garcia	60	4000	AS	.io	USD	Dollar	246			en-IO	1282588		
// IQ	IRQ	368	IZ	Iraq	Baghdad	437072	38433600	AS	.iq	IQD	Dinar	964	#####	^(\d{5})$	ar-IQ,ku,hy	99237	SY,SA,IR,JO,TR,KW	
// IR	IRN	364	IR	Iran	Tehran	1648000	81800269	AS	.ir	IRR	Rial	98	##########	^(\d{10})$	fa-IR,ku	130758	TM,AF,IQ,AM,PK,AZ,TR	
// IS	ISL	352	IC	Iceland	Reykjavik	103000	353574	EU	.is	ISK	Krona	354	###	^(\d{3})$	is,en,de,da,sv,no	2629691		
// IT	ITA	380	IT	Italy	Rome	301230	60431283	EU	.it	EUR	Euro	39	#####	^(\d{5})$	it-IT,de-IT,fr-IT,sc,ca,co,sl	3175395	CH,VA,SI,SM,FR,AT	
// JE	JEY	832	JE	Jersey	Saint Helier	116	90812	EU	.je	GBP	Pound	+44-1534	@# #@@|@## #@@|@@# #@@|@@## #@@|@#@ #@@|@@#@ #@@|GIR0AA	^((?:(?:[A-PR-UWYZ][A-HK-Y]\d[ABEHMNPRV-Y0-9]|[A-PR-UWYZ]\d[A-HJKPS-UW0-9])\s\d[ABD-HJLNP-UW-Z]{2})|GIR\s?0AA)$	en,fr,nrf	3042142		
// JM	JAM	388	JM	Jamaica	Kingston	10991	2934855	NA	.jm	JMD	Dollar	+1-876			en-JM	3489940		
// JO	JOR	400	JO	Jordan	Amman	92300	9956011	AS	.jo	JOD	Dinar	962	#####	^(\d{5})$	ar-JO,en	248816	SY,SA,IQ,IL,PS	
// JP	JPN	392	JA	Japan	Tokyo	377835	126529100	AS	.jp	JPY	Yen	81	###-####	^\d{3}-\d{4}$	ja	1861060		
// KE	KEN	404	KE	Kenya	Nairobi	582650	51393010	AF	.ke	KES	Shilling	254	#####	^(\d{5})$	en-KE,sw-KE	192950	ET,TZ,SS,SO,UG	
// KG	KGZ	417	KG	Kyrgyzstan	Bishkek	198500	6315800	AS	.kg	KGS	Som	996	######	^(\d{6})$	ky,uz,ru	1527747	CN,TJ,UZ,KZ	
// KH	KHM	116	CB	Cambodia	Phnom Penh	181040	16249798	AS	.kh	KHR	Riels	855	#####	^(\d{5})$	km,fr,en	1831722	LA,TH,VN	
// KI	KIR	296	KR	Kiribati	Tarawa	811	115847	OC	.ki	AUD	Dollar	686			en-KI,gil	4030945		
// KM	COM	174	CN	Comoros	Moroni	2170	832322	AF	.km	KMF	Franc	269			ar,fr-KM	921929		
// KN	KNA	659	SC	Saint Kitts and Nevis	Basseterre	261	52441	NA	.kn	XCD	Dollar	+1-869			en-KN	3575174		
// KP	PRK	408	KN	North Korea	Pyongyang	120540	25549819	AS	.kp	KPW	Won	850	###-###	^(\d{6})$	ko-KP	1873107	CN,KR,RU	
// KR	KOR	410	KS	South Korea	Seoul	98480	51635256	AS	.kr	KRW	Won	82	#####	^(\d{5})$	ko-KR,en	1835841	KP	
// XK	XKX	0	KV	Kosovo	Pristina	10908	1845300	EU		EUR	Euro				sq,sr	831053	RS,AL,MK,ME	
// KW	KWT	414	KU	Kuwait	Kuwait City	17820	4137309	AS	.kw	KWD	Dinar	965	#####	^(\d{5})$	ar-KW,en	285570	SA,IQ	
// KY	CYM	136	CJ	Cayman Islands	George Town	262	64174	NA	.ky	KYD	Dollar	+1-345			en-KY	3580718		
// KZ	KAZ	398	KZ	Kazakhstan	Nur-Sultan	2717300	18276499	AS	.kz	KZT	Tenge	7	######	^(\d{6})$	kk,ru	1522867	TM,CN,KG,UZ,RU	
// LA	LAO	418	LA	Laos	Vientiane	236800	7061507	AS	.la	LAK	Kip	856	#####	^(\d{5})$	lo,fr,en	1655842	CN,MM,KH,TH,VN	
// LB	LBN	422	LE	Lebanon	Beirut	10400	6848925	AS	.lb	LBP	Pound	961	#### ####|####	^(\d{4}(\d{4})?)$	ar-LB,fr-LB,en,hy	272103	SY,IL	
// LC	LCA	662	ST	Saint Lucia	Castries	616	181889	NA	.lc	XCD	Dollar	+1-758			en-LC	3576468		
// LI	LIE	438	LS	Liechtenstein	Vaduz	160	37910	EU	.li	CHF	Franc	423	####	^(\d{4})$	de-LI	3042058	CH,AT	
// LK	LKA	144	CE	Sri Lanka	Colombo	65610	21670000	AS	.lk	LKR	Rupee	94	#####	^(\d{5})$	si,ta,en	1227603		
// LR	LBR	430	LI	Liberia	Monrovia	111370	4818977	AF	.lr	LRD	Dollar	231	####	^(\d{4})$	en-LR	2275384	SL,CI,GN	
// LS	LSO	426	LT	Lesotho	Maseru	30355	2108132	AF	.ls	LSL	Loti	266	###	^(\d{3})$	en-LS,st,zu,xh	932692	ZA	
// LT	LTU	440	LH	Lithuania	Vilnius	65200	2789533	EU	.lt	EUR	Euro	370	LT-#####	^(?:LT)*(\d{5})$	lt,ru,pl	597427	PL,BY,RU,LV	
// LU	LUX	442	LU	Luxembourg	Luxembourg	2586	607728	EU	.lu	EUR	Euro	352	L-####	^(?:L-)?\d{4}$	lb,de-LU,fr-LU	2960313	DE,BE,FR	
// LV	LVA	428	LG	Latvia	Riga	64589	1926542	EU	.lv	EUR	Euro	371	LV-####	^(?:LV)*(\d{4})$	lv,ru,lt	458258	LT,EE,BY,RU	
// LY	LBY	434	LY	Libya	Tripoli	1759540	6678567	AF	.ly	LYD	Dinar	218			ar-LY,it,en	2215636	TD,NE,DZ,SD,TN,EG	
// MA	MAR	504	MO	Morocco	Rabat	446550	36029138	AF	.ma	MAD	Dirham	212	#####	^(\d{5})$	ar-MA,ber,fr	2542007	DZ,EH,ES	
// MC	MCO	492	MN	Monaco	Monaco	1.95	38682	EU	.mc	EUR	Euro	377	#####	^(\d{5})$	fr-MC,en,it	2993457	FR	
// MD	MDA	498	MD	Moldova	Chisinau	33843	3545883	EU	.md	MDL	Leu	373	MD-####	^MD-\d{4}$	ro,ru,gag,tr	617790	RO,UA	
// ME	MNE	499	MJ	Montenegro	Podgorica	14026	622345	EU	.me	EUR	Euro	382	#####	^(\d{5})$	sr,hu,bs,sq,hr,rom	3194884	AL,HR,BA,RS,XK	
// MF	MAF	663	RN	Saint Martin	Marigot	53	37264	NA	.gp	EUR	Euro	590	#####	^(\d{5})$	fr	3578421	SX	
// MG	MDG	450	MA	Madagascar	Antananarivo	587040	26262368	AF	.mg	MGA	Ariary	261	###	^(\d{3})$	fr-MG,mg	1062947		
// MH	MHL	584	RM	Marshall Islands	Majuro	181.3	58413	OC	.mh	USD	Dollar	692	#####-####	^969\d{2}(-\d{4})$	mh,en-MH	2080185		
// MK	MKD	807	MK	North Macedonia	Skopje	25333	2082958	EU	.mk	MKD	Denar	389	####	^(\d{4})$	mk,sq,tr,rmm,sr	718075	AL,GR,BG,RS,XK	
// ML	MLI	466	ML	Mali	Bamako	1240000	19077690	AF	.ml	XOF	Franc	223			fr-ML,bm	2453866	SN,NE,DZ,CI,GN,MR,BF	
// MM	MMR	104	BM	Myanmar	Nay Pyi Taw	678500	53708395	AS	.mm	MMK	Kyat	95	#####	^(\d{5})$	my	1327865	CN,LA,TH,BD,IN	
// MN	MNG	496	MG	Mongolia	Ulaanbaatar	1565000	3170208	AS	.mn	MNT	Tugrik	976	######	^(\d{6})$	mn,ru	2029969	CN,RU	
// MO	MAC	446	MC	Macao	Macao	254	631636	AS	.mo	MOP	Pataca	853			zh,zh-MO,pt	1821275		
// MP	MNP	580	CQ	Northern Mariana Islands	Saipan	477	56882	OC	.mp	USD	Dollar	+1-670	#####	^9695\d{1}$	fil,tl,zh,ch-MP,en-MP	4041468		
// MQ	MTQ	474	MB	Martinique	Fort-de-France	1100	432900	NA	.mq	EUR	Euro	596	#####	^(\d{5})$	fr-MQ	3570311		
// MR	MRT	478	MR	Mauritania	Nouakchott	1030700	4403319	AF	.mr	MRU	Ouguiya	222			ar-MR,fuc,snk,fr,mey,wo	2378080	SN,DZ,EH,ML	
// MS	MSR	500	MH	Montserrat	Plymouth	102	9341	NA	.ms	XCD	Dollar	+1-664			en-MS	3578097		
// MT	MLT	470	MT	Malta	Valletta	316	483530	EU	.mt	EUR	Euro	356	@@@ ####	^[A-Z]{3}\s?\d{4}$	mt,en-MT	2562770		
// MU	MUS	480	MP	Mauritius	Port Louis	2040	1265303	AF	.mu	MUR	Rupee	230			en-MU,bho,fr	934292		
// MV	MDV	462	MV	Maldives	Male	300	515696	AS	.mv	MVR	Rufiyaa	960	#####	^(\d{5})$	dv,en	1282028		
// MW	MWI	454	MI	Malawi	Lilongwe	118480	17563749	AF	.mw	MWK	Kwacha	265	######	^(\d{6})$	ny,yao,tum,swk	927384	TZ,MZ,ZM	
// MX	MEX	484	MX	Mexico	Mexico City	1972550	126190788	NA	.mx	MXN	Peso	52	#####	^(\d{5})$	es-MX	3996063	GT,US,BZ	
// MY	MYS	458	MY	Malaysia	Kuala Lumpur	329750	31528585	AS	.my	MYR	Ringgit	60	#####	^(\d{5})$	ms-MY,en,zh,ta,te,ml,pa,th	1733045	BN,TH,ID	
// MZ	MOZ	508	MZ	Mozambique	Maputo	801590	29495962	AF	.mz	MZN	Metical	258	####	^(\d{4})$	pt-MZ,vmw	1036973	ZW,TZ,SZ,ZA,ZM,MW	
// NA	NAM	516	WA	Namibia	Windhoek	825418	2448255	AF	.na	NAD	Dollar	264			en-NA,af,de,hz,naq	3355338	ZA,BW,ZM,AO	
// NC	NCL	540	NC	New Caledonia	Noumea	19060	284060	OC	.nc	XPF	Franc	687	#####	^(\d{5})$	fr-NC	2139685		
// NE	NER	562	NG	Niger	Niamey	1267000	22442948	AF	.ne	XOF	Franc	227	####	^(\d{4})$	fr-NE,ha,kr,dje	2440476	TD,BJ,DZ,LY,BF,NG,ML	
// NF	NFK	574	NF	Norfolk Island	Kingston	34.6	1828	OC	.nf	AUD	Dollar	672	####	^(\d{4})$	en-NF	2155115		
// NG	NGA	566	NI	Nigeria	Abuja	923768	195874740	AF	.ng	NGN	Naira	234	######	^(\d{6})$	en-NG,ha,yo,ig,ff	2328926	TD,NE,BJ,CM	
// NI	NIC	558	NU	Nicaragua	Managua	129494	6465513	NA	.ni	NIO	Cordoba	505	###-###-#	^(\d{7})$	es-NI,en	3617476	CR,HN	
// NL	NLD	528	NL	The Netherlands	Amsterdam	41526	17231017	EU	.nl	EUR	Euro	31	#### @@	^(\d{4}\s?[a-zA-Z]{2})$	nl-NL,fy-NL	2750405	DE,BE	
// NO	NOR	578	NO	Norway	Oslo	324220	5314336	EU	.no	NOK	Krone	47	####	^(\d{4})$	no,nb,nn,se,fi	3144096	FI,RU,SE	
// NP	NPL	524	NP	Nepal	Kathmandu	140800	28087871	AS	.np	NPR	Rupee	977	#####	^(\d{5})$	ne,en	1282988	CN,IN	
// NR	NRU	520	NR	Nauru	Yaren	21	12704	OC	.nr	AUD	Dollar	674			na,en-NR	2110425		
// NU	NIU	570	NE	Niue	Alofi	260	2166	OC	.nu	NZD	Dollar	683			niu,en-NU	4036232		
// NZ	NZL	554	NZ	New Zealand	Wellington	268680	4885500	OC	.nz	NZD	Dollar	64	####	^(\d{4})$	en-NZ,mi	2186224		
// OM	OMN	512	MU	Oman	Muscat	212460	4829483	AS	.om	OMR	Rial	968	###	^(\d{3})$	ar-OM,en,bal,ur	286963	SA,YE,AE	
// PA	PAN	591	PM	Panama	Panama City	78200	4176873	NA	.pa	PAB	Balboa	507	#####	^(\d{5})$	es-PA,en	3703430	CR,CO	
// PE	PER	604	PE	Peru	Lima	1285220	31989256	SA	.pe	PEN	Sol	51	#####	^(\d{5})$	es-PE,qu,ay	3932488	EC,CL,BO,BR,CO	
// PF	PYF	258	FP	French Polynesia	Papeete	4167	277679	OC	.pf	XPF	Franc	689	#####	^((97|98)7\d{2})$	fr-PF,ty	4030656		
// PG	PNG	598	PP	Papua New Guinea	Port Moresby	462840	8606316	OC	.pg	PGK	Kina	675	###	^(\d{3})$	en-PG,ho,meu,tpi	2088628	ID	
// PH	PHL	608	RP	Philippines	Manila	300000	106651922	AS	.ph	PHP	Peso	63	####	^(\d{4})$	tl,en-PH,fil,ceb,ilo,hil,war,pam,bik,bcl,pag,mrw,tsg,mdh,cbk,krj,sgd,msb,akl,ibg,yka,mta,abx	1694008		
// PK	PAK	586	PK	Pakistan	Islamabad	803940	212215030	AS	.pk	PKR	Rupee	92	#####	^(\d{5})$	ur-PK,en-PK,pa,sd,ps,brh	1168579	CN,AF,IR,IN	
// PL	POL	616	PL	Poland	Warsaw	312685	37978548	EU	.pl	PLN	Zloty	48	##-###	^\d{2}-\d{3}$	pl	798544	DE,LT,SK,CZ,BY,UA,RU	
// PM	SPM	666	SB	Saint Pierre and Miquelon	Saint-Pierre	242	7012	NA	.pm	EUR	Euro	508	#####	^(97500)$	fr-PM	3424932		
// PN	PCN	612	PC	Pitcairn	Adamstown	47	46	OC	.pn	NZD	Dollar	870			en-PN	4030699		
// PR	PRI	630	RQ	Puerto Rico	San Juan	9104	3195153	NA	.pr	USD	Dollar	+1-787 and 1-939	#####-####	^00[679]\d{2}(?:-\d{4})?$	en-PR,es-PR	4566966		
// PS	PSE	275	WE	Palestinian Territory	East Jerusalem	5970	4569087	AS	.ps	ILS	Shekel	970			ar-PS	6254930	JO,IL,EG	
// PT	PRT	620	PO	Portugal	Lisbon	92391	10281762	EU	.pt	EUR	Euro	351	####-###	^\d{4}-\d{3}\s?[a-zA-Z]{0,25}$	pt-PT,mwl	2264397	ES	
// PW	PLW	585	PS	Palau	Melekeok	458	17907	OC	.pw	USD	Dollar	680	96940	^(96940)$	pau,sov,en-PW,tox,ja,fil,zh	1559582		
// PY	PRY	600	PA	Paraguay	Asuncion	406750	6956071	SA	.py	PYG	Guarani	595	####	^(\d{4})$	es-PY,gn	3437598	BO,BR,AR	
// QA	QAT	634	QA	Qatar	Doha	11437	2781677	AS	.qa	QAR	Rial	974			ar-QA,es	289688	SA	
// RE	REU	638	RE	Reunion	Saint-Denis	2517	776948	AF	.re	EUR	Euro	262	#####	^((97|98)(4|7|8)\d{2})$	fr-RE	935317		
// RO	ROU	642	RO	Romania	Bucharest	237500	19473936	EU	.ro	RON	Leu	40	######	^(\d{6})$	ro,hu,rom	798549	MD,HU,UA,BG,RS	
// RS	SRB	688	RI	Serbia	Belgrade	88361	6982084	EU	.rs	RSD	Dinar	381	######	^(\d{6})$	sr,hu,bs,rom	6290252	AL,HU,MK,RO,HR,BA,BG,ME,XK	
// RU	RUS	643	RS	Russia	Moscow	17100000	144478050	EU	.ru	RUB	Ruble	7	######	^(\d{6})$	ru,tt,xal,cau,ady,kv,ce,tyv,cv,udm,tut,mns,bua,myv,mdf,chm,ba,inh,kbd,krc,av,sah,nog	2017370	GE,CN,BY,UA,KZ,LV,PL,EE,LT,FI,MN,NO,AZ,KP	
// RW	RWA	646	RW	Rwanda	Kigali	26338	12301939	AF	.rw	RWF	Franc	250			rw,en-RW,fr-RW,sw	49518	TZ,CD,BI,UG	
// SA	SAU	682	SA	Saudi Arabia	Riyadh	1960582	33699947	AS	.sa	SAR	Rial	966	#####	^(\d{5})$	ar-SA	102358	QA,OM,IQ,YE,JO,AE,KW	
// SB	SLB	090	BP	Solomon Islands	Honiara	28450	652858	OC	.sb	SBD	Dollar	677			en-SB,tpi	2103350		
// SC	SYC	690	SE	Seychelles	Victoria	455	96762	AF	.sc	SCR	Rupee	248			en-SC,fr-SC	241170		
// SD	SDN	729	SU	Sudan	Khartoum	1861484	41801533	AF	.sd	SDG	Pound	249	#####	^(\d{5})$	ar-SD,en,fia	366755	SS,TD,EG,ET,ER,LY,CF	
// SS	SSD	728	OD	South Sudan	Juba	644329	8260490	AF	.ss	SSP	Pound	211			en	7909807	CD,CF,ET,KE,SD,UG	
// SE	SWE	752	SW	Sweden	Stockholm	449964	10183175	EU	.se	SEK	Krona	46	### ##	^(?:SE)?\d{3}\s\d{2}$	sv-SE,se,sma,fi-SE	2661886	NO,FI	
// SG	SGP	702	SN	Singapore	Singapore	692.7	5638676	AS	.sg	SGD	Dollar	65	######	^(\d{6})$	cmn,en-SG,ms-SG,ta-SG,zh-SG	1880251		
// SH	SHN	654	SH	Saint Helena	Jamestown	410	7460	AF	.sh	SHP	Pound	290	STHL 1ZZ	^(STHL1ZZ)$	en-SH	3370751		
// SI	SVN	705	SI	Slovenia	Ljubljana	20273	2067372	EU	.si	EUR	Euro	386	####	^(?:SI)*(\d{4})$	sl,sh	3190538	HU,IT,HR,AT	
// SJ	SJM	744	SV	Svalbard and Jan Mayen	Longyearbyen	62049	2550	EU	.sj	NOK	Krone	47	####	^(\d{4})$	no,ru	607072		
// SK	SVK	703	LO	Slovakia	Bratislava	48845	5447011	EU	.sk	EUR	Euro	421	### ##	^\d{3}\s?\d{2}$	sk,hu	3057568	PL,HU,CZ,UA,AT	
// SL	SLE	694	SL	Sierra Leone	Freetown	71740	7650154	AF	.sl	SLL	Leone	232			en-SL,men,tem	2403846	LR,GN	
// SM	SMR	674	SM	San Marino	San Marino	61.2	33785	EU	.sm	EUR	Euro	378	4789#	^(4789\d)$	it-SM	3168068	IT	
// SN	SEN	686	SG	Senegal	Dakar	196190	15854360	AF	.sn	XOF	Franc	221	#####	^(\d{5})$	fr-SN,wo,fuc,mnk	2245662	GN,MR,GW,GM,ML	
// SO	SOM	706	SO	Somalia	Mogadishu	637657	15008154	AF	.so	SOS	Shilling	252	@@  #####	^([A-Z]{2}\d{5})$	so-SO,ar-SO,it,en-SO	51537	ET,KE,DJ	
// SR	SUR	740	NS	Suriname	Paramaribo	163270	575991	SA	.sr	SRD	Dollar	597			nl-SR,en,srn,hns,jv	3382998	GY,BR,GF	
// ST	STP	678	TP	Sao Tome and Principe	Sao Tome	1001	197700	AF	.st	STN	Dobra	239			pt-ST	2410758		
// SV	SLV	222	ES	El Salvador	San Salvador	21040	6420744	NA	.sv	USD	Dollar	503	CP ####	^(?:CP)*(\d{4})$	es-SV	3585968	GT,HN	
// SX	SXM	534	NN	Sint Maarten	Philipsburg	21	40654	NA	.sx	ANG	Guilder	599			nl,en	7609695	MF	
// SY	SYR	760	SY	Syria	Damascus	185180	16906283	AS	.sy	SYP	Pound	963			ar-SY,ku,hy,arc,fr,en	163843	IQ,JO,IL,TR,LB	
// SZ	SWZ	748	WZ	Eswatini	Mbabane	17363	1136191	AF	.sz	SZL	Lilangeni	268	@###	^([A-Z]\d{3})$	en-SZ,ss-SZ	934841	ZA,MZ	
// TC	TCA	796	TK	Turks and Caicos Islands	Cockburn Town	430	37665	NA	.tc	USD	Dollar	+1-649	TKCA 1ZZ	^(TKCA 1ZZ)$	en-TC	3576916		
// TD	TCD	148	CD	Chad	N'Djamena	1284000	15477751	AF	.td	XAF	Franc	235			fr-TD,ar-TD,sre	2434508	NE,LY,CF,SD,CM,NG	
// TF	ATF	260	FS	French Southern Territories	Port-aux-Francais	7829	140	AN	.tf	EUR	Euro				fr	1546748		
// TG	TGO	768	TO	Togo	Lome	56785	7889094	AF	.tg	XOF	Franc	228			fr-TG,ee,hna,kbp,dag,ha	2363686	BJ,GH,BF	
// TH	THA	764	TH	Thailand	Bangkok	514000	69428524	AS	.th	THB	Baht	66	#####	^(\d{5})$	th,en	1605651	LA,MM,KH,MY	
// TJ	TJK	762	TI	Tajikistan	Dushanbe	143100	9100837	AS	.tj	TJS	Somoni	992	######	^(\d{6})$	tg,ru	1220409	CN,AF,KG,UZ	
// TK	TKL	772	TL	Tokelau		10	1466	OC	.tk	NZD	Dollar	690			tkl,en-TK	4031074		
// TL	TLS	626	TT	Timor Leste	Dili	15007	1267972	OC	.tl	USD	Dollar	670			tet,pt-TL,id,en	1966436	ID	
// TM	TKM	795	TX	Turkmenistan	Ashgabat	488100	5850908	AS	.tm	TMT	Manat	993	######	^(\d{6})$	tk,ru,uz	1218197	AF,IR,UZ,KZ	
// TN	TUN	788	TS	Tunisia	Tunis	163610	11565204	AF	.tn	TND	Dinar	216	####	^(\d{4})$	ar-TN,fr	2464461	DZ,LY	
// TO	TON	776	TN	Tonga	Nuku'alofa	748	103197	OC	.to	TOP	Pa'anga	676			to,en-TO	4032283		
// TR	TUR	792	TU	Turkey	Ankara	780580	82319724	AS	.tr	TRY	Lira	90	#####	^(\d{5})$	tr-TR,ku,diq,az,av	298795	SY,GE,IQ,IR,GR,AM,AZ,BG	
// TT	TTO	780	TD	Trinidad and Tobago	Port of Spain	5128	1389858	NA	.tt	TTD	Dollar	+1-868			en-TT,hns,fr,es,zh	3573591		
// TV	TUV	798	TV	Tuvalu	Funafuti	26	11508	OC	.tv	AUD	Dollar	688			tvl,en,sm,gil	2110297		
// TW	TWN	158	TW	Taiwan	Taipei	35980	23451837	AS	.tw	TWD	Dollar	886	#####	^(\d{5})$	zh-TW,zh,nan,hak	1668284		
// TZ	TZA	834	TZ	Tanzania	Dodoma	945087	56318348	AF	.tz	TZS	Shilling	255			sw-TZ,en,ar	149590	MZ,KE,CD,RW,ZM,BI,UG,MW	
// UA	UKR	804	UP	Ukraine	Kyiv	603700	44622516	EU	.ua	UAH	Hryvnia	380	#####	^(\d{5})$	uk,ru-UA,rom,pl,hu	690791	PL,MD,HU,SK,BY,RO,RU	
// UG	UGA	800	UG	Uganda	Kampala	236040	42723139	AF	.ug	UGX	Shilling	256			en-UG,lg,sw,ar	226074	TZ,KE,SS,CD,RW	
// UM	UMI	581		United States Minor Outlying Islands		0	0	OC	.um	USD	Dollar	1			en-UM	5854968		
// US	USA	840	US	United States	Washington	9629091	327167434	NA	.us	USD	Dollar	1	#####-####	^\d{5}(-\d{4})?$	en-US,es-US,haw,fr	6252001	CA,MX,CU	
// UY	URY	858	UY	Uruguay	Montevideo	176220	3449299	SA	.uy	UYU	Peso	598	#####	^(\d{5})$	es-UY	3439705	BR,AR	
// UZ	UZB	860	UZ	Uzbekistan	Tashkent	447400	32955400	AS	.uz	UZS	Som	998	######	^(\d{6})$	uz,ru,tg	1512440	TM,AF,KG,TJ,KZ	
// VA	VAT	336	VT	Vatican	Vatican City	0.44	921	EU	.va	EUR	Euro	379	#####	^(\d{5})$	la,it,fr	3164670	IT	
// VC	VCT	670	VC	Saint Vincent and the Grenadines	Kingstown	389	110211	NA	.vc	XCD	Dollar	+1-784			en-VC,fr	3577815		
// VE	VEN	862	VE	Venezuela	Caracas	912050	28870195	SA	.ve	VES	Bolivar Soberano	58	####	^(\d{4})$	es-VE	3625428	GY,BR,CO	
// VG	VGB	092	VI	British Virgin Islands	Road Town	153	29802	NA	.vg	USD	Dollar	+1-284			en-VG	3577718		
// VI	VIR	850	VQ	U.S. Virgin Islands	Charlotte Amalie	352	106977	NA	.vi	USD	Dollar	+1-340	#####-####	^008\d{2}(?:-\d{4})?$	en-VI	4796775		
// VN	VNM	704	VM	Vietnam	Hanoi	329560	95540395	AS	.vn	VND	Dong	84	######	^(\d{6})$	vi,en,fr,zh,km	1562822	CN,LA,KH	
// VU	VUT	548	NH	Vanuatu	Port Vila	12200	292680	OC	.vu	VUV	Vatu	678			bi,en-VU,fr-VU	2134431		
// WF	WLF	876	WF	Wallis and Futuna	Mata Utu	274	16025	OC	.wf	XPF	Franc	681	#####	^(986\d{2})$	wls,fud,fr-WF	4034749		
// WS	WSM	882	WS	Samoa	Apia	2944	196130	OC	.ws	WST	Tala	685			sm,en-WS	4034894		
// YE	YEM	887	YM	Yemen	Sanaa	527970	28498687	AS	.ye	YER	Rial	967			ar-YE	69543	SA,OM	
// YT	MYT	175	MF	Mayotte	Mamoudzou	374	279471	AF	.yt	EUR	Euro	262	#####	^(\d{5})$	fr-YT	1024031		
// ZA	ZAF	710	SF	South Africa	Pretoria	1219912	57779622	AF	.za	ZAR	Rand	27	####	^(\d{4})$	zu,xh,af,nso,en-ZA,tn,st,ts,ss,ve,nr	953987	ZW,SZ,MZ,BW,NA,LS	
// ZM	ZMB	894	ZA	Zambia	Lusaka	752614	17351822	AF	.zm	ZMW	Kwacha	260	#####	^(\d{5})$	en-ZM,bem,loz,lun,lue,ny,toi	895949	ZW,TZ,MZ,CD,NA,MW,AO	
// ZW	ZWE	716	ZI	Zimbabwe	Harare	390580	14439018	AF	.zw	ZWL	Dollar	263			en-ZW,sn,nr,nd	878675	ZA,MZ,BW,ZM	
// CS	SCG	891	YI	Serbia and Montenegro	Belgrade	102350	10829175	EU	.cs	RSD	Dinar	381	#####	^(\d{5})$	cu,hu,sq,sr	8505033	AL,HU,MK,RO,HR,BA,BG	
// AN	ANT	530	NT	Netherlands Antilles	Willemstad	960	300000	NA	.an	ANG	Guilder	599			nl-AN,en,es	8505032	GP	

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
        automated: get_automated_keyphrases(&region_map, "KH"),
        names: Some(vec!["cambodia".into()]),
        demonyms: Some(vec!["khmer".into()]),
        enterprises: None,
        misc: Some(vec!["funcinpec".into()]),
    }.get_region_vec(), "Cambodia".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "KM"),
        names: Some(vec!["comoros".into()]),
        demonyms: Some(vec!["comorian".into()]),
        enterprises: None,
        misc: Some(vec!["orange party".into(), "republican organization for the future of new generations".into()]),
    }.get_region_vec(), "Comoros".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "HR"),
        names: Some(vec!["croatia".into()]),
        demonyms: None,
        enterprises: None,
        misc: Some(vec!["sabor".into(), "banski dvori".into(), "hdz".into()]),
    }.get_region_vec(), "Croatia".into()));
    map.push((RegionKeyphrases {
        automated: get_automated_keyphrases(&region_map, "TD"),
        names: None,
        demonyms: Some(vec!["chadian".into()]),
        enterprises: None,
        misc: Some(vec!["national transitional council".into()]),
    }.get_region_vec(), "Chad".into()));

    remove_ambiguities(map, vec!["chad".into(), "georgia".into(), "jordan".into(), "turkey".into()].into_par_iter().collect()) //TODO: look at sqlite db for more
});