use serde::Serialize;
use std::cmp::Ordering;
use std::collections::HashMap;
use wasm_bindgen::prelude::*;
mod user_data;
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
    skill: f64,
}

impl Eq for CalcCard {}

impl Ord for CalcCard {
    fn cmp(&self, other: &Self) -> Ordering {
        let diff = if self.score > other.score {
            self.score - other.score
        } else {
            other.score - self.score
        };
        // If score is close, we can look at skill
        if diff <= 1000 {
            self.skill.partial_cmp(&other.skill).unwrap()
        } else {
            self.score.cmp(&other.score)
        }
    }
}

impl PartialOrd for CalcCard {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for CalcCard {
    fn eq(&self, other: &Self) -> bool {
        self.score == other.score && self.skill == other.skill
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
            for stat in info.iter() {
                card_data.performance += mul(stat.performance, level_percentage);
                card_data.technique += mul(stat.technique, level_percentage);
                card_data.visual += mul(stat.visual, level_percentage);
            }
        // Level bonus
        // TODO Use card stats
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
    let mut score = card_data.performance + card_data.technique + card_data.visual;
    score = (score as f64 * bonus) as u32;
    // Parameter bonus
    if has_event == 2 {
        score += (1.5
            * (match event_bonus.parameter.as_str() {
                "performance" => card_data.performance,
                "technique" => card_data.technique,
                "visual" => card_data.visual,
                _ => 0,
            }) as f64) as u32;
    }
    score
}

/// Use user profile and event bonus to calculate max score cardset
fn calc_max_score(
    cards: &HashMap<String, Card>,
    user_profile: &UserProfile,
    event_bonus: &EventBonus,
    character_band: &HashMap<u8, String>,
) -> HashMap<u8, CalcCard> {
    let card_skill = card_skill_new();
    let mut best_cardset: HashMap<u8, CalcCard> = HashMap::new();
    let mut best_score = 0;
    // Iterator props and bands to find best card set
    // Maybe greedy algorithm can boost it up?
    for (prop_name, prop_bonus) in user_profile.props.iter() {
        for (band_name, band_bonus) in user_profile.bands.iter() {
            let mut calc_cards: Vec<CalcCard> = Vec::new();
            for card_stat in user_profile.card_status.iter() {
                if card_stat.exclude {
                    continue;
                }
                let card = match cards.get(&card_stat.id.to_string()) {
                    Some(value) => value,
                    None => {
                        println!("Cannot find card {}", card_stat.id);
                        continue;
                    }
                };
                // If card doesn't release
                if card.released_at[user_profile.server as usize].is_null() {
                    continue;
                }
                calc_cards.push(CalcCard {
                    card_id: card_stat.id,
                    character_id: card.character_id,
                    score: calc_card_score(
                        card,
                        card_stat,
                        event_bonus,
                        character_band,
                        band_name,
                        band_bonus,
                        prop_name,
                        prop_bonus,
                    ),
                    skill: card_skill[&card.skill_id],
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
                    result_score += it.score;
                    result.insert(it.character_id, it);
                }
            }
            if result_score > best_score {
                best_score = result_score;
                best_cardset = result;
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
) -> JsValue {
    console_error_panic_hook::set_once();
    let event_bonus = event_bonus.into_serde().unwrap();
    let raw_user_profile = raw_user_profile.into_serde().unwrap();
    let character_band = character_band_new(
        characters.into_serde().unwrap(),
        bands.into_serde().unwrap(),
    );
    let all_cards: HashMap<String, Card> = cards.into_serde().unwrap();
    let user_profile = UserProfile::new(&raw_user_profile);
    JsValue::from_serde(&calc_max_score(
        &all_cards,
        &user_profile,
        &event_bonus,
        &character_band,
    ))
    .unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::prelude::*;

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
            character_band.insert(character_id, band.band_name[1].to_string());
        }
        Ok(character_band)
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
        // 只属于我们的SUMMER VACATION
        let event_bonus = EventBonus {
            prop: String::from("happy"),
            characters: vec![16, 17, 18, 19, 20],
            prop_bonus: 0.1,
            character_bonus: 0.2,
            parameter: String::from("technique"),
            all_fit_bonus: 0.0,
        };
        let result = calc_max_score(&all_cards, &user_profile, &event_bonus, &character_band);
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
}
