use serde::Serialize;
use std::collections::HashMap;
use std::{cmp::Ordering, collections::HashSet};
use wasm_bindgen::prelude::*;
mod song_calculate;
mod user_data;
use song_calculate::*;
use user_data::*;

#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

#[wasm_bindgen]
pub fn init_panic_hook() {
    console_error_panic_hook::set_once();
}

/// Score calculate result
#[derive(Copy, Clone, Serialize)]
pub struct CalcCard {
    card_id: u32,
    character_id: u8,
    score: u32,
    skill_id: u8,
    skill_mul: f64,
}

impl Eq for CalcCard {}

impl Ord for CalcCard {
    fn cmp(&self, other: &Self) -> Ordering {
        let score = self.score as f64 * self.skill_mul;
        let other_score = other.score as f64 * other.skill_mul;
        score.partial_cmp(&other_score).unwrap_or(Ordering::Equal)
    }
}

impl PartialOrd for CalcCard {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for CalcCard {
    fn eq(&self, other: &Self) -> bool {
        self.score == other.score && self.skill_mul == other.skill_mul
    }
}

/// Multiply wrapper for u32 and f64
fn mul(n1: u32, n2: f64) -> u32 {
    (n1 as f64 * n2) as u32
}

/// Calculate single card's score
fn calc_card_score(
    card: &Card,
    card_stat: &CardStatus,
    event_bonus: &EventBonus,
    character_band: &HashMap<u8, String>,
    magazine_name: &String,
    magazine: &f64,
    band_name: &String,
    band_bonus: &Vec<f64>,
    prop_name: &String,
    prop_bonus: &Vec<f64>,
) -> u32 {
    let mut card_data = CardData {
        performance: 0,
        technique: 0,
        visual: 0,
    };
    let mut bonus = 0.0;
    let level_percentage = get_level_score(card_stat.level, card.rarity);
    // Card stat related
    for (rank, info) in card.stat.iter() {
        // Episode bonus score
        if rank == "episodes" {
            let info: Vec<CardData> = serde_json::from_value(info.clone()).unwrap();
            for (idx, stat) in info.iter().enumerate() {
                if idx >= card_stat.ep as usize {
                    break;
                }
                card_data.performance += stat.performance;
                card_data.technique += stat.technique;
                card_data.visual += stat.visual;
            }
        // Level bonus
        } else if rank != "1" {
            let info: CardData = serde_json::from_value(info.clone()).unwrap();
            card_data.performance += mul(info.performance, level_percentage);
            card_data.technique += mul(info.technique, level_percentage);
            card_data.visual += mul(info.visual, level_percentage);
        }
    }
    let mut has_event = 0;
    // Character related
    if event_bonus.characters.contains(&card.character_id) {
        bonus += event_bonus.character_bonus;
        has_event += 1;
    }
    // Band related
    if character_band.get(&card.character_id).unwrap() == band_name {
        bonus += band_bonus.iter().sum::<f64>();
    }
    // Attribute related
    // Event attribute
    if event_bonus.prop == card.attribute {
        bonus += event_bonus.prop_bonus;
        has_event += 1;
    }
    // Properity attribute
    if &card.attribute == prop_name {
        bonus += prop_bonus.iter().sum::<f64>();
    }
    // All fit bonus
    if has_event == 2 {
        bonus += event_bonus.all_fit_bonus;
    }
    // All bonus sum up
    let mut score: f64 = (card_data.performance + card_data.technique + card_data.visual) as f64;
    score *= bonus + 1.0;
    // Parameter bonus
    if has_event == 2 {
        score += 0.5
            * match event_bonus.parameter.as_str() {
                "performance" => card_data.performance,
                "technique" => card_data.technique,
                "visual" => card_data.visual,
                _ => 0,
            } as f64;
    }
    score += magazine
        * match magazine_name.as_str() {
            "performance" => card_data.performance,
            "technique" => card_data.technique,
            "visual" => card_data.visual,
            _ => 0,
        } as f64;
    score as u32
}

/// Use user profile and event bonus to calculate max score cardset
fn calc_max_score(
    cards: &HashMap<String, Card>,
    user_profile: &UserProfile,
    event_bonus: &EventBonus,
    character_band: &HashMap<u8, String>,
    song_data: &Vec<SongNote>,
    skills: &HashMap<String, Skill>,
) -> HashMap<u8, CalcCard> {
    let mut best_cardset: HashMap<u8, CalcCard> = HashMap::new();
    let mut best_score = 0;
    let mut magazines: HashMap<String, f64> = HashMap::new();
    magazines.insert(
        String::from("performance"),
        user_profile.magazine.performance,
    );
    magazines.insert(String::from("technique"), user_profile.magazine.technique);
    magazines.insert(String::from("visual"), user_profile.magazine.visual);
    // Cache skill mul table
    let mut skill_set: HashSet<u32> = HashSet::new();
    for card_stat in user_profile.card_status.iter() {
        let card = cards.get(&card_stat.id.to_string()).unwrap_or(&cards[&1.to_string()]);
        let tag = card.skill_id as u32 * 10 + card_stat.skill as u32;
        skill_set.insert(tag);
    }
    let calc_skills: Vec<u32> = skill_set.into_iter().collect();
    let cache_table = cache_table(&calc_skills, &skills, song_data, 26, 0.97, false);
    // Iterator props and bands to find best card set
    // Maybe greedy algorithm can boost it up?
    for (prop_name, prop_bonus) in user_profile.props.iter() {
        for (band_name, band_bonus) in user_profile.bands.iter() {
            for (magazine_name, magazine_bonus) in magazines.iter() {
                let mut calc_cards: Vec<CalcCard> = Vec::new();
                for card_stat in user_profile.card_status.iter() {
                    if card_stat.exclude {
                        continue;
                    }
                    let card = cards.get(&card_stat.id.to_string()).unwrap_or(&cards[&1.to_string()]);
                    // If card doesn't release
                    if card.released_at[user_profile.server as usize].is_null() {
                        continue;
                    }
                    let skill_tag = card.skill_id as u32 * 10 + card_stat.skill as u32;
                    let skill_mul = cache_table[&skill_tag][&skill_tag] / 6.0;
                    calc_cards.push(CalcCard {
                        card_id: card_stat.id,
                        character_id: card.character_id,
                        score: calc_card_score(
                            card,
                            card_stat,
                            event_bonus,
                            character_band,
                            magazine_name,
                            magazine_bonus,
                            band_name,
                            band_bonus,
                            prop_name,
                            prop_bonus,
                        ),
                        skill_id: card.skill_id,
                        skill_mul,
                    });
                }
                // Sort by score
                calc_cards.sort_by(|a, b| b.cmp(a));
                // Calculate score
                let mut result: HashMap<u8, CalcCard> = HashMap::new();
                let mut result_score = 0;
                for it in calc_cards {
                    if result.len() >= 5 {
                        break;
                    }
                    if !result.contains_key(&it.character_id) {
                        result_score += (it.score as f64 * it.skill_mul) as u32;
                        result.insert(it.character_id, it);
                    }
                }
                if result_score > best_score {
                    best_score = result_score;
                    best_cardset = result;
                }
            }
        }
    }
    best_cardset
}

/// Use JS side data to build team that can get best score
#[wasm_bindgen]
pub fn gene_score(
    event_bonus: &JsValue,
    cards: &JsValue,
    raw_user_profile: &JsValue,
    characters: &JsValue,
    bands: &JsValue,
    song_data: &JsValue,
    skills: &JsValue,
) -> JsValue {
    console_error_panic_hook::set_once();
    let event_bonus = event_bonus.into_serde().unwrap();
    let raw_user_profile = raw_user_profile.into_serde().unwrap();
    let character_band = character_band_new(
        characters.into_serde().unwrap(),
        bands.into_serde().unwrap(),
    );
    let all_cards: HashMap<String, Card> = cards.into_serde().unwrap();
    let song_data = song_data.into_serde().unwrap();
    let user_profile = UserProfile::new(&raw_user_profile);
    let skills: HashMap<String, Skill> = skills.into_serde().unwrap();
    JsValue::from_serde(&calc_max_score(
        &all_cards,
        &user_profile,
        &event_bonus,
        &character_band,
        &song_data,
        &skills,
    ))
    .unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::to_string;
    use std::fs::File;
    use std::io::prelude::*;

    impl CardStatus {
        fn new(
            id: u32,
            level: u8,
            exclude: bool,
            art: u8,
            train: u8,
            ep: u8,
            skill: u8,
        ) -> CardStatus {
            CardStatus {
                id,
                level,
                exclude,
                art,
                train,
                ep,
                skill,
            }
        }
    }

    fn read_to_str(path: String) -> Result<String, std::io::Error> {
        let mut file = File::open(path)?;
        let mut buffer = String::new();
        file.read_to_string(&mut buffer)?;
        Ok(buffer)
    }

    fn read_cards(path: String) -> Result<HashMap<String, Card>, Box<dyn std::error::Error>> {
        let buffer = read_to_str(path)?;
        Ok(serde_json::from_str(buffer.as_str())?)
    }

    fn read_characters(
        path: String,
    ) -> Result<HashMap<String, Character>, Box<dyn std::error::Error>> {
        let buffer = read_to_str(path)?;
        Ok(serde_json::from_str(buffer.as_str())?)
    }

    fn read_bands(path: String) -> Result<HashMap<String, Band>, Box<dyn std::error::Error>> {
        let buffer = read_to_str(path)?;
        Ok(serde_json::from_str(buffer.as_str())?)
    }

    fn read_raw_user_profile(path: String) -> Result<RawUserProfile, Box<dyn std::error::Error>> {
        let buffer = read_to_str(path)?;
        Ok(serde_json::from_str(buffer.as_str())?)
    }

    fn character_band_new_from_string(
        characters_path: String,
        bands_path: String,
    ) -> Result<HashMap<u8, String>, Box<dyn std::error::Error>> {
        let mut character_band: HashMap<u8, String> = HashMap::new();
        let characters = read_characters(characters_path)?;
        let bands = read_bands(bands_path)?;
        for (character_id, character) in characters.iter() {
            let character_id = character_id.parse::<u8>().unwrap_or(1);
            let band = bands.get(&character.band_id.to_string()).unwrap();
            character_band.insert(
                character_id,
                band.band_name[1].as_str().unwrap().to_string(),
            );
        }
        Ok(character_band)
    }

    fn read_song_notes(path: String) -> Result<Vec<SongNote>, Box<dyn std::error::Error>> {
        let buffer = read_to_str(path)?;
        Ok(serde_json::from_str(buffer.as_str())?)
    }

    fn read_skill(path: String) -> Result<HashMap<String, Skill>, Box<dyn std::error::Error>> {
        let buffer = read_to_str(path)?;
        Ok(serde_json::from_str(buffer.as_str())?)
    }

    #[test]
    fn calc_test() {
        let cards_path = String::from("docs/cards.json");
        let raw_user_profile_path = String::from("docs/user_profile.json");
        let characters_path = String::from("docs/characters.json");
        let bands_path = String::from("docs/bands.json");
        let raw_user_profile = read_raw_user_profile(raw_user_profile_path).unwrap();
        let character_band = character_band_new_from_string(characters_path, bands_path).unwrap();
        let all_cards: HashMap<String, Card> = read_cards(cards_path).unwrap();
        let user_profile = UserProfile::new(&raw_user_profile);
        let song_notes = read_song_notes(String::from("docs/125.expert.json")).unwrap();
        let skills = read_skill(String::from("docs/skills.json")).unwrap();
        // 只属于我们的SUMMER VACATION
        let event_bonus = EventBonus {
            prop: String::from("happy"),
            characters: vec![16, 17, 18, 19, 20],
            prop_bonus: 0.1,
            character_bonus: 0.2,
            parameter: String::from("technique"),
            all_fit_bonus: 0.0,
        };
        let result = calc_max_score(
            &all_cards,
            &user_profile,
            &event_bonus,
            &character_band,
            &song_notes,
            &skills,
        );
        for (k, v) in result.iter() {
            println!(
                "{} {}",
                k,
                all_cards[&v.card_id.to_string()].prefix[3]
                    .as_str()
                    .unwrap()
            );
        }
        assert_eq!(result.len(), 5, "Calculation failed!")
    }

    #[test]
    fn score_test() {
        let cards_path = String::from("docs/cards.json");
        let characters_path = String::from("docs/characters.json");
        let bands_path = String::from("docs/bands.json");
        let character_band = character_band_new_from_string(characters_path, bands_path).unwrap();
        let all_cards: HashMap<String, Card> = read_cards(cards_path).unwrap();

        let band_name = String::from("Hello, Happy World!");
        let band_bonus = vec![0.04, 0.04, 0.04, 0.04, 0.04, 0.1, 0.1];
        let prop_name = String::from("pure");
        let prop_bonus = vec![0.1, 0.1];
        let card_status = vec![
            CardStatus::new(683, 50, false, 1, 1, 1, 0),
            CardStatus::new(466, 50, false, 1, 1, 1, 0),
            CardStatus::new(588, 60, false, 1, 1, 1, 0),
            CardStatus::new(589, 50, false, 1, 1, 1, 0),
            CardStatus::new(382, 50, false, 1, 1, 1, 0),
        ];
        let event_bonus = EventBonus {
            prop: String::from("pure"),
            characters: vec![11, 12, 13, 14, 15],
            prop_bonus: 0.1,
            character_bonus: 0.2,
            parameter: String::from("technique"),
            all_fit_bonus: 0.2,
        };
        let magazine = Magazine {
            performance: 0.16,
            technique: 0.16,
            visual: 0.16,
        };
        let mut final_score = 0;
        for card_stat in card_status.iter() {
            let card = all_cards.get(&card_stat.id.to_string()).unwrap();
            let curr_score = calc_card_score(
                &card,
                &card_stat,
                &event_bonus,
                &character_band,
                &String::from("performance"),
                &magazine.performance,
                &band_name,
                &band_bonus,
                &prop_name,
                &prop_bonus,
            );
            final_score += curr_score;
        }
        // TODO: Use f64 to sum up card score
        let game_score = 314763;
        assert!((game_score - 5..game_score + 5).contains(&final_score));
    }

    #[test]
    fn song_test() {
        // A to Z
        let song_notes = read_song_notes(String::from("docs/125.expert.json")).unwrap();
        let skills = read_skill(String::from("docs/skills.json")).unwrap();
        // 圣诞老人要来我家
        // 梦幻的抽鬼牌
        let calc_card = CalcCard {
            card_id: 588,
            character_id: 12,
            score: 53505,
            skill_id: 4,
            skill_mul: 0.5,
        };
        // 极其梦幻的生物
        let calc_card2 = CalcCard {
            card_id: 298,
            character_id: 12,
            score: 63880,
            skill_id: 13,
            skill_mul: 0.5,
        };
        let score1 = song_score(
            &vec![calc_card.skill_id; 6],
            &vec![0; 6],
            26,
            false,
            0.97,
            &song_notes,
            &skills,
        );
        let score2 = song_score(
            &vec![calc_card2.skill_id; 6],
            &vec![0; 6],
            26,
            false,
            0.97,
            &song_notes,
            &skills,
        );
        println!("{} {}", score1, score2);
        assert!(score1 > score2, "{} {}", score1, score2);
    }

    #[test]
    fn skill_test() {
        let skills = read_skill(String::from("docs/skills.json")).unwrap();
        let tags: Vec<u32> = vec![
            120, 180, 40, 140, 70, 200, 100, 260, 90, 60, 30, 110, 130, 170, 250, 240,
        ];
        let song_notes = read_song_notes(String::from("docs/125.expert.json")).unwrap();
        let table = cache_table(&tags, &skills, &song_notes, 26, 0.97, false);
        println!("{}", to_string(&table).unwrap());
    }
}
