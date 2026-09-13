use crate::leagues::{Division, LeagueFile, World};

pub fn validate_filename(name: &str) -> Result<(), String> {
    if name.is_empty()
        || name.trim() != name
        || name.ends_with('.')
        || name
            .chars()
            .any(|ch| ch.is_control() || "<>:\"/\\|?*".contains(ch))
    {
        return Err(format!("Nome non valido per un file: {name}"));
    }
    let stem = name.split('.').next().unwrap_or("").to_ascii_uppercase();
    if [
        "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
        "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
    ]
    .contains(&stem.as_str())
    {
        return Err("Nome riservato da Windows.".into());
    }
    Ok(())
}

fn validate_division(name: &str, teams: &[String]) -> Result<(), String> {
    if name.trim().is_empty() || name.contains(['\r', '\n']) {
        return Err("Inserisci il nome della divisione.".into());
    }
    if !(3..=24).contains(&teams.len()) {
        return Err("Ogni divisione deve contenere da 3 a 24 club.".into());
    }
    let mut seen = std::collections::HashSet::new();
    for team in teams {
        validate_filename(team)?;
        if team.starts_with(['!', '+', ':', '*', '#', ';', '$', '@', '='])
            || !seen.insert(team.to_lowercase())
        {
            return Err(format!("Club duplicato o non valido: {team}"));
        }
    }
    Ok(())
}

impl World {
    /// Club con un file Team esistente, ma senza alcun riferimento nel mondo aperto.
    pub fn neutral_clubs(&self) -> Vec<String> {
        let team_directory = self
            .league_directory
            .parent()
            .map(|directory| directory.join("Team"));
        let Some(team_directory) = team_directory else {
            return Vec::new();
        };
        let countries: std::collections::HashSet<_> = self
            .international_countries
            .iter()
            .map(|country| country.name.to_lowercase())
            .chain(
                self.leagues
                    .iter()
                    .map(|league| league.country.to_lowercase()),
            )
            .collect();
        let assigned: std::collections::HashSet<_> = self
            .international_countries
            .iter()
            .flat_map(|country| &country.clubs)
            .chain(
                self.leagues
                    .iter()
                    .flat_map(|league| league.divisions.iter())
                    .flat_map(|division| &division.teams),
            )
            .map(|club| club.to_lowercase())
            .collect();
        let mut clubs = std::fs::read_dir(team_directory)
            .into_iter()
            .flatten()
            .flatten()
            .filter_map(|entry| {
                let path = entry.path();
                (path.is_file()
                    && path
                        .extension()
                        .is_some_and(|extension| extension.eq_ignore_ascii_case("txt")))
                .then(|| path.file_stem()?.to_str().map(str::to_owned))
                .flatten()
            })
            .filter(|club| {
                !countries.contains(&club.to_lowercase())
                    && !assigned.contains(&club.to_lowercase())
            })
            .collect::<Vec<_>>();
        clubs.sort_by_key(|club| club.to_lowercase());
        clubs
    }

    pub fn add_club_to_division(
        &mut self,
        league_index: usize,
        division_index: usize,
        club: &str,
    ) -> Result<(), String> {
        validate_filename(club)?;
        let league = self
            .leagues
            .get_mut(league_index)
            .ok_or("Lega non trovata")?;
        if league
            .divisions
            .iter()
            .flat_map(|division| &division.teams)
            .any(|team| team.eq_ignore_ascii_case(club))
        {
            return Err("Il club è già assegnato a una divisione di questo paese.".into());
        }
        let division = league
            .divisions
            .get_mut(division_index)
            .ok_or("Divisione non trovata")?;
        if division.teams.len() >= 24 {
            return Err("Una divisione può contenere al massimo 24 club.".into());
        }
        division.teams.push(club.to_owned());
        Ok(())
    }

