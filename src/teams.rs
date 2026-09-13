use eframe::egui;
use std::collections::{BTreeMap, HashMap};
use std::path::Path;

pub const KIT_PARTS: [&str; 5] = ["shirt", "stripes", "sleeves", "shorts", "socks"];
pub const PLAYER_SKILLS: [(&str, &str); 22] = [
    ("speed", "Velocità"),
    ("shotPower", "Potenza tiro"),
    ("shotControl", "Controllo tiro"),
    ("stamina", "Resistenza"),
    ("dribbling", "Dribbling"),
    ("jumping", "Elevazione"),
    ("anticipation", "Anticipo"),
    ("agility", "Agilità"),
    ("tackling", "Contrasto"),
    ("heading", "Colpo di testa"),
    ("ballControl", "Controllo palla"),
    ("handling", "Presa"),
    ("reflexes", "Riflessi"),
    ("aggression", "Aggressività"),
    ("longShots", "Tiri da lontano"),
    ("teamwork", "Gioco di squadra"),
    ("offensiveness", "Propensione offensiva"),
    ("forwardRuns", "Inserimenti"),
    ("vision", "Visione"),
    ("adaptability", "Adattabilità"),
    ("loyalty", "Lealtà"),
    ("greed", "Avidità"),
];

#[derive(Clone)]
pub struct TeamProfile {
    pub file_exists: bool,
    pub reputation: u32,
    pub kits: [[String; 5]; 3],
    pub source_bytes: Option<Vec<u8>>,
    pub players: Vec<String>,
}

impl TeamProfile {
    pub fn defaults(reputation: u32) -> Self {
        Self {
            file_exists: false,
            reputation,
            kits: std::array::from_fn(|_| std::array::from_fn(|_| "white".into())),
            source_bytes: None,
            players: Vec::new(),
        }
    }
}

pub fn load_team(path: &Path, default_reputation: u32) -> TeamProfile {
    let Ok(bytes) = std::fs::read(path) else {
        return TeamProfile::defaults(default_reputation);
    };
    let mut profile = TeamProfile::defaults(default_reputation);
    profile.file_exists = true;
    profile.source_bytes = Some(bytes.clone());
    let mut in_details = false;
    for line in crate::leagues::decode_text(&bytes).lines().map(str::trim) {
        if line.starts_with('$') {
            in_details = true;
            continue;
        }
        if !in_details && !line.is_empty() && !line.starts_with(['!', '#', ';']) {
            profile.players.push(line.into());
            continue;
        }
        let Some(instruction) = line.strip_prefix('!') else {
            continue;
        };
        let fields: Vec<_> = instruction.split_whitespace().collect();
        match fields.as_slice() {
            ["reputation", value] => {
                if let Ok(value) = value.parse::<u32>() {
                    profile.reputation = value;
                }
            }
            [number, part, colour] => {
                if let Ok(number) = number.parse::<usize>()
                    && let Some(part_index) = KIT_PARTS
                        .iter()
                        .position(|item| item.eq_ignore_ascii_case(part))
                    && (1..=3).contains(&number)
                {
                    profile.kits[number - 1][part_index] = (*colour).into();
                }
            }
            _ => {}
        }
    }
    profile
}

pub fn player_nationality(path: &Path, fallback: &str) -> String {
    let Ok(bytes) = std::fs::read(path) else {
        return fallback.into();
    };
    crate::leagues::decode_text(&bytes)
        .lines()
        .filter_map(|line| line.split_once('='))
        .find(|(key, _)| key.trim().eq_ignore_ascii_case("nationality"))
        .map(|(_, value)| value.trim().into())
        .unwrap_or_else(|| fallback.into())
}

pub fn player_file_name(player_entry: &str) -> &str {
    player_entry
        .split_once(':')
        .map_or(player_entry, |(name, _)| name)
        .trim()
}

pub fn roster_position(player_entry: &str) -> Option<(String, String)> {
    let (_, position) = player_entry.split_once(':')?;
    let mut fields = position.split_whitespace();
    let role = fields.next()?;
    if !matches!(role, "GK" | "SW" | "D" | "DM" | "M" | "AM" | "F") {
        return None;
    }
    let side = fields
        .next()
        .filter(|side| matches!(*side, "L" | "C" | "R"))
        .unwrap_or("");
    Some((role.into(), side.into()))
}

