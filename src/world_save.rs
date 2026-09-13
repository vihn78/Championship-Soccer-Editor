use crate::league_save;
use crate::leagues::{InternationalCountry, World, decode_text, parse_international_countries};
use std::io::Write;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct FileChange {
    path: PathBuf,
    before: Option<Vec<u8>>,
    after: Option<Vec<u8>>,
}

impl FileChange {
    fn replacement(path: PathBuf, before: Option<Vec<u8>>, after: Vec<u8>) -> Self {
        Self {
            path,
            before,
            after: Some(after),
        }
    }

    fn deletion(path: PathBuf, before: Vec<u8>) -> Self {
        Self {
            path,
            before: Some(before),
            after: None,
        }
    }
}

fn encode_like(source: &[u8], text: String) -> Result<Vec<u8>, String> {
    if std::str::from_utf8(source).is_ok() {
        let mut bytes = if source.starts_with(&[239, 187, 191]) {
            vec![239, 187, 191]
        } else {
            Vec::new()
        };
        bytes.extend_from_slice(text.as_bytes());
        Ok(bytes)
    } else {
        let (bytes, _, errors) = encoding_rs::WINDOWS_1252.encode(&text);
        if errors {
            return Err("Un nome internazionale non è rappresentabile in Windows-1252.".into());
        }
        Ok(bytes.into_owned())
    }
}

/// Sostituisce esclusivamente le righe-club dei blocchi AdditionalCountry.
fn render_international(
    source_bytes: &[u8],
    countries: &[InternationalCountry],
) -> Result<Vec<u8>, String> {
    let source = decode_text(source_bytes);
    let (original, warnings) = parse_international_countries(&source);
    if !warnings.is_empty() || original.len() != countries.len() {
        return Err(
            "Il file internazionale è cambiato o non può essere sincronizzato in sicurezza.".into(),
        );
    }
    for (old, new) in original.iter().zip(countries) {
        if !old.name.eq_ignore_ascii_case(&new.name) || old.clubs.len() != new.clubs.len() {
            return Err("La struttura dei paesi internazionali non può essere modificata da questa schermata.".into());
        }
    }
    let mut output = String::new();
    let mut current = None;
    let mut next_country = 0usize;
    let mut club_positions = vec![0usize; countries.len()];
    for raw in source.split_inclusive('\n') {
        let (body, ending) = if let Some(body) = raw.strip_suffix("\r\n") {
            (body, "\r\n")
        } else if let Some(body) = raw.strip_suffix('\n') {
            (body, "\n")
        } else {
            (raw, "")
        };
        let trimmed = body.trim();
        if trimmed.starts_with("! AdditionalCountry") {
            current = (next_country < countries.len()).then_some(next_country);
            next_country += 1;
            output.push_str(raw);
            continue;
        }
        if trimmed.starts_with('*') {
            current = None;
        }
        if let Some(country_index) = current
            && !trimmed.is_empty()
            && !trimmed.starts_with(['!', '+', '#', ';', '$', '@', '='])
        {
            let club_index = club_positions[country_index];
            let club = countries[country_index]
                .clubs
                .get(club_index)
                .ok_or("Elenco club internazionale non coerente.")?;
            club_positions[country_index] += 1;
            let leading = &body[..body.len() - body.trim_start().len()];
            let trailing = &body[body.trim_end().len()..];
            output.push_str(leading);
            output.push_str(club);
            output.push_str(trailing);
            output.push_str(ending);
        } else {
            output.push_str(raw);
        }
    }
    if club_positions
        .iter()
        .zip(countries)
        .any(|(position, country)| *position != country.clubs.len())
    {
        return Err("Non tutte le righe-club internazionali sono state trovate.".into());
    }
    let (check, warnings) = parse_international_countries(&output);
    if !warnings.is_empty() || check != countries {
        return Err("Verifica del file internazionale generato fallita.".into());
    }
    encode_like(source_bytes, output)
}