    pub fn sync_international(&mut self, league_index: usize) -> Result<(), String> {
        let league = self.leagues.get(league_index).ok_or("Lega non trovata")?;
        let matching: Vec<_> = self
            .international_countries
            .iter()
            .enumerate()
            .filter(|(_, country)| country.name.eq_ignore_ascii_case(&league.country))
            .map(|(index, _)| index)
            .collect();
        if matching.len() > 1 {
            return Err(
                "Paese duplicato nel file internazionale: sincronizzazione ambigua.".into(),
            );
        }
        let Some(index) = matching.first().copied() else {
            return Ok(());
        };
        let top: Vec<_> = league
            .divisions
            .iter()
            .filter(|division| division.level == Some(1))
            .collect();
        if top.len() != 1 {
            return Err("La sincronizzazione richiede una sola divisione di livello 1.".into());
        }
        let count = self.international_countries[index].clubs.len();
        if top[0].teams.len() < count {
            return Err(format!(
                "{}: servono almeno {count} club in prima divisione per l'elenco internazionale.",
                league.country
            ));
        }
        self.international_countries[index].clubs = top[0].teams[..count].to_vec();
        Ok(())
    }

    pub fn create_league(
        &mut self,
        country: &str,
        division_name: &str,
        extra_teams: Vec<String>,
    ) -> Result<usize, String> {
        validate_filename(country)?;
        if country.eq_ignore_ascii_case("International teams and tournaments")
            || self.leagues.iter().any(|league| {
                league.country.eq_ignore_ascii_case(country)
                    || league
                        .path
                        .file_stem()
                        .is_some_and(|stem| stem.eq_ignore_ascii_case(country))
            })
        {
            return Err("Esiste già una lega per questo paese.".into());
        }
        let international = self
            .international_countries
            .iter()
            .find(|item| item.name.eq_ignore_ascii_case(country));
        let mut teams = international.map_or_else(Vec::new, |item| item.clubs.clone());
        teams.extend(extra_teams);
        validate_division(division_name, &teams)?;
        let name_files = international
            .filter(|item| item.name_files.len() == 2)
            .map(|item| item.name_files.clone())
            .unwrap_or_else(|| {
                vec![
                    format!("data/names/{country}_1st.txt"),
                    format!("data/names/{country}_2nd.txt"),
                ]
            });
        let path = self.league_directory.join(format!("{country}.txt"));
        if path.exists() {
            return Err("Il file della lega esiste già su disco.".into());
        }
        let league = LeagueFile {
            path,
            country: country.into(),
            name_files,
            division_sources: vec![None],
            divisions: vec![Division {
                name: division_name.trim().into(),
                level: Some(1),
                promotions: Some(0),
                relegations: Some(0),
                reputation: Some(international.and_then(|item| item.reputation).unwrap_or(5)),
                teams,
            }],
            ..Default::default()
        };
        self.leagues.push(league);
        Ok(self.leagues.len() - 1)
    }

    pub fn add_division(
        &mut self,
        league_index: usize,
        name: &str,
        teams: Vec<String>,
    ) -> Result<(), String> {
        validate_division(name, &teams)?;
        let league = self
            .leagues
            .get_mut(league_index)
            .ok_or("Lega non trovata")?;
        let count = league.divisions.len();
        if count >= 12 {
            return Err("Limite di 12 divisioni raggiunto.".into());
        }
        let mut levels: Vec<_> = league
            .divisions
            .iter()
            .filter_map(|item| item.level)
            .collect();
        levels.sort();
        if levels != (1..=count as u32).collect::<Vec<_>>() {
            return Err(
                "Correggi i livelli: devono essere consecutivi prima di aggiungere divisioni."
                    .into(),
            );
        }
        if teams.iter().any(|team| {
            league.divisions.iter().any(|division| {
                division
                    .teams
                    .iter()
                    .any(|other| other.eq_ignore_ascii_case(team))
            })
        }) {
            return Err("Un club è già assegnato a una divisione di questo paese.".into());
        }
        if league
            .divisions
            .iter()
            .any(|division| division.name.eq_ignore_ascii_case(name.trim()))
        {
            return Err("Nome divisione già usato.".into());
        }
        // La nuova ultima divisione eredita il numero di promozioni che la
        // precedente ultima divisione già richiedeva come retrocessioni.
        let promotions = league
            .divisions
            .iter()
            .max_by_key(|division| division.level)
            .and_then(|division| division.relegations)
            .unwrap_or(0);
        league.divisions.push(Division {
            name: name.trim().into(),
            level: Some(count as u32 + 1),
            promotions: Some(promotions),
            relegations: Some(0),
            reputation: Some(1),
            teams,
        });
        league.division_sources.push(None);
        Ok(())
    }