#[derive(Clone)]
pub struct PlayerProfile {
    pub source_bytes: Option<Vec<u8>>,
    fields: BTreeMap<String, String>,
}

impl PlayerProfile {
    pub fn empty() -> Self {
        Self {
            source_bytes: None,
            fields: BTreeMap::new(),
        }
    }

    pub fn value(&self, key: &str) -> Option<&str> {
        canonical_player_key(key).and_then(|key| self.fields.get(key).map(String::as_str))
    }

    pub fn set(&mut self, key: &str, value: impl Into<String>) {
        if let Some(key) = canonical_player_key(key) {
            self.fields.insert(key.into(), value.into());
        }
    }

    pub fn remove(&mut self, key: &str) {
        if let Some(key) = canonical_player_key(key) {
            self.fields.remove(key);
        }
    }
}

pub fn load_player(path: &Path) -> PlayerProfile {
    let Ok(bytes) = std::fs::read(path) else {
        return PlayerProfile::empty();
    };
    let mut profile = PlayerProfile {
        source_bytes: Some(bytes.clone()),
        ..PlayerProfile::empty()
    };
    for line in crate::leagues::decode_text(&bytes).lines() {
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        profile.set(key.trim(), value.trim());
    }
    profile
}

fn canonical_player_key(key: &str) -> Option<&'static str> {
    let normalized: String = key
        .chars()
        .filter(|character| !matches!(character, ' ' | '-' | '_'))
        .flat_map(char::to_lowercase)
        .collect();
    match normalized.as_str() {
        "nationality" | "nation" => Some("nationality"),
        "ability" => Some("ability"),
        "reputation" => Some("reputation"),
        "potential" => Some("potential"),
        "position" => Some("position"),
        "yearofbirth" => Some("yearOfBirth"),
        "dateofbirth" => Some("dateOfBirth"),
        "skin" => Some("skin"),
        "hair" => Some("hair"),
        "keeping" | "handling" => Some("handling"),
        "attackmindedness" | "offensiveness" => Some("offensiveness"),
        "adaptation" | "adaptability" => Some("adaptability"),
        "greediness" | "greed" => Some("greed"),
        _ => PLAYER_SKILLS
            .iter()
            .find(|(field, _)| field.to_lowercase() == normalized)
            .map(|(field, _)| *field),
    }
}

pub fn save_player(path: &Path, profile: &PlayerProfile) -> Result<(), String> {
    if let Some(source) = &profile.source_bytes
        && std::fs::read(path).map_err(|error| error.to_string())? != *source
    {
        return Err(format!("{} è cambiato esternamente.", path.display()));
    }
    let source = profile
        .source_bytes
        .as_deref()
        .map_or_else(String::new, crate::leagues::decode_text);
    let newline = if source.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    };
    let mut seen = std::collections::BTreeSet::new();
    let mut output = String::new();
    for raw in source.split_inclusive('\n') {
        let (body, ending) = raw.strip_suffix("\r\n").map_or_else(
            || {
                raw.strip_suffix('\n')
                    .map_or((raw, ""), |body| (body, "\n"))
            },
            |body| (body, "\r\n"),
        );
        if let Some((key, _)) = body.split_once('=')
            && let Some(key) = canonical_player_key(key.trim())
        {
            seen.insert(key);
            if let Some(value) = profile.fields.get(key) {
                output.push_str(&format!("{key} = {value}{ending}"));
            }
        } else {
            output.push_str(raw);
        }
    }
    if !output.is_empty() && !output.ends_with('\n') {
        output.push_str(newline);
    }
    for (key, value) in &profile.fields {
        if !seen.contains(key.as_str()) {
            output.push_str(&format!("{key} = {value}{newline}"));
        }
    }
    std::fs::create_dir_all(path.parent().ok_or("Cartella Player non disponibile.")?)
        .map_err(|error| error.to_string())?;
    let temporary = path.with_extension("txt.world-editor.tmp");
    let bytes = if profile
        .source_bytes
        .as_ref()
        .is_some_and(|source| std::str::from_utf8(source).is_err())
    {
        let (bytes, _, errors) = encoding_rs::WINDOWS_1252.encode(&output);
        if errors {
            return Err(
                "Il giocatore contiene caratteri non rappresentabili in Windows-1252.".into(),
            );
        }
        bytes.into_owned()
    } else {
        output.into_bytes()
    };
    std::fs::write(&temporary, bytes).map_err(|error| error.to_string())?;
    std::fs::rename(&temporary, path).map_err(|error| error.to_string())
}

