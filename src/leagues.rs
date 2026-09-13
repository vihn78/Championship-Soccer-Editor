use std::path::{Path, PathBuf};

#[derive(Debug, Default, Clone, PartialEq)]
pub struct Division {
    pub name: String,
    pub level: Option<u32>,
    pub promotions: Option<u32>,
    pub relegations: Option<u32>,
    pub reputation: Option<u32>,
    pub teams: Vec<String>,
}

#[derive(Debug, Default, Clone)]
pub struct LeagueFile {
    pub division_sources: Vec<Option<usize>>,
    pub name_files: Vec<String>,
    pub source_bytes: Vec<u8>,
    pub path: PathBuf,
    pub country: String,
    pub divisions: Vec<Division>,
    pub warnings: Vec<String>,
}

impl LeagueFile {
    pub fn swap_clubs(&mut self, source: (usize, usize), target: (usize, usize)) -> bool {
        let club = |position: (usize, usize)| {
            self.divisions
                .get(position.0)
                .and_then(|division| division.teams.get(position.1))
                .cloned()
        };
        let (Some(first), Some(second)) = (club(source), club(target)) else {
            return false;
        };
        if source == target {
            return false;
        }
        self.divisions[source.0].teams[source.1] = second;
        self.divisions[target.0].teams[target.1] = first;
        true
    }

    pub fn move_target(
        &self,
        division: usize,
        position: usize,
        up: bool,
    ) -> Option<(usize, usize)> {
        let current = self.divisions.get(division)?;
        current.teams.get(position)?;
        if up && position > 0 {
            return Some((division, position - 1));
        }
        if !up && position + 1 < current.teams.len() {
            return Some((division, position + 1));
        }
        let level = current.level?;
        // Livelli ambigui o mancanti non devono provocare scambi con la divisione sbagliata.
        if self
            .divisions
            .iter()
            .filter(|item| item.level == Some(level))
            .count()
            != 1
        {
            return None;
        }
        let adjacent_level = if up {
            level.checked_sub(1)?
        } else {
            level.checked_add(1)?
        };
        let mut adjacent = self
            .divisions
            .iter()
            .enumerate()
            .filter(|(_, item)| item.level == Some(adjacent_level));
        let (index, target) = adjacent.next()?;
        if adjacent.next().is_some() || target.teams.is_empty() {
            return None;
        }
        Some((index, if up { target.teams.len() - 1 } else { 0 }))
    }

    pub fn move_club(
        &mut self,
        division: usize,
        position: usize,
        up: bool,
    ) -> Option<(usize, usize)> {
        let target = self.move_target(division, position, up)?;
        self.swap_clubs((division, position), target)
            .then_some(target)
    }
    /// Confronto esatto dei nomi già ripuliti dal parser, in tutte le divisioni del paese.
    /// Più risultati indicano un duplicato: nessuna corrispondenza viene scelta arbitrariamente.
    pub fn club_locations(&self, club: &str) -> Vec<(usize, usize)> {
        self.divisions
            .iter()
            .enumerate()
            .flat_map(|(division_index, division)| {
                division
                    .teams
                    .iter()
                    .enumerate()
                    .filter_map(move |(team_index, name)| {
                        (name == club).then_some((division_index, team_index))
                    })
            })
            .collect()
    }
    pub fn division_issues(&self, index: usize) -> Vec<String> {
        let division = &self.divisions[index];
        let mut issues = Vec::new();
        if division.name.trim().is_empty() {
            issues.push("Nome: inserisci il nome della divisione.".into());
        }
        if let Some(level) = division.level {
            if level == 0 {
                issues.push("Livello: deve essere maggiore di zero.".into());
            }
            if self
                .divisions
                .iter()
                .enumerate()
                .any(|(other, value)| other != index && value.level == Some(level))
            {
                issues.push("Livello: già occupato da un'altra divisione.".into());
            }
            if level == 1 && division.promotions.is_some_and(|value| value != 0) {
                issues.push("Promozioni: la prima divisione deve avere zero promozioni.".into());
            }
            if let Some(above) = self
                .divisions
                .iter()
                .find(|other| other.level == level.checked_sub(1))
                && division.promotions != above.relegations
            {
                issues.push(format!(
                    "Promozioni: devono corrispondere alle retrocessioni di {}.",
                    above.name
                ));
            }
            if let Some(below) = self
                .divisions
                .iter()
                .find(|other| other.level == level.checked_add(1))
            {
                if division.relegations != below.promotions {
                    issues.push(format!(
                        "Retrocessioni: devono corrispondere alle promozioni di {}.",
                        below.name
                    ));
                }
            } else if !self
                .divisions
                .iter()
                .any(|other| other.level.is_some_and(|other| other > level))
                && division.relegations.is_some_and(|value| value != 0)
            {
                issues.push(
                    "Retrocessioni: l'ultima divisione deve avere zero retrocessioni.".into(),
                );
            }
        }
        if division
            .reputation
            .is_some_and(|value| !(1..=20).contains(&value))
        {
            issues.push("Reputazione: usa un valore da 1 a 20.".into());
        }
        issues
    }
}

