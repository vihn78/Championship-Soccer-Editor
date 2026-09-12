use std::path::{Path, PathBuf};

#[derive(Debug, Default)]
pub struct Division {
    pub name: String,
    pub level: Option<u32>,
    pub promotions: Option<u32>,
    pub relegations: Option<u32>,
    pub reputation: Option<u32>,
    pub teams: Vec<String>,
}

#[derive(Debug, Default)]
pub struct LeagueFile {
    pub path: PathBuf,
    pub country: String,
    pub divisions: Vec<Division>,
    pub warnings: Vec<String>,
}

pub struct World {
    pub league_directory: PathBuf,
    pub leagues: Vec<LeagueFile>,
    pub international_file: Option<PathBuf>,
    pub warnings: Vec<String>,
}

/// Accettiamo la cartella del gioco/pacchetto, Data oppure League.
pub fn load_world(selected: &Path) -> Result<World, String> {
    let directory = [selected.join("Data/League"), selected.join("League")]
        .into_iter()
        .chain(selected.file_name().filter(|name| name.eq_ignore_ascii_case("League"))
            .map(|_| selected.to_path_buf()))
        .find(|path| path.is_dir())
        .ok_or_else(|| "Cartella non valida: seleziona un mondo contenente Data/League oppure la cartella Data o League.".to_owned())?;
    let entries = std::fs::read_dir(&directory)
        .map_err(|error| format!("{}: {error}", directory.display()))?;
    let mut world = World {
        league_directory: directory,
        leagues: Vec::new(),
        international_file: None,
        warnings: Vec::new(),
    };
    let mut paths = Vec::new();
    for entry in entries {
        match entry {
            Ok(entry) => {
                let path = entry.path();
                if path.is_file()
                    && path
                        .extension()
                        .is_some_and(|extension| extension.eq_ignore_ascii_case("txt"))
                {
                    paths.push(path);
                }
            }
            Err(error) => world
                .warnings
                .push(format!("Impossibile leggere una voce: {error}")),
        }
    }
    paths.sort_by_key(|path| path.to_string_lossy().to_lowercase());
    for path in paths {
        if path
            .file_stem()
            .is_some_and(|name| name.eq_ignore_ascii_case("International teams and tournaments"))
        {
            world.international_file = Some(path);
            continue;
        }
        match std::fs::read(&path) {
            Ok(bytes) => {
                let text = decode_text(&bytes);
                let league = parse_league(&path, &text);
                world.warnings.extend(
                    league
                        .warnings
                        .iter()
                        .map(|warning| format!("{}: {warning}", path.display())),
                );
                world.leagues.push(league);
            }
            Err(error) => world.warnings.push(format!("{}: {error}", path.display())),
        }
    }
    if world.leagues.is_empty() && world.international_file.is_none() {
        return Err(format!(
            "Nessun file lega leggibile in {}. {}",
            world.league_directory.display(),
            world.warnings.join("\n")
        ));
    }
    Ok(world)
}

fn decode_text(bytes: &[u8]) -> String {
    // UTF-8 valido (anche con BOM) oppure il Windows-1252 dei dati storici.
    match std::str::from_utf8(bytes) {
        Ok(text) => text.trim_start_matches('\u{feff}').to_owned(),
        Err(_) => encoding_rs::WINDOWS_1252.decode(bytes).0.into_owned(),
    }
}