    pub fn can_remove_division(&self, league_index: usize, index: usize) -> bool {
        self.leagues.get(league_index).is_some_and(|league| {
            let mut levels: Vec<_> = league
                .divisions
                .iter()
                .filter_map(|item| item.level)
                .collect();
            levels.sort();
            league.divisions.len() > 1
                && levels == (1..=league.divisions.len() as u32).collect::<Vec<_>>()
                && league
                    .divisions
                    .get(index)
                    .is_some_and(|division| division.level == Some(league.divisions.len() as u32))
        })
    }

    pub fn is_referenced(&self, club: &str) -> bool {
        self.international_countries.iter().any(|country| {
            country
                .clubs
                .iter()
                .any(|name| name.eq_ignore_ascii_case(club))
        }) || self.leagues.iter().any(|league| {
            league.divisions.iter().any(|division| {
                division
                    .teams
                    .iter()
                    .any(|name| name.eq_ignore_ascii_case(club))
            })
        })
    }

    fn release_clubs(&mut self, clubs: Vec<String>, delete: bool) {
        for club in clubs {
            if delete && !self.is_referenced(&club) {
                self.retained_clubs.remove(&club);
                self.deleted_clubs.insert(club);
            } else {
                self.deleted_clubs.remove(&club);
                self.retained_clubs.insert(club);
            }
        }
    }

    pub fn remove_division(
        &mut self,
        league_index: usize,
        index: usize,
        delete_clubs: bool,
    ) -> Result<(), String> {
        if !self.can_remove_division(league_index, index) {
            return Err("Puoi rimuovere soltanto l'ultima divisione, mai la prima.".into());
        }
        let league = &mut self.leagues[league_index];
        let removed = league.divisions.remove(index);
        league.division_sources.remove(index);
        if let Some(last) = league
            .divisions
            .iter_mut()
            .max_by_key(|division| division.level)
        {
            last.relegations = Some(0);
        }
        self.release_clubs(removed.teams, delete_clubs);
        Ok(())
    }

    pub fn remove_league(&mut self, index: usize, delete_clubs: bool) -> Result<(), String> {
        if index >= self.leagues.len() {
            return Err("Lega non trovata".into());
        }
        let league = self.leagues.remove(index);
        self.release_clubs(
            league
                .divisions
                .into_iter()
                .flat_map(|division| division.teams)
                .collect(),
            delete_clubs,
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::leagues::load_world;
    use std::path::Path;

    #[test]
    fn creates_san_marino_syncs_top_three_and_removes_only_last_level() {
        let mut world =
            load_world(Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap()).unwrap();
        let index = world
            .create_league("San Marino", "Campionato", Vec::new())
            .unwrap();
        assert_eq!(
            world.leagues[index].divisions[0].teams,
            ["Cailunge", "Domagnane", "Faetane"]
        );
        world
            .add_division(
                index,
                "Seconda",
                vec!["New A".into(), "New B".into(), "New C".into()],
            )
            .unwrap();
        world.add_club_to_division(index, 0, "Nuovo club").unwrap();
        assert!(world.add_club_to_division(index, 0, "Nuovo club").is_err());
        world.leagues[index].swap_clubs((0, 0), (1, 0));
        world.sync_international(index).unwrap();
        assert_eq!(
            world
                .international_countries
                .iter()
                .find(|country| country.name == "San Marino")
                .unwrap()
                .clubs,
            ["New A", "Domagnane", "Faetane"]
        );
        assert!(world.remove_division(index, 0, true).is_err());
        world.remove_division(index, 1, true).unwrap();
        assert_eq!(world.leagues[index].divisions[0].relegations, Some(0));
        assert!(world.remove_division(index, 0, true).is_err());
        world.remove_league(index, true).unwrap();
        assert!(!world.deleted_clubs.contains("New A"));
        assert!(world.deleted_clubs.contains("Cailunge"));
    }
}