#[derive(Clone)]
pub struct World {
    pub international_bytes: Vec<u8>,
    pub retained_clubs: std::collections::BTreeSet<String>,
    pub deleted_clubs: std::collections::BTreeSet<String>,
    pub country_names: crate::country_display::CountryNames,
    pub league_directory: PathBuf,
    pub leagues: Vec<LeagueFile>,
    pub international_file: Option<PathBuf>,
    pub international_countries: Vec<InternationalCountry>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct InternationalCountry {
    pub name: String,
    pub name_files: Vec<String>,
    pub reputation: Option<u32>,
    pub clubs: Vec<String>,
}

/// Legge esclusivamente i blocchi paese; le istruzioni delle coppe restano separate.
pub(crate) fn parse_international_countries(
    text: &str,
) -> (Vec<InternationalCountry>, Vec<String>) {
    let mut countries: Vec<InternationalCountry> = Vec::new();
    let mut warnings = Vec::new();
    let mut current = None;
    for (index, raw) in text.lines().enumerate() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with(['#', ';']) {
            continue;
        }
        if line.starts_with(['*', ':']) {
            current = None;
            continue;
        }
        if let Some(instruction) = line.strip_prefix('!').or_else(|| line.strip_prefix('+')) {
            let (key, value) = split_instruction(instruction);
            if key.eq_ignore_ascii_case("AdditionalCountry") && line.starts_with('!') {
                current = None;
                if let Some(name) = value
                    .strip_prefix('"')
                    .and_then(|value| value.strip_suffix('"'))
                    .filter(|name| !name.trim().is_empty())
                {
                    countries.push(InternationalCountry {
                        name: name.trim().into(),
                        ..Default::default()
                    });
                    current = Some(countries.len() - 1);
                } else {
                    warnings.push(format!(
                        "riga {}: paese internazionale non valido",
                        index + 1
                    ));
                }
            } else if let Some(country_index) = current {
                let country = &mut countries[country_index];
                match key.to_ascii_lowercase().as_str() {
                    "tempnames" => {
                        let parts: Vec<_> = value.split('"').collect();
                        if parts.len() == 5
                            && parts[0].trim().is_empty()
                            && parts[2].trim().is_empty()
                            && parts[4].trim().is_empty()
                            && !parts[1].is_empty()
                            && !parts[3].is_empty()
                        {
                            country.name_files = vec![parts[1].into(), parts[3].into()];
                        } else {
                            warnings.push(format!(
                                "riga {}: TempNames richiede due percorsi tra virgolette",
                                index + 1
                            ));
                        }
                    }
                    "reputation" => match value.parse() {
                        Ok(reputation) => country.reputation = Some(reputation),
                        Err(_) => {
                            warnings.push(format!("riga {}: reputazione non valida", index + 1))
                        }
                    },
                    _ => warnings.push(format!(
                        "riga {}: istruzione paese non riconosciuta ({line})",
                        index + 1
                    )),
                }
            }
        } else if let Some(country_index) = current {
            if line.starts_with(['$', '@', '=']) {
                warnings.push(format!(
                    "riga {}: istruzione paese non riconosciuta ({line})",
                    index + 1
                ));
            } else {
                countries[country_index].clubs.push(line.into());
            }
        }
    }
    (countries, warnings)
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
        international_bytes: Vec::new(),
        retained_clubs: Default::default(),
        deleted_clubs: Default::default(),
        country_names: crate::country_display::CountryNames,
        league_directory: directory,
        leagues: Vec::new(),
        international_file: None,
        international_countries: Vec::new(),
        warnings: Vec::new(),
    };
    // Il gioco ha un problema con alcuni display name nelle competizioni nazionali.
    // Perciò l'editor usa l'identità dei file e non legge Nationalities.txt.
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
            match std::fs::read(&path) {
                Ok(bytes) => {
                    let (countries, warnings) = parse_international_countries(&decode_text(&bytes));
                    world.international_countries = countries;
                    world.international_bytes = bytes;
                    world.warnings.extend(
                        warnings
                            .into_iter()
                            .map(|warning| format!("{}: {warning}", path.display())),
                    );
                    world.international_file = Some(path);
                }
                Err(error) => world.warnings.push(format!("{}: {error}", path.display())),
            }
            continue;
        }
        match std::fs::read(&path) {
            Ok(bytes) => {
                let text = decode_text(&bytes);
                let mut league = parse_league(&path, &text);
                league.source_bytes = bytes;
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

pub(crate) fn decode_text(bytes: &[u8]) -> String {
    // UTF-8 valido (anche con BOM) oppure il Windows-1252 dei dati storici.
    match std::str::from_utf8(bytes) {
        Ok(text) => text.trim_start_matches('\u{feff}').to_owned(),
        Err(_) => encoding_rs::WINDOWS_1252.decode(bytes).0.into_owned(),
    }
}

pub(crate) fn parse_league(path: &Path, text: &str) -> LeagueFile {
    let mut league = LeagueFile {
        source_bytes: text.as_bytes().to_vec(),
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
                "names" => {
                    let parts: Vec<_> = value.split('"').collect();
                    if parts.len() == 5 {
                        league.name_files = vec![parts[1].into(), parts[3].into()];
                    }
                }
                "intro" => {}
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
    league.division_sources = (0..league.divisions.len()).map(Some).collect();
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
    fn moving_across_divisions_swaps_boundary_clubs_and_preserves_counts() {
        let mut league = parse_league(
            Path::new("Test.txt"),
            "! Country \"Test\"\n: A\n+ division 1\nA1\nA2\nA3\n: B\n+ division 2\nB1\nB2\nB3\n",
        );
        assert_eq!(league.move_club(1, 0, true), Some((0, 2)));
        assert_eq!(league.divisions[0].teams, ["A1", "A2", "B1"]);
        assert_eq!(league.divisions[1].teams, ["A3", "B2", "B3"]);
        assert_eq!(league.move_club(0, 2, false), Some((1, 0)));
        assert_eq!(league.divisions[0].teams, ["A1", "A2", "A3"]);
        assert_eq!(league.move_club(0, 0, true), None);
        assert_eq!(league.move_club(1, 2, false), None);
        assert!(league.swap_clubs((0, 0), (1, 2)));
        assert_eq!(league.divisions[0].teams[0], "B3");
        assert_eq!(league.divisions[1].teams[2], "A1");
        assert!(!league.swap_clubs((99, 0), (0, 0)));
    }

    #[test]
    fn international_club_matches_all_divisions_without_guessing() {
        let league = parse_league(
            Path::new("Test.txt"),
            "! Country \"Test\"\n: First\n+ division 1\nAlpha\nDuplicate\n: Second\n+ division 2\nBeta\nDuplicate\n",
        );
        assert_eq!(league.club_locations("Beta"), vec![(1, 0)]);
        assert_eq!(league.club_locations("Missing"), vec![]);
        assert_eq!(league.club_locations("Duplicate"), vec![(0, 1), (1, 1)]);
        assert_eq!(league.club_locations("Alph"), vec![]);
        assert_eq!(league.club_locations("alpha"), vec![]);
    }

    #[test]
    fn division_edit_validates_and_preserves_other_divisions_and_team_order() {
        let mut league = parse_league(
            Path::new("Test.txt"),
            "! Country \"Test\"\n: First\n+ division 1\n+ promotions 0\n+ relegations 1\n+ reputation 10\nZulu\nAlpha\nBeta\n: Second\n+ division 2\n+ promotions 1\n+ relegations 0\n+ reputation 5\nOther\n",
        );
        let original = league.divisions.clone();
        league.divisions[0].name = "New name".into();
        league.divisions[0].reputation = Some(12);
        assert!(league.division_issues(0).is_empty());
        assert_eq!(league.divisions[0].teams, original[0].teams);
        assert_eq!(league.divisions[1], original[1]);
        league.divisions[0].level = Some(2);
        assert!(
            league
                .division_issues(0)
                .iter()
                .any(|message| message.contains("occupato"))
        );
        league.divisions[0].name.clear();
        assert!(
            league
                .division_issues(0)
                .iter()
                .any(|message| message.contains("Nome"))
        );
        league.divisions = original.clone();
        assert_eq!(league.divisions, original);
    }

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

    #[test]
    fn international_blocks_ignore_comments_and_cups_and_accept_both_reputation_prefixes() {
        let (countries, warnings) = parse_international_countries(
            "#! AdditionalCountry \"Ignored\"\n! AdditionalCountry \"One\"\n! TempNames \"first names.txt\" \"last names.txt\"\n+ reputation 4\nZulu\nAlpha\n! AdditionalCountry \"Two\"\n! reputation 9\nBeta\n* Cup\n+ reputation 20\n+ add 1 from \"One\"\nCup team\n",
        );
        assert!(warnings.is_empty(), "{warnings:?}");
        assert_eq!(countries.len(), 2);
        assert_eq!(countries[0].clubs, ["Zulu", "Alpha"]);
        assert_eq!(
            countries[0].name_files,
            ["first names.txt", "last names.txt"]
        );
        assert_eq!(countries[0].reputation, Some(4));
        assert_eq!(countries[1].reputation, Some(9));
        assert_eq!(countries[1].clubs, ["Beta"]);
    }

    #[test]
    fn invalid_country_does_not_append_clubs_to_previous_country() {
        let (countries, warnings) = parse_international_countries(
            "! AdditionalCountry \"Valid\"\nClub\n! AdditionalCountry Invalid\nWrong club\n",
        );
        assert_eq!(countries[0].clubs, ["Club"]);
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("riga 3"));
    }

    #[test]
    fn san_marino_is_available_and_italian_sources_stay_separate() {
        let world = load_world(Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap()).unwrap();
        let san_marino = world
            .international_countries
            .iter()
            .find(|country| country.name == "San Marino")
            .unwrap();
        assert_eq!(san_marino.clubs, ["Cailunge", "Domagnane", "Faetane"]);
        assert_eq!(san_marino.reputation, Some(4));
        assert!(
            !world
                .leagues
                .iter()
                .any(|league| league.country == "San Marino")
        );
        let italy = world
            .international_countries
            .iter()
            .find(|country| country.name == "Italy")
            .unwrap();
        assert_eq!(italy.clubs[0], "Inter");
        let domestic = world
            .leagues
            .iter()
            .find(|league| league.country == "Italy")
            .unwrap();
        assert_eq!(domestic.divisions[0].teams[0], "Inter");
        for (position, club) in italy.clubs.iter().enumerate() {
            assert_eq!(domestic.club_locations(club), vec![(0, position)]);
        }
    }
}
