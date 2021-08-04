use std::collections::HashMap;

use crate::user_data::{Skill, SongNote};

/// Calculate combo bonus
fn combo_bonus(combon: u32) -> f64 {
    match combon {
        0..=20 => 1.0,
        21..=50 => 1.01,
        51..=100 => 1.02,
        101..=150 => 1.03,
        151..=200 => 1.04,
        201..=250 => 1.05,
        251..=300 => 1.06,
        301..=400 => 1.07,
        401..=500 => 1.08,
        501..=600 => 1.09,
        601..=700 => 1.1,
        _ => 1.11,
    }
}

/// Calculate skill bonus
/// TODO: rename single character variable
pub fn skill_bonus(skill: &Skill, accurate: f64, index: u32) -> f64 {
    let mut a = 0.0;
    let mut r = 0.0;
    let mut s = 0.0;
    let mut m = true;
    for (k, v) in skill.activation_effect.activate_effect_types.iter() {
        match k.as_str() {
            "score" | "score_over_life" | "score_under_life" | "score_continued_note_judge" => {
                let effect_value = if m
                    && skill
                        .activation_effect
                        .unification_activate_effect_value
                        .is_some()
                {
                    skill
                        .activation_effect
                        .unification_activate_effect_value
                        .unwrap()
                } else {
                    v.activate_effect_value[0].as_u64().unwrap_or(0) as u32
                };
                if k == "score_continued_note_judge" {
                    a = 1.0 + effect_value as f64 / 100.0;
                } else if v.activate_condition == "perfect" {
                    if r == 0.0 {
                        r = effect_value as f64 / 100.0;
                    }
                } else {
                    if r == 0.0 {
                        r = effect_value as f64 / 100.0;
                    }
                    if s == 0.0 {
                        s = effect_value as f64 / 100.0;
                    }
                }
                m = false;
            }
            _ => {}
        }
    }
    r += 1.0;
    s += 1.0;
    let o = 1.1 * accurate + 0.8 * (1.0 - accurate);
    if a != 0.0 {
        s + accurate.powi(index as i32) * (a - s)
    } else {
        if r == s {
            r
        } else {
            (1.1 * r * accurate + 0.8 * s * (1.0 - accurate)) / o
        }
    }
}

/// Calculate the skill bonus in real song
pub fn song_score(
    skill_ids: &Vec<u8>,
    skill_levels: &Vec<u8>,
    song_level: u32,
    has_fever: bool,
    accuracy: f64,
    song_data: &Vec<SongNote>,
    skills: &HashMap<String, Skill>,
) -> f64 {
    let accuracy_rate = 1.1 * accuracy + 0.8 * (1.0 - accuracy);
    let song_level_rate = (3.0 + 0.03 * (song_level as f64 - 5.0)) / song_data.len() as f64;
    let mut final_score = 0f64;
    let mut skill_end = 0.0;
    let mut combo_count = 0;
    let mut skill_order = 0;
    let mut y = 0;
    for note in song_data.iter() {
        // Basic bonus
        let mut bonus = accuracy_rate * song_level_rate * combo_bonus(combo_count);
        // Fever
        bonus *= match note.fever {
            Some(_) => match has_fever {
                true => 2.0,
                false => 1.0,
            },
            None => 1.0,
        };
        // Skill
        if note.time < skill_end {
            y += 1;
            let skill = &skills[&skill_ids[skill_order - 1].to_string()];
            bonus *= skill_bonus(&skill, accuracy, y);
        }
        match note.skill {
            Some(_) => {
                skill_order += 1;
                y = 0;
                let skill = &skills[&skill_ids[skill_order - 1].to_string()];
                skill_end = note.time + skill.duration[skill_levels[skill_order - 1] as usize];
                bonus *= skill_bonus(&skill, accuracy, y);
                // NOTE: Missing Center position bonus...
            }
            None => {}
        };
        final_score += bonus;
        combo_count += 1;
    }
    final_score
}

/// Generate song-skill cache
pub fn cache_table(
    calc_skills: &Vec<u32>,
    skills: &HashMap<String, Skill>,
    song_data: &Vec<SongNote>,
    song_level: u32,
    accurate: f64,
    has_fever: bool,
) -> HashMap<u32, HashMap<u32, f64>> {
    let mut table: HashMap<u32, HashMap<u32, f64>> = HashMap::new();
    for it1 in calc_skills.iter() {
        let s1 = it1 % 10;
        let l1 = (it1 - s1) / 10;
        let mut temp: HashMap<u32, f64> = HashMap::new();
        for it2 in calc_skills.iter() {
            let s2 = it2 % 10;
            let l2 = (it2 - s2) / 10;
            temp.insert(
                *it2,
                song_score(
                    &vec![l2 as u8, l2 as u8, l2 as u8, l2 as u8, l2 as u8, l1 as u8],
                    &vec![s2 as u8, s2 as u8, s2 as u8, s2 as u8, s2 as u8, s1 as u8],
                    song_level,
                    has_fever,
                    accurate,
                    song_data,
                    skills,
                ),
            );
        }
        table.insert(*it1, temp);
    }
    table
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::to_string;
    use crate::CalcCard;
    use crate::read_json::*;

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
            bp_mul: 1.0,
        };
        // 极其梦幻的生物
        let calc_card2 = CalcCard {
            card_id: 298,
            character_id: 12,
            score: 63880,
            skill_id: 13,
            skill_mul: 0.5,
            bp_mul: 1.0,
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
