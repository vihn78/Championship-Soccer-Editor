use crate::leagues::{Division, LeagueFile, decode_text, parse_league};

fn properties(division: &Division) -> [(&'static str, Option<u32>); 4] {
    [
        ("division", division.level),
        ("promotions", division.promotions),
        ("relegations", division.relegations),
        ("reputation", division.reputation),
    ]
}

fn appended_teams(
    league: &LeagueFile,
    original: &LeagueFile,
    source_index: usize,
    newline: &str,
) -> Result<String, String> {
    let index = league
        .division_sources
        .iter()
        .position(|source| *source == Some(source_index))
        .ok_or("Mappa delle divisioni non valida.")?;
    let previous = original
        .divisions
        .get(source_index)
        .ok_or("Divisione originale non trovata.")?;
    Ok(league.divisions[index]
        .teams
        .iter()
        .skip(previous.teams.len())
        .map(|team| format!("{team}{newline}"))
        .collect())
}

/// Mantiene le righe originali, sostituendo solo valori modificati e nomi nelle posizioni della rosa.
pub fn render(league: &LeagueFile) -> Result<Vec<u8>, String> {
    let source = decode_text(&league.source_bytes);
    let original = parse_league(&league.path, &source);
    if league.divisions.is_empty() || league.divisions.len() > 12 {
        return Err("Una lega deve contenere da 1 a 12 divisioni.".into());
    }
    if league.division_sources.len() != league.divisions.len() {
        return Err("Mappa delle divisioni non valida.".into());
    }
    let mut levels: Vec<_> = league
        .divisions
        .iter()
        .filter_map(|division| division.level)
        .collect();
    levels.sort();
    if levels != (1..=league.divisions.len() as u32).collect::<Vec<_>>() {
        return Err("I livelli devono essere consecutivi da 1, senza buchi.".into());
    }
    let mut all_teams = std::collections::HashSet::new();
    for (index, division) in league.divisions.iter().enumerate() {
        let issues = league.division_issues(index);
        if !issues.is_empty() {
            return Err(issues.join("\n"));
        }
        if !(3..=24).contains(&division.teams.len()) {
            return Err(format!("{}: servono da 3 a 24 squadre.", division.name));
        }
        if division.level.is_none() || division.level.is_some_and(|level| level > 12) {
            return Err(format!(
                "{}: il livello deve essere compreso tra 1 e 12.",
                division.name
            ));
        }
        if division.name.contains(['\r', '\n']) || division.name.trim() != division.name {
            return Err(
                "Il nome divisione non può contenere ritorni a capo o spazi iniziali/finali."
                    .into(),
            );
        }
        if division
            .promotions
            .unwrap_or(0)
            .saturating_add(division.relegations.unwrap_or(0))
            > division.teams.len() as u32
        {
            return Err(format!(
                "{}: promozioni e retrocessioni superano il numero di squadre.",
                division.name
            ));
        }
        if league.division_sources[index].is_some_and(|source| {
            original
                .divisions
                .get(source)
                .is_none_or(|old| division.teams.len() < old.teams.len())
        }) {
            return Err("La rimozione di squadre non è ancora supportata nel salvataggio.".into());
        }
        for team in &division.teams {
            if team.trim().is_empty()
                || team.contains(['\r', '\n'])
                || team.starts_with(['!', '+', ':', '*', '#', ';', '$', '@', '='])
            {
                return Err(format!("Nome club non valido: {team}"));
            }
            if !all_teams.insert(team) {
                return Err(format!("Club duplicato: {team}"));
            }
        }
    }
    let newline = if source.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    };
    let mut output = String::new();
    if league.source_bytes.is_empty() {
        crate::world_edit::validate_filename(&league.country)?;
        if league.name_files.len() != 2
            || league
                .name_files
                .iter()
                .any(|path| path.contains(['"', '\r', '\n']))
        {
            return Err("Servono due percorsi validi per gli archivi dei nomi.".into());
        }
        output = format!(
            "! Country \"{}\"{newline}! Names \"{}\" \"{}\"{newline}",
            league.country, league.name_files[0], league.name_files[1]
        );
    }
    let mut division_index = None;
    let mut next_division = 0;
    let mut team_index = 0;
    for raw in source.split_inclusive('\n') {
        let (body, ending) = if let Some(body) = raw.strip_suffix("\r\n") {
            (body, "\r\n")
        } else if let Some(body) = raw.strip_suffix('\n') {
            (body, "\n")
        } else {
            (raw, "")
        };
        let trimmed = body.trim();
        let mut replacement: Option<String> = None;
        let mut additions = String::new();
        let mut prefix = String::new();
        if (trimmed.starts_with(':') || trimmed.starts_with('*'))
            && let Some(source_index) = division_index
        {
            prefix = appended_teams(league, &original, source_index, newline)?;
        }
        if trimmed.starts_with(':') {
            let source_index = next_division;
            next_division += 1;
            division_index = Some(source_index);
            let Some(index) = league
                .division_sources
                .iter()
                .position(|source| *source == Some(source_index))
            else {
                continue;
            };
            team_index = 0;
            let changed = &league.divisions[index];
            let previous = &original.divisions[source_index];
            if changed.name != previous.name {
                replacement = Some(format!(": {}", changed.name));
            }
            for ((key, value), (_, old)) in
                properties(changed).into_iter().zip(properties(previous))
            {
                if old.is_none()
                    && let Some(value) = value
                {
                    additions.push_str(&format!("+ {key} {value}{newline}"));
                }
            }
        } else if trimmed.starts_with('*') {
            division_index = None;
        } else if let Some(source_index) = division_index {
            let Some(index) = league
                .division_sources
                .iter()
                .position(|source| *source == Some(source_index))
            else {
                continue;
            };
            if let Some(instruction) = trimmed.strip_prefix('+') {
                let key = instruction.split_whitespace().next().unwrap_or("");
                for ((field, value), (_, old)) in properties(&league.divisions[index])
                    .into_iter()
                    .zip(properties(&original.divisions[source_index]))
                {
                    if key.eq_ignore_ascii_case(field) && value != old {
                        let value = value.ok_or_else(|| {
                            format!("Rimozione del parametro {field} non supportata.")
                        })?;
                        replacement = Some(format!("+ {key} {value}"));
                    }
                }
            } else if !trimmed.is_empty() && !trimmed.starts_with(['!', '#', ';', '$', '@', '=']) {
                let team = league.divisions[index]
                    .teams
                    .get(team_index)
                    .ok_or("Impossibile associare una riga della rosa.")?;
                if team != trimmed {
                    replacement = Some(team.clone());
                }
                team_index += 1;
            }
        }
        output.push_str(&prefix);
        if let Some(replacement) = replacement {
            let leading = &body[..body.len() - body.trim_start().len()];
            let trailing = &body[body.trim_end().len()..];
            output.push_str(leading);
            output.push_str(&replacement);
            output.push_str(trailing);
            output.push_str(ending);
        } else {
            output.push_str(raw);
        }
        if !additions.is_empty() {
            if ending.is_empty() {
                output.push_str(newline);
            }
            output.push_str(&additions);
        }
    }
    if let Some(source_index) = division_index {
        let additions = appended_teams(league, &original, source_index, newline)?;
        if !additions.is_empty() {
            if !output.is_empty() && !output.ends_with('\n') {
                output.push_str(newline);
            }
            output.push_str(&additions);
        }
    }
    for (division, source_index) in league.divisions.iter().zip(&league.division_sources) {
        if source_index.is_some() {
            continue;
        }
        if !output.is_empty() && !output.ends_with('\n') {
            output.push_str(newline);
        }
        output.push_str(&format!(": {}{newline}", division.name));
        for (key, value) in properties(division) {
            if let Some(value) = value {
                output.push_str(&format!("+ {key} {value}{newline}"));
            }
        }
        for team in &division.teams {
            output.push_str(team);
            output.push_str(newline);
        }
    }
    if parse_league(&league.path, &output).divisions != league.divisions {
        return Err(
            "Verifica del file generato fallita: il contenuto non corrisponde alle modifiche."
                .into(),
        );
    }
    if !league.source_bytes.is_empty() && std::str::from_utf8(&league.source_bytes).is_ok() {
        let mut bytes = if league.source_bytes.starts_with(&[239, 187, 191]) {
            vec![239, 187, 191]
        } else {
            Vec::new()
        };
        bytes.extend_from_slice(output.as_bytes());
        Ok(bytes)
    } else {
        let (encoded, _, errors) = encoding_rs::WINDOWS_1252.encode(&output);
        if errors {
            return Err("Un nome contiene caratteri non rappresentabili in Windows-1252.".into());
        }
        Ok(encoded.into_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::leagues::parse_league;
    use std::path::Path;

    const SOURCE: &str = "; keep comment\r\n! Country \"Test\"\r\n* Cup\r\n+ round 1 \"Final\" 1-2 47\r\n: First\r\n+ division 1\r\n+ promotions 0\r\n+ relegations 0\r\n+ reputation 10\r\nZulu\r\n; between teams\r\nAlpha\r\nBeta";

    #[test]
    fn roundtrip_is_exact_and_edits_preserve_cups_comments_and_newlines() {
        let mut league = parse_league(Path::new("Test.txt"), SOURCE);
        assert_eq!(render(&league).unwrap(), SOURCE.as_bytes());
        league.divisions[0].name = "New division".into();
        league.divisions[0].reputation = Some(12);
        league.divisions[0].teams.swap(0, 2);
        let expected = SOURCE
            .replace(": First", ": New division")
            .replace("reputation 10", "reputation 12")
            .replace(
                "Zulu\r\n; between teams\r\nAlpha\r\nBeta",
                "Beta\r\n; between teams\r\nAlpha\r\nZulu",
            );
        assert_eq!(render(&league).unwrap(), expected.as_bytes());
    }

    #[test]
    fn keeps_legacy_encoding_and_rejects_unrepresentable_text() {
        let source = SOURCE.replace("First", "Première");
        let mut league = parse_league(Path::new("Test.txt"), &source);
        league.source_bytes = encoding_rs::WINDOWS_1252.encode(&source).0.into_owned();
        assert_eq!(render(&league).unwrap(), league.source_bytes);
        league.divisions[0].name = "日本".into();
        assert!(render(&league).is_err());
    }

    #[test]
    fn rejects_invalid_structure_and_duplicate_clubs() {
        let mut league = parse_league(Path::new("Test.txt"), SOURCE);
        league.divisions[0].name = "Bad\nInjected".into();
        assert!(render(&league).is_err());
        league.divisions[0].name = "Valid".into();
        league.divisions[0].teams[1] = "Zulu".into();
        assert!(render(&league).is_err());
    }

    #[test]
    fn appends_a_new_club_before_the_next_section() {
        let mut league = parse_league(Path::new("Test.txt"), SOURCE);
        league.divisions[0].teams.push("Gamma".into());
        let rendered = render(&league).unwrap();
        assert_eq!(
            parse_league(&league.path, &decode_text(&rendered)).divisions,
            league.divisions
        );
        assert!(decode_text(&rendered).ends_with("Beta\r\nGamma\r\n"));
    }

    #[test]
    fn adds_missing_property_and_preserves_utf8_bom() {
        let source = SOURCE.replace("+ reputation 10\r\n", "");
        let mut league = parse_league(Path::new("Test.txt"), &source);
        league.source_bytes = [vec![239, 187, 191], source.as_bytes().to_vec()].concat();
        league.divisions[0].reputation = Some(9);
        let bytes = render(&league).unwrap();
        assert!(bytes.starts_with(&[239, 187, 191]));
        assert_eq!(
            parse_league(&league.path, &decode_text(&bytes)).divisions,
            league.divisions
        );
    }

    #[test]
    fn italian_swap_roundtrip_preserves_original_file_bytes_outside_team_names() {
        let world =
            crate::leagues::load_world(Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap())
                .unwrap();
        let mut italy = world
            .leagues
            .into_iter()
            .find(|league| league.country == "Italy")
            .unwrap();
        let original_bytes = italy.source_bytes.clone();
        assert_eq!(render(&italy).unwrap(), original_bytes);
        italy.move_club(1, 0, true).unwrap();
        let updated = render(&italy).unwrap();
        assert_eq!(
            parse_league(&italy.path, &decode_text(&updated)).divisions,
            italy.divisions
        );
        // Una seconda mossa inversa deve riprodurre il file originale byte per byte.
        let last = italy.divisions[0].teams.len() - 1;
        italy.move_club(0, last, false).unwrap();
        assert_eq!(render(&italy).unwrap(), original_bytes);
    }
}