fn parse_league(path: &Path, text: &str) -> LeagueFile {
    let mut league = LeagueFile {
        path: path.to_path_buf(),
        ..Default::default()
    };
    let mut current_division = None;
    let mut in_cup = false;
    for (index, raw_line) in text.lines().enumerate() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with(['#', ';']) {
            continue;
        }
        let warning = |message: &str| format!("riga {}: {message} ({line})", index + 1);
        if let Some(name) = line.strip_prefix(':') {
            league.divisions.push(Division {
                name: name.trim().into(),
                ..Default::default()
            });
            current_division = Some(league.divisions.len() - 1);
            in_cup = false;
            if name.trim().is_empty() {
                league.warnings.push(warning("Nome divisione mancante"));
            }
        } else if line.starts_with('*') {
            // Le coppe non devono contaminare proprietà o partecipanti della divisione.
            current_division = None;
            in_cup = true;
        } else if let Some(instruction) = line.strip_prefix('!') {
            let (key, value) = split_instruction(instruction);
            match key.to_ascii_lowercase().as_str() {
                "country" => {
                    let country = value
                        .strip_prefix('"')
                        .and_then(|value| value.strip_suffix('"'));
                    if let Some(country) = country.filter(|value| !value.is_empty()) {
                        league.country = country.into();
                    } else {
                        league
                            .warnings
                            .push(warning("Paese mancante o virgolette non valide"));
                    }
                }
                "names" | "intro" => {}
                _ => league.warnings.push(warning("Istruzione non riconosciuta")),
            }
        } else if let Some(instruction) = line.strip_prefix('+') {
            let (key, value) = split_instruction(instruction);
            let key = key.to_ascii_lowercase();
            if let Some(division_index) = current_division {
                let division = &mut league.divisions[division_index];
                let field = match key.as_str() {
                    "division" => Some(&mut division.level),
                    "promotions" => Some(&mut division.promotions),
                    "relegations" => Some(&mut division.relegations),
                    "reputation" => Some(&mut division.reputation),
                    _ => None,
                };
                if let Some(field) = field {
                    match value.parse::<u32>() {
                        Ok(number) => {
                            *field = Some(number);
                        }
                        Err(_) => league.warnings.push(warning("Valore numerico non valido")),
                    }
                } else {
                    league
                        .warnings
                        .push(warning("Proprietà divisione non riconosciuta"));
                }
            } else if !in_cup
                || !matches!(
                    key.as_str(),
                    "country"
                        | "reputation"
                        | "round"
                        | "add"
                        | "draw"
                        | "matches"
                        | "week"
                        | "groups"
                        | "venue"
                        | "international"
                        | "exclude"
                        | "noexclude"
                        | "nations"
                        | "year"
                        | "offset"
                )
            {
                league.warnings.push(warning("Istruzione non interpretata"));
            }
        } else if let Some(division_index) = current_division {
            if line.starts_with(['$', '@', '=']) {
                league.warnings.push(warning("Istruzione non riconosciuta"));
            } else {
                league.divisions[division_index].teams.push(line.into());
            }
        } else {
            league.warnings.push(warning("Riga fuori da una divisione"));
        }
    }
    if league.country.is_empty() {
        league
            .warnings
            .push("Paese non definito con ! Country".into());
    }
    if league.divisions.is_empty() {
        league.warnings.push("Nessuna divisione trovata".into());
    }
    for division in &league.divisions {
        if division.level.is_none() {
            league
                .warnings
                .push(format!("{}: livello divisione mancante", division.name));
        }
        if division.teams.is_empty() {
            league
                .warnings
                .push(format!("{}: nessuna squadra", division.name));
        }
    }
    league
}

fn split_instruction(instruction: &str) -> (&str, &str) {
    let instruction = instruction.trim();
    instruction
        .split_once(char::is_whitespace)
        .map_or((instruction, ""), |(key, value)| (key, value.trim()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preserves_order_and_separates_cup_properties() {
        let league = parse_league(
            Path::new("Example.txt"),
            "# comment\n! Country \"Example\"\n: First\n+ division 1\n+ reputation 8\nZulu\nAlpha\n* Cup\n+ reputation 20\n+ round 1 \"Final\" 1-2 47\n: Second\n+ division 2\nBeta\n",
        );
        assert_eq!(league.divisions.len(), 2);
        assert_eq!(league.divisions[0].teams, ["Zulu", "Alpha"]);
        assert_eq!(league.divisions[0].reputation, Some(8));
        assert_eq!(league.divisions[1].name, "Second");
        assert!(league.warnings.is_empty(), "{:?}", league.warnings);
    }

    #[test]
    fn decodes_legacy_accents_and_utf8_bom() {
        assert_eq!(decode_text(b"Li\xe8ge"), "Liège");
        assert_eq!(decode_text(b"\xef\xbb\xbfItaly"), "Italy");
    }

    #[test]
    fn reports_invalid_values_and_unknown_instructions_with_line_numbers() {
        let league = parse_league(
            Path::new("Bad.txt"),
            "! Country \"Test\"\n: First\n+ division invalid\n+ mystery 5\nClub\n",
        );
        assert!(
            league
                .warnings
                .iter()
                .any(|warning| warning.contains("riga 3"))
        );
        assert!(
            league
                .warnings
                .iter()
                .any(|warning| warning.contains("riga 4"))
        );
        assert_eq!(league.divisions[0].level, None);
        assert_eq!(league.divisions[0].teams, ["Club"]);
    }

    #[test]
    fn loads_supplied_world_and_italian_divisions() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
        for selected in [
            root.to_path_buf(),
            root.join("Data"),
            root.join("Data/League"),
        ] {
            let world = load_world(&selected).unwrap();
            assert_eq!(world.leagues.len(), 23);
            assert!(world.international_file.is_some());
            let italy = world
                .leagues
                .iter()
                .find(|league| league.country == "Italy")
                .unwrap();
            assert_eq!(italy.divisions.len(), 9);
            assert_eq!(italy.divisions[0].name, "Serie A");
            assert_eq!(italy.divisions[0].teams.len(), 20);
            assert_eq!(&italy.divisions[0].teams[..3], ["Inter", "Napoli", "Roma"]);
            assert!(italy.warnings.is_empty(), "{:?}", italy.warnings);
        }
    }

    #[test]
    fn rejects_folder_without_leagues() {
        assert!(load_world(Path::new(env!("CARGO_MANIFEST_DIR"))).is_err());
    }
}