pub fn colour_names(path: &Path) -> Vec<String> {
    let Ok(bytes) = std::fs::read(path) else {
        return vec!["white".into()];
    };
    let mut names = Vec::new();
    for line in crate::leagues::decode_text(&bytes).lines() {
        let mut fields = line.split_whitespace();
        if fields.next().is_some_and(|hex| hex.starts_with('$'))
            && let Some(name) = fields.next()
            && !names
                .iter()
                .any(|existing: &String| existing.eq_ignore_ascii_case(name))
        {
            names.push(name.into());
        }
    }
    if names.is_empty() {
        vec!["white".into()]
    } else {
        names
    }
}

pub fn save_team(path: &Path, team: &str, profile: &TeamProfile) -> Result<(), String> {
    if let Some(source) = &profile.source_bytes
        && std::fs::read(path).map_err(|error| error.to_string())? != *source
    {
        return Err(format!("{} è cambiato esternamente.", path.display()));
    }
    let source = profile
        .source_bytes
        .as_deref()
        .map_or_else(String::new, crate::leagues::decode_text);
    let newline = if source.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    };
    let mut seen = [[false; 5]; 3];
    let mut reputation_seen = false;
    let mut player_index = 0usize;
    let mut in_details = false;
    let mut output = String::new();
    if source.is_empty() {
        output.push_str(&format!("! Name \"{team}\"{newline}"));
    }
    for raw in source.split_inclusive('\n') {
        let (body, ending) = raw.strip_suffix("\r\n").map_or_else(
            || {
                raw.strip_suffix('\n')
                    .map_or((raw, ""), |body| (body, "\n"))
            },
            |body| (body, "\r\n"),
        );
        let fields: Vec<_> = body
            .trim()
            .strip_prefix('!')
            .unwrap_or("")
            .split_whitespace()
            .collect();
        if body.trim().starts_with('$') {
            in_details = true;
        }
        if let [key, _value] = fields.as_slice()
            && key.eq_ignore_ascii_case("reputation")
        {
            reputation_seen = true;
            output.push_str(&format!("! reputation {}{ending}", profile.reputation));
        } else if let [number, part, _colour] = fields.as_slice()
            && let Ok(number) = number.parse::<usize>()
            && let Some(part_index) = KIT_PARTS
                .iter()
                .position(|item| item.eq_ignore_ascii_case(part))
            && (1..=3).contains(&number)
        {
            seen[number - 1][part_index] = true;
            output.push_str(&format!(
                "! {} {part} {}{ending}",
                number,
                profile.kits[number - 1][part_index]
            ));
        } else if !in_details
            && !body.trim().is_empty()
            && !body.trim().starts_with(['!', '#', ';'])
        {
            if let Some(player) = profile.players.get(player_index) {
                output.push_str(player);
                output.push_str(ending);
            }
            player_index += 1;
        } else {
            output.push_str(raw);
        }
    }
    if !output.is_empty() && !output.ends_with('\n') {
        output.push_str(newline);
    }
    if !reputation_seen {
        output.push_str(&format!("! reputation {}{newline}", profile.reputation));
    }
    for (kit_index, kit) in profile.kits.iter().enumerate() {
        for (part_index, part) in KIT_PARTS.iter().enumerate() {
            if !seen[kit_index][part_index] {
                output.push_str(&format!(
                    "! {} {part} {}{newline}",
                    kit_index + 1,
                    kit[part_index]
                ));
            }
        }
    }
    let parent = path
        .parent()
        .ok_or("Cartella Team non disponibile.")?
        .parent()
        .ok_or("Cartella Data non disponibile.")?;
    let temporary = parent.join(format!(
        ".world-editor-team-{}-{}.tmp",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|error| error.to_string())?
            .as_nanos()
    ));
    let bytes = if profile
        .source_bytes
        .as_ref()
        .is_some_and(|source| std::str::from_utf8(source).is_err())
    {
        let (bytes, _, errors) = encoding_rs::WINDOWS_1252.encode(&output);
        if errors {
            return Err("La divisa contiene caratteri non rappresentabili in Windows-1252.".into());
        }
        bytes.into_owned()
    } else {
        output.into_bytes()
    };
    std::fs::write(&temporary, bytes).map_err(|error| error.to_string())?;
    if let Err(error) = std::fs::rename(&temporary, path) {
        let _ = std::fs::remove_file(temporary);
        return Err(error.to_string());
    }
    Ok(())
}

