#![allow(dead_code)]
/// This crate has been used in tests, but rustc doesn't recognize it...
use crate::{Card, Character, Band, RawUserProfile, SongNote, Skill};
use std::fs::File;
use std::io::prelude::*;
use std::collections::HashMap;

pub fn read_to_str(path: String) -> Result<String, std::io::Error> {
    let mut file = File::open(path)?;
    let mut buffer = String::new();
    file.read_to_string(&mut buffer)?;
    Ok(buffer)
}

pub fn read_cards(path: String) -> Result<HashMap<String, Card>, Box<dyn std::error::Error>> {
    let buffer = read_to_str(path)?;
    Ok(serde_json::from_str(buffer.as_str())?)
}

pub fn read_characters(path: String) -> Result<HashMap<String, Character>, Box<dyn std::error::Error>> {
    let buffer = read_to_str(path)?;
    Ok(serde_json::from_str(buffer.as_str())?)
}

pub fn read_bands(path: String) -> Result<HashMap<String, Band>, Box<dyn std::error::Error>> {
    let buffer = read_to_str(path)?;
    Ok(serde_json::from_str(buffer.as_str())?)
}

pub fn read_raw_user_profile(path: String) -> Result<RawUserProfile, Box<dyn std::error::Error>> {
    let buffer = read_to_str(path)?;
    Ok(serde_json::from_str(buffer.as_str())?)
}

pub fn character_band_new_from_string(
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

pub fn read_song_notes(path: String) -> Result<Vec<SongNote>, Box<dyn std::error::Error>> {
    let buffer = read_to_str(path)?;
    Ok(serde_json::from_str(buffer.as_str())?)
}

pub fn read_skill(path: String) -> Result<HashMap<String, Skill>, Box<dyn std::error::Error>> {
    let buffer = read_to_str(path)?;
    Ok(serde_json::from_str(buffer.as_str())?)
}
