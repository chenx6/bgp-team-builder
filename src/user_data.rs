use serde::Deserialize;
use serde_json::Value;

use std::collections::HashMap;

/// Raw user profile from bestdori
#[derive(Deserialize)]
pub struct RawUserProfile {
    name: String,
    server: u8,
    compression: String,
    data: String,
    items: HashMap<String, Vec<u8>>,
}

/// Number of the card data, include performance, technique, visual
#[derive(Deserialize)]
pub struct CardData {
    pub performance: u32,
    pub technique: u32,
    pub visual: u32,
}

/// Card base data
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Card {
    pub character_id: u8,
    pub rarity: u8,
    pub attribute: String,
    pub level_limit: u8,
    pub resource_set_name: String,
    pub prefix: Vec<Value>,
    pub released_at: Vec<Value>,
    pub skill_id: u8,
    #[serde(rename = "type")]
    pub type_: String,
    pub stat: HashMap<String, Value>, // TODO Find a way to type it
}

/// Character data
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Character {
    pub character_name: Value,
    pub first_name: Value,
    pub last_name: Value,
    pub nickname: Value,
    pub band_id: u8,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Band {
    pub band_name: Value,
}

pub struct Magazine {
    pub performance: f64,
    pub technique: f64,
    pub visual: f64,
}

/// This library's own user profile
pub struct UserProfile {
    pub name: String,
    pub server: u8,
    /// Band items, such as gtitar
    pub bands: HashMap<String, Vec<f64>>,
    /// Property items, such as food
    pub props: HashMap<String, Vec<f64>>,
    /// Magazine
    pub magazine: Magazine,
    /// Card status
    pub card_status: Vec<CardStatus>,
}

/// Event bonus
#[derive(Deserialize)]
pub struct EventBonus {
    /// Property, such as happy, cool
    pub prop: String,
    /// Character ids
    pub characters: Vec<u8>,
    /// Property bonus
    pub prop_bonus: f64,
    /// Character bonus
    pub character_bonus: f64,
    /// Parameter, such as performance, technique
    pub parameter: String,
    /// All fit parameter bonus
    pub all_fit_bonus: f64,
}

/// Card status from Bestdori's encode data
pub struct CardStatus {
    /// Card id
    pub id: u32,
    /// Level of the card
    pub level: u8,
    /// Card is exclude or not
    pub exclude: bool,
    /// Art picture
    pub art: u8,
    /// Trained or not
    pub train: u8,
    /// Episode unlock count
    pub ep: u8,
    /// Skill level
    pub skill: u8,
}

#[derive(Deserialize)]
pub struct SongNote {
    pub time: f64,
    pub fever: Option<bool>,
    pub skill: Option<bool>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActivateEffectType {
    pub activate_effect_value: Vec<Value>,
    pub activate_effect_value_type: String,
    pub activate_condition: String
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActivationEffect {
    pub unification_activate_effect_value: Option<u32>,
    pub unification_activate_condition_band_id: Option<u32>,
    pub activate_effect_types: HashMap<String, ActivateEffectType>
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Skill {
    pub duration: Vec<f64>,
    pub activation_effect: ActivationEffect,
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

fn b(t: &str) -> u32 {
    let mut n: u32 = 0;
    let base: u32 = 64;
    let table: &'static str = "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz-_";
    for (cnt, ch) in t.chars().enumerate() {
        n += match table.find(ch) {
            Some(value) => value as u32,
            None => {
                println!("Invalid input!");
                return 0;
            }
        } * base.pow((t.len() - cnt - 1) as u32);
    }
    n
}

pub fn decode_data(encrypted: &String) -> Vec<CardStatus> {
    let mut lst: Vec<CardStatus> = Vec::new();
    let mut n = 0;
    while n < encrypted.len() {
        let id = b(&encrypted[n..n + 2]);
        n += 2;
        let level = b(&encrypted[n..n + 1]) as u8;
        n += 1;
        let mut i = b(&encrypted[n..n + 2]);
        n += 2;
        let exclude = i % 2 == 1;
        i = (i as f64 / 2.0).floor() as u32;
        let art = (i % 2) as u8;
        i = (i as f64 / 2.0).floor() as u32;
        let train = (i % 2) as u8;
        i = (i as f64 / 2.0).floor() as u32;
        let ep = (i % 3) as u8;
        i = (i as f64 / 3.0).floor() as u32;
        let skill = i as u8;
        lst.push(CardStatus {
            id,
            level,
            exclude,
            art,
            train,
            ep,
            skill,
        })
    }
    lst
}

impl UserProfile {
    pub fn new(raw: &RawUserProfile) -> UserProfile {
        let mut bands: HashMap<String, Vec<f64>> = HashMap::new();
        let band_item_percentage = |v: &u8| *v as f64 / 100.0;
        let band_rename = [
            ("Afterglow", "Afterglow"),
            ("Everyone", "Everyone"),
            ("Hello, Happy World!", "HelloHappyWorld"),
            ("Pastel＊Palettes", "PastelPalettes"),
            ("Poppin'Party", "PoppinParty"),
            ("Roselia", "Roselia"),
        ];
        for i in band_rename.iter() {
            bands.insert(
                String::from(i.0),
                raw.items[i.1].iter().map(band_item_percentage).collect(),
            );
        }
        // bands.insert(String::from("RAISE A SUILEN"), map_to_percentage(raw.items["RAISE A SUILEN"]));
        // bands.insert(String::from("Morfonica"), map_to_percentage(raw.items["Morfonica"]));
        let item_percentage = |v: &u8| *v as f64 / 100.0;
        let mut props: HashMap<String, Vec<f64>> = HashMap::new();
        let menu: Vec<f64> = raw.items["Menu"].iter().map(item_percentage).collect();
        let plaza: Vec<f64> = raw.items["Plaza"].iter().map(item_percentage).collect();
        for (i, attr) in ["powerful", "cool", "happy", "pure"].iter().enumerate() {
            props.insert(String::from(attr.clone()), vec![menu[i], plaza[i]]);
        }
        let magazine_percentage = |v: &u8| match v {
            0 => 0.0,
            _ => *v as f64 * 0.02 + 0.06,
        };
        let magazine: Vec<f64> = raw.items["Magazine"]
            .iter()
            .map(magazine_percentage)
            .collect();
        let magazine = Magazine {
            performance: magazine[0],
            technique: magazine[1],
            visual: magazine[2],
        };
        let card_status = decode_data(&raw.data);
        UserProfile {
            name: raw.name.clone(),
            server: raw.server,
            bands,
            props,
            magazine,
            card_status,
        }
    }
}

/// Generate character and band relation
pub fn character_band_new(
    characters: HashMap<String, Character>,
    bands: HashMap<String, Band>,
) -> HashMap<u8, String> {
    let mut character_band: HashMap<u8, String> = HashMap::new();
    for (character_id, character) in characters.iter() {
        let character_id = character_id.parse::<u8>().unwrap();
        let band = bands.get(&character.band_id.to_string()).unwrap();
        character_band.insert(character_id, band.band_name[1].to_string());
    }
    character_band
}

/// Generate card skill bonus score
/// TODO: Use JSONED data
pub fn card_skill_new() -> HashMap<u8, f64> {
    [
        (1, 1.1),
        (2, 1.3),
        (3, 1.6),
        (4, 2.0),
        (5, 1.1),
        (6, 1.2),
        (7, 1.4),
        (8, 1.1),
        (9, 1.2),
        (10, 1.4),
        (11, 1.3),
        (12, 1.6),
        (13, 1.3),
        (14, 1.6),
        (15, 1.0),
        (16, 1.0),
        (17, 1.65),
        (18, 2.1),
        (20, 2.15),
        (21, 1.4),
        (22, 1.8),
        (23, 1.1),
        (24, 1.3),
        (25, 1.65),
        (26, 2.1),
    ]
    .iter()
    .cloned()
    .collect()
}

pub fn get_level_score(curr_level: u8, rarity: u8) -> f64 {
    let r1 = [
        0.0,
        0.027741577148418566,
        0.05827079766928518,
        0.09157023784727926,
        0.12763157919247634,
        0.16646335421245123,
        0.2080467329810958,
        0.2523725635815978,
        0.29945970809515476,
        0.3493549269647502,
        0.4019829891560635,
        0.45737145350963615,
        0.5155589441139967,
        0.5765070194790465,
        0.6401879689268755,
        0.7066399177484718,
        0.77583438444173,
        0.8477702087090347,
        0.922495295719485,
        1.0,
    ];
    let r2 = [
        0.0,
        0.017856719467337606,
        0.036896819500198366,
        0.05712720353262898,
        0.0785481410852151,
        0.10115068684022054,
        0.12493367020010375,
        0.14991873595264252,
        0.17609001362625878,
        0.20343906942389595,
        0.23198087664087155,
        0.26171208783900635,
        0.2926379539012518,
        0.324741136882865,
        0.3580344489450513,
        0.3925231410698117,
        0.4281932552740394,
        0.46505212999997836,
        0.5031024182461973,
        0.5423349324988558,
        0.5827566752115649,
        0.6243699948827242,
        0.6671642798084607,
        0.7111523418284023,
        0.7563202420759938,
        0.802684231316528,
        0.8502340870909751,
        0.8989700203928614,
        0.9488922676005076,
        1.0,
    ];

    let r3 = [
        0.0,
        0.006579811100923505,
        0.013490376941789076,
        0.02072357033736649,
        0.0282857469218734,
        0.036174782566627796,
        0.044399557457405224,
        0.0529520377427399,
        0.06182744102603252,
        0.07104081341142438,
        0.0805784925749624,
        0.09044396348716642,
        0.1006360985970335,
        0.11115826738840272,
        0.12201077801526514,
        0.1331914612552324,
        0.14469696632024545,
        0.1565321016810991,
        0.16870064899154105,
        0.18119441263664268,
        0.19401618052690653,
        0.20716987792842392,
        0.220646377954231,
        0.23445670502582086,
        0.24859157352745323,
        0.26305982052078125,
        0.2778572809267125,
        0.29297495807935886,
        0.30842764497977343,
        0.3242045972904203,
        0.3403108556813293,
        0.3567490256479929,
        0.37351842656938367,
        0.3906106859938102,
        0.4080321560667454,
        0.4257874104768857,
        0.4438663146273812,
        0.4622730945351113,
        0.48101099472751,
        0.5000713733964425,
        0.5277508312304107,
        0.5603880999928903,
        0.5979841991476733,
        0.6405413574510139,
        0.6880557418260439,
        0.7405268767404022,
        0.7979648036251932,
        0.8603540332444733,
        0.9276952510130176,
        1.0,
    ];
    let r4 = [
        0.0,
        0.0052137940761305835,
        0.010626772033633004,
        0.01625140619739073,
        0.02208300765878985,
        0.028130123879720123,
        0.03438591969492338,
        0.040845165089679725,
        0.04751431401768039,
        0.05438920952360014,
        0.06146926738626297,
        0.06876102000541633,
        0.07625624123106317,
        0.08396613839157453,
        0.09187956441780568,
        0.1000002838288349,
        0.1083369509623417,
        0.11687849700242568,
        0.12562643419463135,
        0.13458349890179105,
        0.14375149230097023,
        0.1531220343111895,
        0.1627012369922088,
        0.1724891364692424,
        0.18248307296777722,
        0.19268953004894984,
        0.2031038118277759,
        0.2137266946705706,
        0.22455873405686588,
        0.23560016086505522,
        0.246845117069297,
        0.2583022752066268,
        0.26996585295356473,
        0.2818368100569856,
        0.29391689352667993,
        0.3062034732003648,
        0.31870850250918314,
        0.3314136656714306,
        0.34432706913752,
        0.35744699511750455,
        0.37077464809342076,
        0.3843126970348029,
        0.39805844303293897,
        0.41201000405281624,
        0.4261726805173379,
        0.440542342723313,
        0.4551202726012628,
        0.46990761662317243,
        0.48490456025578976,
        0.5001066997170803,
        0.5277872974717714,
        0.5604260864848988,
        0.5980193982337021,
        0.640573453684307,
        0.6880874211513187,
        0.7405574835992293,
        0.7979798883017865,
        0.8603639584557542,
        0.927706300412736,
        1.0,
    ];
    match rarity {
        1 => r1[(curr_level - 1) as usize],
        2 => r2[(curr_level - 1) as usize],
        3 => r3[(curr_level - 1) as usize],
        4 => r4[(curr_level - 1) as usize],
        _ => 1.0
    }
}