pub fn load_colours(path: &Path) -> HashMap<String, egui::Color32> {
    let Ok(bytes) = std::fs::read(path) else {
        return HashMap::new();
    };
    crate::leagues::decode_text(&bytes)
        .lines()
        .filter_map(|line| {
            let mut fields = line.split_whitespace();
            let hex = fields.next()?.strip_prefix('$')?;
            let rgb = u32::from_str_radix(hex, 16).ok()?;
            Some((
                fields.collect::<Vec<_>>(),
                egui::Color32::from_rgb((rgb >> 16) as u8, (rgb >> 8) as u8, rgb as u8),
            ))
        })
        .flat_map(|(names, colour)| {
            names
                .into_iter()
                .map(move |name| (name.to_lowercase(), colour))
        })
        .collect()
}

pub fn colour_for(colours: &HashMap<String, egui::Color32>, name: &str) -> egui::Color32 {
    colours
        .get(&name.to_lowercase())
        .copied()
        .unwrap_or(egui::Color32::WHITE)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_team_file_uses_white_kits_and_the_supplied_reputation() {
        let profile = load_team(Path::new("missing-team.txt"), 7);
        assert!(!profile.file_exists);
        assert_eq!(profile.reputation, 7);
        assert!(
            profile
                .kits
                .iter()
                .flatten()
                .all(|colour| colour == "white")
        );
    }

    #[test]
    fn parses_team_colours_and_reputation() {
        let directory = std::env::temp_dir().join("world-editor-team-parser");
        let _ = std::fs::create_dir(&directory);
        let path = directory.join("Test.txt");
        std::fs::write(&path, "! reputation 18\n! 1 shirt red\n! 2 socks blue\n").unwrap();
        let profile = load_team(&path, 5);
        assert_eq!(profile.reputation, 18);
        assert_eq!(profile.kits[0][0], "red");
        assert_eq!(profile.kits[1][4], "blue");
        let _ = std::fs::remove_file(path);
        let _ = std::fs::remove_dir(directory);
    }

    #[test]
    fn saves_kit_colours_without_removing_team_details_or_players() {
        let root = std::env::temp_dir().join("world-editor-team-save");
        let team_directory = root.join("Data").join("Team");
        let _ = std::fs::create_dir_all(&team_directory);
        let path = team_directory.join("Test.txt");
        std::fs::write(
            &path,
            "! Name \"Test\"\n! reputation 12\n! 1 shirt red\nPlayer, One: F C\nPlayer, Two: M C\n",
        )
        .unwrap();
        let mut profile = load_team(&path, 5);
        profile.kits[0][0] = "blue".into();
        profile.players.swap(0, 1);
        save_team(&path, "Test", &profile).unwrap();
        let saved = std::fs::read_to_string(&path).unwrap();
        assert!(saved.contains("! reputation 12"));
        assert!(saved.contains("! 1 shirt blue"));
        assert!(saved.contains("Player, Two: M C"));
        assert!(saved.contains("Player, One: F C"));
        assert!(saved.find("Player, Two: M C") < saved.find("Player, One: F C"));
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn player_profile_removes_disabled_fields_when_saved() {
        let root = std::env::temp_dir().join("world-editor-player-save");
        let path = root.join("Test.txt");
        let _ = std::fs::create_dir_all(&root);
        std::fs::write(&path, "nationality = Italy\nspeed = 18\nskin = white\n").unwrap();
        let mut profile = load_player(&path);
        profile.remove("speed");
        profile.set("ability", "197");
        save_player(&path, &profile).unwrap();
        let saved = std::fs::read_to_string(&path).unwrap();
        assert!(saved.contains("nationality = Italy"));
        assert!(saved.contains("ability = 197"));
        assert!(!saved.contains("speed ="));
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn roster_position_separates_role_and_side() {
        assert_eq!(
            roster_position("Taylor, Alex (1997): AM R"),
            Some(("AM".into(), "R".into()))
        );
        assert_eq!(
            roster_position("Taylor, Alex (1997): GK"),
            Some(("GK".into(), "".into()))
        );
    }
}