pub fn prepare(world: &World, saved: &World) -> Result<Vec<FileChange>, String> {
    let mut changes = Vec::new();
    for league in &world.leagues {
        let old = saved.leagues.iter().find(|item| item.path == league.path);
        if old.is_none_or(|item| {
            item.divisions != league.divisions || item.division_sources != league.division_sources
        }) {
            let before = old.map(|item| item.source_bytes.clone());
            if let Some(bytes) = &before
                && std::fs::read(&league.path).map_err(|error| error.to_string())? != *bytes
            {
                return Err(format!(
                    "{} è cambiato esternamente. Riapri il mondo prima di salvare.",
                    league.path.display()
                ));
            }
            changes.push(FileChange::replacement(
                league.path.clone(),
                before,
                league_save::render(league)?,
            ));
        }
    }
    for old in &saved.leagues {
        if !world.leagues.iter().any(|league| league.path == old.path) {
            if std::fs::read(&old.path).map_err(|error| error.to_string())? != old.source_bytes {
                return Err(format!(
                    "{} è cambiato esternamente. Riapri il mondo prima di salvare.",
                    old.path.display()
                ));
            }
            changes.push(FileChange::deletion(
                old.path.clone(),
                old.source_bytes.clone(),
            ));
        }
    }
    if world.international_countries != saved.international_countries {
        let path = world
            .international_file
            .as_ref()
            .ok_or("File internazionale non disponibile.")?;
        if std::fs::read(path).map_err(|error| error.to_string())? != world.international_bytes {
            return Err(
                "Il file internazionale è cambiato esternamente. Riapri il mondo prima di salvare."
                    .into(),
            );
        }
        changes.push(FileChange::replacement(
            path.clone(),
            Some(world.international_bytes.clone()),
            render_international(&world.international_bytes, &world.international_countries)?,
        ));
    }
    let teams = world
        .league_directory
        .parent()
        .ok_or("Cartella Data non disponibile.")?
        .join("Team");
    for club in &world.deleted_clubs {
        if world.is_referenced(club) {
            continue;
        }
        let path = teams.join(format!("{club}.txt"));
        if path.exists() {
            changes.push(FileChange::deletion(
                path.clone(),
                std::fs::read(path).map_err(|error| error.to_string())?,
            ));
        }
    }
    Ok(changes)
}

fn write_temporary(staging: &Path, contents: &[u8]) -> Result<PathBuf, String> {
    let id = format!(
        ".world-editor-{}-{}.tmp",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|error| error.to_string())?
            .as_nanos()
    );
    let path = staging.join(id);
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .map_err(|error| error.to_string())?;
    file.write_all(contents)
        .and_then(|_| file.sync_all())
        .map_err(|error| error.to_string())?;
    Ok(path)
}

pub fn commit(changes: &[FileChange], staging: &Path) -> Result<(), String> {
    for change in changes {
        match &change.before {
            Some(before)
                if std::fs::read(&change.path).map_err(|error| error.to_string())? != *before =>
            {
                return Err(format!(
                    "{} è cambiato esternamente; nessun file è stato salvato.",
                    change.path.display()
                ));
            }
            None if change.path.exists() => {
                return Err(format!(
                    "{} esiste già; nessun file è stato salvato.",
                    change.path.display()
                ));
            }
            _ => {}
        }
    }
    let mut applied: Vec<&FileChange> = Vec::new();
    for change in changes {
        let result = match &change.after {
            Some(bytes) => {
                let temporary = write_temporary(staging, bytes)?;
                let result =
                    std::fs::rename(&temporary, &change.path).map_err(|error| error.to_string());
                if result.is_err() {
                    let _ = std::fs::remove_file(temporary);
                }
                result
            }
            None => std::fs::remove_file(&change.path).map_err(|error| error.to_string()),
        };
        if let Err(error) = result {
            for previous in applied.into_iter().rev() {
                if let Some(bytes) = &previous.before {
                    let _ = std::fs::write(&previous.path, bytes);
                } else {
                    let _ = std::fs::remove_file(&previous.path);
                }
            }
            return Err(format!("Salvataggio interrotto: {error}"));
        }
        applied.push(change);
    }
    Ok(())
}

pub fn mark_saved(world: &mut World, _changes: &[FileChange]) {
    for league in &mut world.leagues {
        if let Ok(bytes) = std::fs::read(&league.path) {
            let mut parsed = crate::leagues::parse_league(&league.path, &decode_text(&bytes));
            parsed.source_bytes = bytes;
            *league = parsed;
        }
    }
    if let Some(path) = &world.international_file
        && let Ok(bytes) = std::fs::read(path)
    {
        let (countries, _) = parse_international_countries(&decode_text(&bytes));
        world.international_bytes = bytes;
        world.international_countries = countries;
    }
    world.retained_clubs.clear();
    world.deleted_clubs.clear();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sync_keeps_international_instructions_and_replaces_only_clubs() {
        let source = "! AdditionalCountry \"Test\"\r\n! TempNames \"a\" \"b\"\r\n+ reputation 4\r\nOld one\r\nOld two\r\n* Cup\r\n+ round 1 \"Final\"\r\n";
        let (mut countries, _) = parse_international_countries(source);
        countries[0].clubs = vec!["New one".into(), "New two".into()];
        let rendered = render_international(source.as_bytes(), &countries).unwrap();
        assert_eq!(
            String::from_utf8(rendered).unwrap(),
            source
                .replace("Old one", "New one")
                .replace("Old two", "New two")
        );
    }
}
