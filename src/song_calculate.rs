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
    let mut m = true; // Mystery variable in bestdori...
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
                }
                if v.activate_condition == "perfect" {
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
                // m = false;
                break;
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
            bonus *= skill_bonus(&skill, 0.97, y);
        }
        match note.skill {
            Some(_) => {
                skill_order += 1;
                y = 0;
                let skill = &skills[&skill_ids[skill_order - 1].to_string()];
                skill_end = note.time + skill.duration[skill_levels[skill_order - 1] as usize];
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
