use eframe::egui;
mod country_display;
mod league_dialogs;
mod league_save;
mod leagues;
mod teams;
mod world_edit;
mod world_save;

const DEFAULT_FONT_SIZE: f32 = 20.0;
const FONT_SIZE_KEY: &str = "editor_font_size";
const TABS: [(&str, &str); 6] = [
    ("Nations", "Paesi, nazionalità e archivi dei nomi"),
    ("Leagues", "Divisioni, squadre, promozioni e retrocessioni"),
    ("Cups", "Partecipanti, turni e calendario delle coppe"),
    ("Teams", "Club, divise e rose"),
    (
        "Players",
        "Anagrafica, ruoli e caratteristiche dei giocatori",
    ),
    ("Transfers", "Trasferimenti tra due rose"),
];

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1600.0, 1000.0])
            .with_min_inner_size([900.0, 600.0])
            .with_maximized(true),
        ..Default::default()
    };
    eframe::run_native(
        "Championship Soccer World Editor",
        options,
        Box::new(|context| Ok(Box::new(WorldEditor::new(context)))),
    )
}

struct WorldEditor {
    flags: std::collections::HashMap<String, egui::TextureHandle>,
    kit_textures: std::collections::HashMap<String, egui::TextureHandle>,
    selected_tab: usize,
    font_size: f32,
    font_name: &'static str,
    world: Option<leagues::World>,
    selected_league: Option<usize>,
    selected_division: usize,
    selected_club: Option<(usize, usize, usize)>,
    swap_source: Option<(usize, usize, usize)>,
    saved_world: Option<leagues::World>,
    creation: Option<league_dialogs::Creation>,
    removal: Option<league_dialogs::Removal>,
    club_addition: Option<league_dialogs::ClubAddition>,
    confirm_discard: bool,
    selected_international_country: Option<usize>,
    league_search: String,
    team_search: String,
    selected_team: Option<String>,
    selected_kit: usize,
    team_edits: std::collections::HashMap<std::path::PathBuf, (String, teams::TeamProfile)>,
    open_error: Option<String>,
    save_message: Option<String>,
}

impl WorldEditor {
    fn new(context: &eframe::CreationContext<'_>) -> Self {
        let font_size = context
            .storage
            .and_then(|storage| storage.get_string(FONT_SIZE_KEY))
            .and_then(|value| value.parse::<f32>().ok())
            .filter(|value| value.is_finite())
            .unwrap_or(DEFAULT_FONT_SIZE)
            .clamp(14.0, 32.0);
        let mut fonts = egui::FontDefinitions::default();
        // Usiamo il font installato senza redistribuire il file di Windows.
        let windows_directory = std::env::var_os("WINDIR").unwrap_or_else(|| "C:\\Windows".into());
        let font_path = std::path::PathBuf::from(windows_directory).join("Fonts/consola.ttf");
        let font_name = if let Ok(bytes) = std::fs::read(font_path) {
            fonts
                .font_data
                .insert("Consolas".into(), egui::FontData::from_owned(bytes).into());
            fonts
                .families
                .entry(egui::FontFamily::Monospace)
                .or_default()
                .insert(0, "Consolas".into());
            "Consolas"
        } else {
            "Monospace"
        };
        context.egui_ctx.set_fonts(fonts);
        // Il tema del sistema non deve sostituire font o colori dell'editor.
        context.egui_ctx.set_theme(egui::Theme::Dark);
        let editor = Self {
            flags: country_display::load_flags(&context.egui_ctx),
            kit_textures: Self::load_kit_textures(&context.egui_ctx),
            selected_tab: 0,
            font_size,
            font_name,
            world: None,
            selected_league: None,
            selected_division: 0,
            selected_club: None,
            swap_source: None,
            saved_world: None,
            creation: None,
            removal: None,
            club_addition: None,
            confirm_discard: false,
            selected_international_country: None,
            league_search: String::new(),
            team_search: String::new(),
            selected_team: None,
            selected_kit: 0,
            team_edits: Default::default(),
            open_error: None,
            save_message: None,
        };
        editor.apply_font_size(&context.egui_ctx);
        editor
    }

    fn load_kit_textures(
        context: &egui::Context,
    ) -> std::collections::HashMap<String, egui::TextureHandle> {
        let mut textures = std::collections::HashMap::new();
        let directory = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("kits");
        for name in [
            "background",
            "shirt",
            "stripes",
            "sleeves",
            "shorts",
            "socks",
        ] {
            let path = directory.join(format!("{name}.png"));
            if let Ok(image) = image::open(path) {
                let rgba = image.to_rgba8();
                let pixels = egui::ColorImage::from_rgba_unmultiplied(
                    [rgba.width() as usize, rgba.height() as usize],
                    rgba.as_raw(),
                );
                textures.insert(
                    name.into(),
                    context.load_texture(
                        format!("kit_{name}"),
                        pixels,
                        egui::TextureOptions::LINEAR,
                    ),
                );
            }
        }
        textures
    }

    fn apply_font_size(&self, context: &egui::Context) {
        context.all_styles_mut(|style| {
            for (text_style, size) in [
                (egui::TextStyle::Heading, self.font_size + 8.0),
                (egui::TextStyle::Body, self.font_size),
                (egui::TextStyle::Button, self.font_size),
                (egui::TextStyle::Monospace, self.font_size),
                (egui::TextStyle::Small, self.font_size - 2.0),
            ] {
                style
                    .text_styles
                    .insert(text_style, egui::FontId::monospace(size));
            }
            // Cambia solo il corpo del testo: margini e padding rimangono costanti.
            style.spacing.item_spacing = egui::vec2(14.0, 8.0);
            style.spacing.button_padding = egui::vec2(14.0, 8.0);
            style.spacing.interact_size.y = 36.0;
            let white = egui::Color32::from_rgb(245, 245, 245);
            let mut visuals = egui::Visuals::dark();
            visuals.override_text_color = Some(white);
            visuals.panel_fill = egui::Color32::from_rgb(12, 20, 38);
            visuals.window_fill = egui::Color32::from_rgb(17, 28, 49);
            visuals.extreme_bg_color = egui::Color32::from_rgb(8, 15, 29);
            visuals.faint_bg_color = egui::Color32::from_rgb(21, 34, 56);
            visuals.selection.bg_fill = egui::Color32::from_rgb(42, 70, 110);
            visuals.selection.stroke = egui::Stroke::new(1.0, white);
            for (widget, fill) in [
                (
                    &mut visuals.widgets.noninteractive,
                    egui::Color32::from_rgb(17, 28, 49),
                ),
                (
                    &mut visuals.widgets.inactive,
                    egui::Color32::from_rgb(24, 39, 62),
                ),
                (
                    &mut visuals.widgets.hovered,
                    egui::Color32::from_rgb(36, 57, 86),
                ),
                (
                    &mut visuals.widgets.active,
                    egui::Color32::from_rgb(42, 70, 110),
                ),
                (
                    &mut visuals.widgets.open,
                    egui::Color32::from_rgb(29, 47, 74),
                ),
            ] {
                widget.bg_fill = fill;
                widget.weak_bg_fill = fill;
                widget.fg_stroke.color = white;
            }
            style.visuals = visuals;
        });
    }

    fn show_menu(&mut self, ui: &mut egui::Ui) {
        egui::MenuBar::new().ui(ui, |ui| {
            ui.menu_button("File", |ui| {
                if ui
                    .add_enabled(!self.has_changes(), egui::Button::new("Apri mondo…"))
                    .on_disabled_hover_text("Salva o scarta prima le modifiche.")
                    .clicked()
                {
                    ui.close();
                    if let Some(path) = rfd::FileDialog::new()
                        .set_title("Seleziona la cartella del gioco, del pacchetto o Data")
                        .pick_folder()
                    {
                        // Sostituiamo il mondo solo dopo un caricamento riuscito.
                        match leagues::load_world(&path) {
                            Ok(world) => {
                                self.selected_league = (!world.leagues.is_empty()).then_some(0);
                                self.selected_international_country = None;
                                self.saved_world = Some(world.clone());
                                self.creation = None;
                                self.removal = None;
                                self.club_addition = None;
                                self.selected_division = 0;
                                self.selected_club = None;
                                self.swap_source = None;
                                self.world = Some(world);
                                self.league_search.clear();
                                self.team_search.clear();
                                self.selected_team = None;
                                self.selected_kit = 0;
                                self.team_edits.clear();
                                self.open_error = None;
                                self.save_message = None;
                                self.selected_tab = 1;
                            }
                            Err(error) => self.open_error = Some(error),
                        }
                    }
                }
                if ui
                    .add_enabled(self.has_changes(), egui::Button::new("Salva"))
                    .clicked()
                {
                    self.save_changes();
                    ui.close();
                }
                ui.add_enabled(false, egui::Button::new("Esporta…"))
                    .on_disabled_hover_text("Disponibile in un prossimo micro-step");
                ui.separator();
                if ui.button("Esci").clicked() {
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                }
            });
            ui.menu_button("Impostazioni", |ui| {
                ui.label(format!("Font: {}", self.font_name));
                let mut changed = ui
                    .add(
                        egui::Slider::new(&mut self.font_size, 14.0..=32.0)
                            .step_by(1.0)
                            .text("Dimensione testo"),
                    )
                    .changed();
                if ui.button("Ripristina 20 punti").clicked() {
                    self.font_size = DEFAULT_FONT_SIZE;
                    changed = true;
                }
                if changed {
                    self.apply_font_size(ui.ctx());
                }
                ui.label("La preferenza viene conservata tra gli avvii.");
            });
        });
    }

    fn show_empty_roster(ui: &mut egui::Ui, heading: &str) {
        ui.heading(heading);
        ui.separator();
        ui.add_enabled(false, egui::Button::new("Seleziona squadra…"));
        ui.add_space(24.0);
        ui.label("Nessuna rosa caricata");
    }

    fn show_league_list(&mut self, ui: &mut egui::Ui) {
        let Some(world) = &self.world else {
            ui.label("Usa File → Apri mondo per caricare i campionati.");
            return;
        };
        let query = self.league_search.to_lowercase();
        let mut matches = 0;
        let league_name = |league: &leagues::LeagueFile| {
            world
                .country_names
                .name(
                    &league.country,
                    league
                        .path
                        .file_stem()
                        .and_then(|name| name.to_str())
                        .unwrap_or(&league.country),
                )
                .to_owned()
        };
        let mut sorted_leagues: Vec<_> = world.leagues.iter().enumerate().collect();
        sorted_leagues.sort_by_cached_key(|(_, league)| league_name(league).to_lowercase());
        for (index, league) in sorted_leagues {
            let label = league_name(league);
            let filename = league
                .path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy();
            if !label.to_lowercase().contains(&query)
                && !league.country.to_lowercase().contains(&query)
                && !filename.to_lowercase().contains(&query)
            {
                continue;
            }
            matches += 1;
            if country_display::flag_label(
                ui,
                &self.flags,
                league
                    .path
                    .file_stem()
                    .and_then(|name| name.to_str())
                    .unwrap_or(&league.country),
                &label,
                self.selected_league == Some(index),
            )
            .on_hover_text(league.path.display().to_string())
            .clicked()
            {
                self.selected_league = Some(index);
                self.selected_division = 0;
                self.selected_international_country = None;
            }
        }
        ui.separator();
        ui.strong("Paesi senza campionato");
        // Ordiniamo i riferimenti della vista, conservando gli indici e l'ordine dei dati.
        let mut countries_alphabetically: Vec<_> =
            world.international_countries.iter().enumerate().collect();
        countries_alphabetically.sort_by_cached_key(|(_, country)| {
            world
                .country_names
                .name(&country.name, &country.name)
                .to_lowercase()
        });
        for (index, country) in countries_alphabetically {
            let label = world.country_names.name(&country.name, &country.name);
            if world
                .leagues
                .iter()
                .any(|league| league.country.eq_ignore_ascii_case(&country.name))
                || (!label.to_lowercase().contains(&query)
                    && !country.name.to_lowercase().contains(&query))
            {
                continue;
            }
            matches += 1;
            if country_display::flag_label(
                ui,
                &self.flags,
                &country.name,
                label,
                self.selected_international_country == Some(index),
            )
            .clicked()
            {
                self.selected_international_country = Some(index);
                self.selected_league = None;
            }
        }
        if matches == 0 {
            ui.label("Nessun campionato corrispondente.");
        }
    }

    fn show_league_details(&self, ui: &mut egui::Ui) {
        let Some(world) = &self.world else {
            ui.label("Apri la cartella del gioco o di un pacchetto da File → Apri mondo.");
            return;
        };
        if !world.warnings.is_empty() {
            egui::CollapsingHeader::new(format!(
                "Segnalazioni del mondo ({})",
                world.warnings.len()
            ))
            .show(ui, |ui| {
                for warning in &world.warnings {
                    ui.label(warning);
                }
            });
        }
        if let Some(country) = self
            .selected_international_country
            .and_then(|index| world.international_countries.get(index))
        {
            let label = world.country_names.name(&country.name, &country.name);
            country_display::flag_label(ui, &self.flags, &country.name, label, false);
            ui.label("Nessun campionato presente nel mondo aperto.");
            ui.label("Questi club saranno il punto di partenza per la nuova lega.");
            Self::show_international_clubs(ui, country, None);
            return;
        }
        let Some(league) = self
            .selected_league
            .and_then(|index| world.leagues.get(index))
        else {
            ui.label("Seleziona un campionato dall'elenco.");
            return;
        };
        let identity = league
            .path
            .file_stem()
            .and_then(|name| name.to_str())
            .unwrap_or(&league.country);
        let label = world.country_names.name(&league.country, identity);
        country_display::flag_label(ui, &self.flags, identity, label, false);
        ui.label(league.path.display().to_string());
        ui.label(format!(
            "{} divisioni • Modifica in memoria",
            league.divisions.len()
        ));
        if let Some(country) = world
            .international_countries
            .iter()
            .find(|country| country.name.eq_ignore_ascii_case(&league.country))
        {
            egui::CollapsingHeader::new("Club richiamati dal file internazionale")
                .id_salt(("international_clubs", &league.path))
                .show(ui, |ui| {
                    ui.label("Usati quando il campionato non è selezionato nella carriera.");
                    ui.label("I nomi corrispondenti richiamano gli stessi club del campionato.");
                    Self::show_international_clubs(ui, country, Some(league));
                });
        }
    }

    fn show_team_league_list(&mut self, ui: &mut egui::Ui) {
        let Some(world) = &self.world else {
            ui.label("Usa File → Apri mondo per caricare le leghe.");
            return;
        };
        let mut leagues: Vec<_> = world.leagues.iter().enumerate().collect();
        leagues.sort_by_key(|(_, league)| league.country.to_lowercase());
        for (index, league) in leagues {
            if ui
                .selectable_label(self.selected_league == Some(index), &league.country)
                .clicked()
            {
                self.selected_league = Some(index);
                self.selected_international_country = None;
                self.selected_team = None;
                self.team_search.clear();
            }
        }
        ui.separator();
        ui.strong("Paesi senza campionato");
        let mut countries: Vec<_> = world.international_countries.iter().enumerate().collect();
        countries.sort_by_key(|(_, country)| country.name.to_lowercase());
        for (index, country) in countries {
            if world
                .leagues
                .iter()
                .any(|league| league.country.eq_ignore_ascii_case(&country.name))
            {
                continue;
            }
            if ui
                .selectable_label(
                    self.selected_league.is_none()
                        && self.selected_international_country == Some(index),
                    &country.name,
                )
                .clicked()
            {
                self.selected_league = None;
                self.selected_international_country = Some(index);
                self.selected_team = None;
                self.team_search.clear();
            }
        }
    }

    fn show_team_list(&mut self, ui: &mut egui::Ui) {
        let Some(world) = &self.world else {
            ui.label("Apri un mondo.");
            return;
        };
        let teams: Vec<_> = if let Some(league) = self
            .selected_league
            .and_then(|index| world.leagues.get(index))
        {
            league
                .divisions
                .iter()
                .flat_map(|division| division.teams.iter())
                .collect()
        } else if let Some(country) = self
            .selected_international_country
            .and_then(|index| world.international_countries.get(index))
        {
            country.clubs.iter().collect()
        } else {
            ui.label("Seleziona una lega.");
            return;
        };
        let query = self.team_search.to_lowercase();
        let mut teams = teams;
        teams.sort_by_key(|team| team.to_lowercase());
        teams.dedup();
        for team in teams {
            if !team.to_lowercase().contains(&query) {
                continue;
            }
            if ui
                .selectable_label(self.selected_team.as_deref() == Some(team), team)
                .clicked()
            {
                self.selected_team = Some(team.clone());
                self.selected_kit = 0;
            }
        }
    }

    fn show_kit_preview(
        &self,
        ui: &mut egui::Ui,
        kit: &[String; 5],
        colours: &std::collections::HashMap<String, egui::Color32>,
    ) {
        let size = egui::vec2(360.0, 360.0);
        let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
        for (index, name) in [
            "background",
            "shirt",
            "stripes",
            "sleeves",
            "shorts",
            "socks",
        ]
        .into_iter()
        .enumerate()
        {
            if let Some(texture) = self.kit_textures.get(name) {
                ui.painter().image(
                    texture.id(),
                    rect,
                    egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
                    if index == 0 {
                        egui::Color32::WHITE
                    } else {
                        teams::colour_for(colours, &kit[index - 1])
                    },
                );
            }
        }
    }

    fn show_team_details(&mut self, ui: &mut egui::Ui) {
        let Some(team) = self.selected_team.clone() else {
            ui.label("Seleziona una squadra dall'elenco.");
            return;
        };
        let default_reputation = self.world.as_ref().map_or(5, |world| {
            if let Some(league) = self
                .selected_league
                .and_then(|index| world.leagues.get(index))
            {
                league
                    .divisions
                    .iter()
                    .find(|division| division.teams.iter().any(|club| club == &team))
                    .and_then(|division| division.reputation)
                    .unwrap_or(10)
            } else {
                self.selected_international_country
                    .and_then(|index| world.international_countries.get(index))
                    .and_then(|country| country.reputation)
                    .unwrap_or(5)
            }
        });
        let data_directory = self
            .world
            .as_ref()
            .and_then(|world| world.league_directory.parent())
            .map(std::path::Path::to_path_buf);
        let path = data_directory
            .as_ref()
            .map(|directory| directory.join("Team").join(format!("{team}.txt")));
        let mut profile = path.as_ref().map_or_else(
            || teams::TeamProfile::defaults(default_reputation),
            |path| teams::load_team(path, default_reputation),
        );
        if let Some(path) = &path
            && let Some((_, edited)) = self.team_edits.get(path)
        {
            profile = edited.clone();
        }
        let colours = data_directory
            .as_ref()
            .map_or_else(std::collections::HashMap::new, |directory| {
                teams::load_colours(&directory.join("Graphics").join("color_table.txt"))
            });
        ui.heading(&team);
        ui.label(format!("Reputazione: {}", profile.reputation));
        ui.label(if profile.file_exists {
            "File Team caricato"
        } else {
            "Nessun file Team: divise bianche e reputazione predefinita"
        });
        ui.horizontal(|ui| {
            if ui.button("◀").clicked() {
                self.selected_kit = self.selected_kit.checked_sub(1).unwrap_or(2);
            }
            ui.heading(["Home", "Away", "Third"][self.selected_kit]);
            if ui.button("▶").clicked() {
                self.selected_kit = (self.selected_kit + 1) % 3;
            }
        });
        let palette = data_directory.as_ref().map_or_else(
            || vec!["white".into()],
            |directory| teams::colour_names(&directory.join("Graphics").join("color_table.txt")),
        );
        let mut colour_change = None;
        for (part_index, part) in teams::KIT_PARTS.iter().enumerate() {
            ui.horizontal(|ui| {
                if ui.button("◀").clicked() {
                    colour_change = Some((part_index, -1isize));
                }
                ui.label(format!(
                    "{part}: {}",
                    profile.kits[self.selected_kit][part_index]
                ));
                if ui.button("▶").clicked() {
                    colour_change = Some((part_index, 1isize));
                }
            });
        }
        if let Some((part_index, direction)) = colour_change {
            let current = &profile.kits[self.selected_kit][part_index];
            let index = palette
                .iter()
                .position(|colour| colour.eq_ignore_ascii_case(current))
                .unwrap_or(0) as isize;
            let next = (index + direction).rem_euclid(palette.len() as isize) as usize;
            profile.kits[self.selected_kit][part_index] = palette[next].clone();
            if let Some(path) = path {
                self.team_edits
                    .insert(path, (team.clone(), profile.clone()));
            }
        }
        ui.add_space(12.0);
        self.show_kit_preview(ui, &profile.kits[self.selected_kit], &colours);
        ui.add_space(12.0);
        ui.label("I colori vengono salvati con Salva modifiche.");
    }

    fn has_changes(&self) -> bool {
        match (&self.world, &self.saved_world) {
            (Some(world), Some(saved)) => {
                world.leagues.len() != saved.leagues.len()
                    || world
                        .leagues
                        .iter()
                        .zip(&saved.leagues)
                        .any(|(league, old)| {
                            league.path != old.path
                                || league.divisions != old.divisions
                                || league.division_sources != old.division_sources
                        })
                    || world.international_countries != saved.international_countries
                    || !world.retained_clubs.is_empty()
                    || !world.deleted_clubs.is_empty()
                    || !self.team_edits.is_empty()
            }
            _ => false,
        }
    }

    fn save_changes(&mut self) {
        self.save_message = None;
        let (Some(world), Some(saved)) = (&mut self.world, &self.saved_world) else {
            return;
        };
        // Sincronizza solo le leghe modificate: l'apertura da sola non riscrive gli altri paesi.
        for index in 0..world.leagues.len() {
            if saved
                .leagues
                .iter()
                .find(|old| old.path == world.leagues[index].path)
                .is_none_or(|old| old.divisions != world.leagues[index].divisions)
                && let Err(error) = world.sync_international(index)
            {
                self.open_error = Some(error);
                return;
            }
        }
        let changes = match world_save::prepare(world, saved) {
            Ok(changes) => changes,
            Err(error) => {
                self.open_error = Some(format!("Nessun file salvato.\n{error}"));
                return;
            }
        };
        let staging = world.league_directory.parent().unwrap();
        if let Err(error) = world_save::commit(&changes, staging) {
            self.open_error = Some(error);
            return;
        }
        world_save::mark_saved(world, &changes);
        self.saved_world = Some(world.clone());
        let pending_team_edits = std::mem::take(&mut self.team_edits);
        for (path, (team, profile)) in pending_team_edits {
            if let Err(error) = teams::save_team(&path, &team, &profile) {
                self.team_edits.insert(path, (team, profile));
                self.open_error = Some(format!(
                    "Modifiche alle leghe salvate, ma non alla divisa: {error}"
                ));
                return;
            }
        }
        self.open_error = None;
        self.save_message = Some(format!(
            "Salvate {} operazioni su file, senza backup automatici.",
            changes.len()
        ));
    }

    fn sync_selected_league(&mut self) {
        if let (Some(world), Some(index)) = (&mut self.world, self.selected_league)
            && let Err(error) = world.sync_international(index)
        {
            self.open_error = Some(error);
        }
    }

    fn begin_creation(&mut self, league: bool) {
        let Some(world) = &self.world else {
            return;
        };
        self.creation = if league {
            let country =
                self.selected_international_country
                    .and_then(|index| world.international_countries.get(index))
                    .filter(|country| {
                        !world
                            .leagues
                            .iter()
                            .any(|league| league.country.eq_ignore_ascii_case(&country.name))
                    })
                    .map(|country| country.name.clone())
                    .or_else(|| {
                        world
                            .international_countries
                            .iter()
                            .find(|country| {
                                !world.leagues.iter().any(|league| {
                                    league.country.eq_ignore_ascii_case(&country.name)
                                })
                            })
                            .map(|country| country.name.clone())
                    })
                    .unwrap_or_default();
            Some(league_dialogs::Creation {
                kind: league_dialogs::CreationKind::League,
                country,
                division_name: "Prima divisione".into(),
                clubs: String::new(),
            })
        } else {
            self.selected_league.map(|index| league_dialogs::Creation {
                kind: league_dialogs::CreationKind::Division {
                    league_index: index,
                },
                country: String::new(),
                division_name: format!("Divisione {}", world.leagues[index].divisions.len() + 1),
                clubs: String::new(),
            })
        };
    }

    fn show_structure_toolbar(&mut self, ui: &mut egui::Ui) {
        let selected = self.selected_league;
        let can_add = selected.is_some_and(|index| {
            self.world
                .as_ref()
                .is_some_and(|world| world.leagues[index].divisions.len() < 12)
        });
        let can_remove = selected.is_some_and(|index| {
            self.world
                .as_ref()
                .is_some_and(|world| world.can_remove_division(index, self.selected_division))
        });
        ui.horizontal_wrapped(|ui| {
            if ui
                .add_enabled(can_add, egui::Button::new("Nuova divisione…"))
                .clicked()
            {
                self.begin_creation(false);
            }
            if ui
                .add_enabled(can_remove, egui::Button::new("Rimuovi divisione…"))
                .clicked()
            {
                self.removal = Some(league_dialogs::Removal::Division {
                    league_index: selected.unwrap(),
                    division_index: self.selected_division,
                    delete_clubs: false,
                });
            }
            if ui
                .add_enabled(selected.is_some(), egui::Button::new("Elimina lega…"))
                .clicked()
            {
                self.removal = Some(league_dialogs::Removal::League {
                    league_index: selected.unwrap(),
                    delete_clubs: false,
                });
            }
            if ui
                .add_enabled(selected.is_some(), egui::Button::new("Aggiungi squadra…"))
                .clicked()
            {
                self.club_addition = selected.map(|league_index| league_dialogs::ClubAddition {
                    league_index,
                    division_index: self.selected_division,
                    source: league_dialogs::ClubSource::Neutral,
                    club_name: String::new(),
                });
            }
        });
    }

    fn show_structure_dialogs(&mut self, context: &egui::Context) {
        let mut create = false;
        let mut close_creation = false;
        if let Some(dialog) = &mut self.creation {
            let countries: Vec<String> =
                self.world
                    .as_ref()
                    .map(|world| {
                        world
                            .international_countries
                            .iter()
                            .filter(|country| {
                                !world.leagues.iter().any(|league| {
                                    league.country.eq_ignore_ascii_case(&country.name)
                                })
                            })
                            .map(|country| country.name.clone())
                            .collect()
                    })
                    .unwrap_or_default();
            let title = if dialog.kind == league_dialogs::CreationKind::League {
                "Nuova lega"
            } else {
                "Nuova divisione"
            };
            egui::Window::new(title)
                .collapsible(false)
                .resizable(true)
                .show(context, |ui| {
                    match dialog.kind {
                        league_dialogs::CreationKind::League => {
                            ui.label("Paese senza campionato domestico");
                            egui::ComboBox::from_id_salt("new_league_country")
                                .selected_text(&dialog.country)
                                .show_ui(ui, |ui| {
                                    for country in &countries {
                                        ui.selectable_value(
                                            &mut dialog.country,
                                            country.clone(),
                                            country,
                                        );
                                    }
                                });
                            ui.label(
                                "I club internazionali del paese saranno aggiunti automaticamente.",
                            );
                        }
                        league_dialogs::CreationKind::Division { .. } => {
                            ui.label("Inserisci da 3 a 24 club, uno per riga.");
                        }
                    }
                    ui.label("Nome della divisione");
                    ui.text_edit_singleline(&mut dialog.division_name);
                    ui.label("Club aggiuntivi / club della nuova divisione");
                    ui.add(
                        egui::TextEdit::multiline(&mut dialog.clubs)
                            .desired_rows(8)
                            .desired_width(520.0),
                    );
                    ui.horizontal(|ui| {
                        if ui.button("Crea").clicked() {
                            create = true;
                        }
                        if ui.button("Annulla").clicked() {
                            close_creation = true;
                        }
                    });
                });
        }
        if create {
            if let Some(dialog) = self.creation.take() {
                let clubs = dialog
                    .clubs
                    .lines()
                    .map(str::trim)
                    .filter(|club| !club.is_empty())
                    .map(str::to_owned)
                    .collect();
                let result = match dialog.kind {
                    league_dialogs::CreationKind::League => self
                        .world
                        .as_mut()
                        .unwrap()
                        .create_league(&dialog.country, &dialog.division_name, clubs)
                        .map(|index| {
                            self.selected_league = Some(index);
                            self.selected_international_country = None;
                            self.selected_division = 0;
                        }),
                    league_dialogs::CreationKind::Division { league_index } => self
                        .world
                        .as_mut()
                        .unwrap()
                        .add_division(league_index, &dialog.division_name, clubs)
                        .map(|_| {
                            self.selected_league = Some(league_index);
                            self.selected_division = self.world.as_ref().unwrap().leagues
                                [league_index]
                                .divisions
                                .len()
                                - 1;
                        }),
                };
                if let Err(error) = result {
                    self.open_error = Some(error);
                }
            }
        } else if close_creation {
            self.creation = None;
        }

        let mut confirm_removal = false;
        let mut close_removal = false;
        if let Some(dialog) = &mut self.removal {
            let description = match dialog {
                league_dialogs::Removal::Division {
                    league_index,
                    division_index,
                    ..
                } => self
                    .world
                    .as_ref()
                    .and_then(|world| world.leagues.get(*league_index))
                    .and_then(|league| league.divisions.get(*division_index))
                    .map(|division| format!("Rimuovere la divisione {}?", division.name))
                    .unwrap_or_else(|| "Divisione non trovata".into()),
                league_dialogs::Removal::League { league_index, .. } => self
                    .world
                    .as_ref()
                    .and_then(|world| world.leagues.get(*league_index))
                    .map(|league| format!("Eliminare la lega {}?", league.country))
                    .unwrap_or_else(|| "Lega non trovata".into()),
                league_dialogs::Removal::Club {
                    league_index,
                    division_index,
                    club_index,
                    ..
                } => self
                    .world
                    .as_ref()
                    .and_then(|world| world.leagues.get(*league_index))
                    .and_then(|league| league.divisions.get(*division_index))
                    .and_then(|division| division.teams.get(*club_index))
                    .map(|club| format!("Rimuovere il club {club} dalla lega?"))
                    .unwrap_or_else(|| "Club non trovato".into()),
            };
            let delete_clubs = match dialog {
                league_dialogs::Removal::Division { delete_clubs, .. }
                | league_dialogs::Removal::League { delete_clubs, .. } => delete_clubs,
                league_dialogs::Removal::Club { delete_club, .. } => delete_club,
            };
            egui::Window::new("Conferma rimozione")
                .collapsible(false)
                .show(context, |ui| {
                    ui.label(description);
                    ui.checkbox(
                        delete_clubs,
                        "Elimina anche il file del club se resta senza riferimenti",
                    );
                    ui.label("I club ancora citati nel file internazionale non saranno eliminati.");
                    ui.horizontal(|ui| {
                        if ui.button("Rimuovi").clicked() {
                            confirm_removal = true;
                        }
                        if ui.button("Annulla").clicked() {
                            close_removal = true;
                        }
                    });
                });
        }
        if confirm_removal {
            if let Some(dialog) = self.removal.take() {
                let result = match dialog {
                    league_dialogs::Removal::Division {
                        league_index,
                        division_index,
                        delete_clubs,
                    } => self
                        .world
                        .as_mut()
                        .unwrap()
                        .remove_division(league_index, division_index, delete_clubs)
                        .map(|_| {
                            self.selected_division = self.selected_division.saturating_sub(1);
                            self.selected_club = None;
                        }),
                    league_dialogs::Removal::League {
                        league_index,
                        delete_clubs,
                    } => self
                        .world
                        .as_mut()
                        .unwrap()
                        .remove_league(league_index, delete_clubs)
                        .map(|_| {
                            self.selected_league = None;
                            self.selected_division = 0;
                            self.selected_club = None;
                        }),
                    league_dialogs::Removal::Club {
                        league_index,
                        division_index,
                        club_index,
                        delete_club,
                    } => self
                        .world
                        .as_mut()
                        .unwrap()
                        .remove_club_from_division(
                            league_index,
                            division_index,
                            club_index,
                            delete_club,
                        )
                        .map(|_| {
                            self.selected_club = None;
                            self.sync_selected_league();
                        }),
                };
                if let Err(error) = result {
                    self.open_error = Some(error);
                }
            }
        } else if close_removal {
            self.removal = None;
        }

        let mut add_club = false;
        let mut close_club_addition = false;
        if let Some(dialog) = &mut self.club_addition {
            let neutral_clubs = self
                .world
                .as_ref()
                .map(leagues::World::neutral_clubs)
                .unwrap_or_default();
            egui::Window::new("Aggiungi squadra")
                .collapsible(false)
                .show(context, |ui| {
                    ui.horizontal(|ui| {
                        ui.selectable_value(
                            &mut dialog.source,
                            league_dialogs::ClubSource::Neutral,
                            "Club neutro esistente",
                        );
                        ui.selectable_value(
                            &mut dialog.source,
                            league_dialogs::ClubSource::New,
                            "Nuovo club",
                        );
                    });
                    match dialog.source {
                        league_dialogs::ClubSource::Neutral => {
                            if neutral_clubs.is_empty() {
                                ui.label("Non esistono club neutri con un file Team disponibile.");
                            } else {
                                egui::ComboBox::from_id_salt("neutral_club")
                                    .selected_text(if dialog.club_name.is_empty() {
                                        "Scegli un club…"
                                    } else {
                                        &dialog.club_name
                                    })
                                    .show_ui(ui, |ui| {
                                        for club in &neutral_clubs {
                                            ui.selectable_value(
                                                &mut dialog.club_name,
                                                club.clone(),
                                                club,
                                            );
                                        }
                                    });
                            }
                        }
                        league_dialogs::ClubSource::New => {
                            ui.label("Nome del nuovo club");
                            ui.text_edit_singleline(&mut dialog.club_name);
                            ui.label("Dettagli e rosa saranno configurabili nella tab Teams.");
                        }
                    }
                    ui.horizontal(|ui| {
                        let valid_selection = match dialog.source {
                            league_dialogs::ClubSource::Neutral => {
                                neutral_clubs.iter().any(|club| club == &dialog.club_name)
                            }
                            league_dialogs::ClubSource::New => !dialog.club_name.trim().is_empty(),
                        };
                        if ui
                            .add_enabled(valid_selection, egui::Button::new("Aggiungi"))
                            .clicked()
                        {
                            add_club = true;
                        }
                        if ui.button("Annulla").clicked() {
                            close_club_addition = true;
                        }
                    });
                });
        }
        if add_club {
            if let Some(dialog) = self.club_addition.take()
                && let Err(error) = self.world.as_mut().unwrap().add_club_to_division(
                    dialog.league_index,
                    dialog.division_index,
                    dialog.club_name.trim(),
                )
            {
                self.open_error = Some(error);
            }
        } else if close_club_addition {
            self.club_addition = None;
        }
    }

    fn show_division_editor(&mut self, ui: &mut egui::Ui) {
        let Some(league) = self.world.as_mut().and_then(|world| {
            self.selected_league
                .and_then(|index| world.leagues.get_mut(index))
        }) else {
            return;
        };
        let (level, promotions, relegations, promotions_changed, relegations_changed, teams) = {
            let Some(division) = league.divisions.get_mut(self.selected_division) else {
                return;
            };
            ui.separator();
            ui.heading("Parametri della divisione");
            ui.label("Nome");
            ui.text_edit_singleline(&mut division.name);
            Self::number_field(ui, "Livello", &mut division.level, 1);
            Self::number_field(ui, "Reputazione", &mut division.reputation, 10);
            let promotions_changed =
                Self::number_field(ui, "Promozioni", &mut division.promotions, 0);
            let relegations_changed =
                Self::number_field(ui, "Retrocessioni", &mut division.relegations, 0);
            (
                division.level,
                division.promotions,
                division.relegations,
                promotions_changed,
                relegations_changed,
                division.teams.clone(),
            )
        };
        // Le due estremità dello stesso passaggio di categoria devono restare uguali.
        if let Some(level) = level {
            if promotions_changed
                && let Some(above) = league
                    .divisions
                    .iter_mut()
                    .find(|division| division.level == level.checked_sub(1))
            {
                above.relegations = promotions;
            }
            if relegations_changed
                && let Some(below) = league
                    .divisions
                    .iter_mut()
                    .find(|division| division.level == level.checked_add(1))
            {
                below.promotions = relegations;
            }
        }
        for issue in league.division_issues(self.selected_division) {
            ui.colored_label(egui::Color32::from_rgb(255, 195, 110), issue);
        }
        ui.separator();
        ui.heading(format!("Squadre ({})", teams.len()));
        ui.label("Ordine iniziale usato per l'accesso alle coppe.");
        // La colonna gestisce lo scorrimento: nessuna area annidata per la rosa.
        for (index, team) in teams.iter().enumerate() {
            let selection = (self.selected_league.unwrap(), self.selected_division, index);
            if ui
                .selectable_label(
                    self.selected_club == Some(selection),
                    format!("{:>2}. {team}", index + 1),
                )
                .clicked()
            {
                self.selected_club = Some(selection);
            }
        }
    }

    fn show_club_toolbar(&mut self, ui: &mut egui::Ui) {
        let selection = self.selected_club.filter(|(country, division, _)| {
            Some(*country) == self.selected_league && *division == self.selected_division
        });
        ui.horizontal_wrapped(|ui| {
            for (label, up) in [("Sposta su", true), ("Sposta giù", false)] {
                let enabled = selection.is_some_and(|(country, division, position)| {
                    self.world.as_ref().is_some_and(|world| {
                        world.leagues[country]
                            .move_target(division, position, up)
                            .is_some()
                    })
                });
                if ui.add_enabled(enabled, egui::Button::new(label)).clicked() {
                    let (country, division, position) = selection.unwrap();
                    if let Some((target_division, target_position)) =
                        self.world.as_mut().unwrap().leagues[country]
                            .move_club(division, position, up)
                    {
                        self.selected_division = target_division;
                        self.selected_club = Some((country, target_division, target_position));
                        self.sync_selected_league();
                    }
                }
            }
            if ui
                .add_enabled(selection.is_some(), egui::Button::new("Scambia squadre…"))
                .clicked()
            {
                self.swap_source = selection;
            }
            if ui
                .add_enabled(selection.is_some(), egui::Button::new("Rimuovi squadra…"))
                .clicked()
            {
                self.removal = selection.map(|(league_index, division_index, club_index)| {
                    league_dialogs::Removal::Club {
                        league_index,
                        division_index,
                        club_index,
                        delete_club: false,
                    }
                });
            }
        });
    }

    fn show_swap_window(&mut self, context: &egui::Context) {
        let Some((country, division, position)) = self.swap_source else {
            return;
        };
        let Some(league) = self
            .world
            .as_ref()
            .and_then(|world| world.leagues.get(country))
        else {
            self.swap_source = None;
            return;
        };
        let source_name = league.divisions[division].teams[position].clone();
        let mut destination = None;
        let mut open = true;
        egui::Window::new("Scambia squadre nello stesso paese")
            .open(&mut open)
            .default_width(600.0)
            .show(context, |ui| {
                ui.label(format!("Scambia {source_name} con:"));
                egui::ScrollArea::vertical()
                    .max_height(500.0)
                    .show(ui, |ui| {
                        let mut divisions: Vec<_> = league.divisions.iter().enumerate().collect();
                        divisions.sort_by_key(|(_, item)| item.level);
                        for (division_index, item) in divisions {
                            egui::CollapsingHeader::new(&item.name)
                                .id_salt(("swap_division", division_index))
                                .show(ui, |ui| {
                                    for (team_index, team) in item.teams.iter().enumerate() {
                                        if (division_index, team_index) != (division, position)
                                            && ui
                                                .button(format!("{}. {team}", team_index + 1))
                                                .clicked()
                                        {
                                            destination = Some((division_index, team_index));
                                        }
                                    }
                                });
                        }
                    });
            });
        if let Some(target) = destination {
            self.world.as_mut().unwrap().leagues[country].swap_clubs((division, position), target);
            self.selected_league = Some(country);
            self.selected_international_country = None;
            self.selected_division = target.0;
            self.selected_club = Some((country, target.0, target.1));
            self.sync_selected_league();
            self.swap_source = None;
        } else if !open {
            self.swap_source = None;
        }
    }

    fn number_field(ui: &mut egui::Ui, label: &str, value: &mut Option<u32>, default: u32) -> bool {
        ui.horizontal(|ui| {
            ui.label(label);
            let before = *value;
            if let Some(number) = value {
                if ui.small_button("−").clicked() {
                    *number = number.saturating_sub(1);
                }
                ui.add(egui::DragValue::new(number).range(0..=999));
                if ui.small_button("+").clicked() {
                    *number = number.saturating_add(1);
                }
            } else {
                ui.label("Non definito");
                if ui.button("Imposta").clicked() {
                    *value = Some(default);
                }
            }
            *value != before
        })
        .inner
    }

    fn show_international_clubs(
        ui: &mut egui::Ui,
        country: &leagues::InternationalCountry,
        league: Option<&leagues::LeagueFile>,
    ) {
        ui.label("Fonte: International teams and tournaments.txt • Sola lettura");
        if let Some(reputation) = country.reputation {
            ui.label(format!("Reputazione: {reputation}"));
        }
        for path in &country.name_files {
            ui.label(format!("Archivio nomi: {path}"));
        }
        ui.label(format!("{} club", country.clubs.len()));
        for (index, club) in country.clubs.iter().enumerate() {
            let Some(league) = league else {
                ui.label(format!("{:>2}. {club}", index + 1));
                continue;
            };
            let locations = league.club_locations(club);
            match locations.as_slice() {
                [(division, position)] => {
                    ui.label(format!(
                        "{:>2}. {club} — {} · posizione {}",
                        index + 1,
                        league.divisions[*division].name,
                        position + 1
                    ));
                }
                [] => {
                    ui.colored_label(
                        egui::Color32::from_rgb(255, 195, 110),
                        format!("{:>2}. {club} — Non presente nel campionato", index + 1),
                    );
                }
                _ => {
                    let details = locations
                        .iter()
                        .map(|(division, position)| {
                            format!(
                                "{} · posizione {}",
                                league.divisions[*division].name,
                                position + 1
                            )
                        })
                        .collect::<Vec<_>>()
                        .join("; ");
                    ui.colored_label(
                        egui::Color32::from_rgb(255, 195, 110),
                        format!("{:>2}. {club} — Duplicato: {details}", index + 1),
                    );
                }
            }
        }
    }
}

impl eframe::App for WorldEditor {
    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        let previous_font_size = self.font_size;
        if self.has_changes() && ui.ctx().input(|input| input.viewport().close_requested()) {
            ui.ctx()
                .send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.open_error = Some(
                "Ci sono modifiche non salvate. Salva o scarta le modifiche prima di chiudere."
                    .into(),
            );
        }
        egui::Panel::top("menu").show(ui, |ui| {
            self.show_menu(ui);
            ui.horizontal_wrapped(|ui| {
                if ui
                    .add_enabled(self.has_changes(), egui::Button::new("Salva modifiche"))
                    .clicked()
                {
                    self.save_changes();
                }
                if ui
                    .add_enabled(self.has_changes(), egui::Button::new("Scarta modifiche"))
                    .clicked()
                {
                    self.confirm_discard = true;
                }
                ui.label(if self.has_changes() {
                    "Modifiche non salvate • solo in memoria"
                } else {
                    "Nessuna modifica"
                });
            });
            if let Some(message) = &self.save_message {
                ui.label(message);
            }
            ui.separator();
            ui.horizontal_wrapped(|ui| {
                ui.strong("WORLD EDITOR");
                ui.label(self.world.as_ref().map_or_else(
                    || "/ Nessun mondo aperto".into(),
                    |world| world.league_directory.display().to_string(),
                ));
            });
            ui.add_space(6.0);
            ui.horizontal_wrapped(|ui| {
                for (index, (name, _)) in TABS.iter().enumerate() {
                    ui.selectable_value(&mut self.selected_tab, index, *name);
                }
            });
        });
        egui::Panel::bottom("status").show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label(self.world.as_ref().map_or_else(
                    || "Nessun dato caricato".into(),
                    |world| {
                        format!(
                            "{} file nazionali • {} segnalazioni • Editor leghe",
                            world.leagues.len(),
                            world.warnings.len()
                        )
                    },
                ));
                ui.separator();
                ui.label(format!("{} · {:.0} pt", self.font_name, self.font_size));
            });
        });
        let (name, description) = TABS[self.selected_tab];
        if name != "Transfers" {
            egui::Panel::left("element_list")
                .default_size(if name == "Teams" { 230.0 } else { 340.0 })
                .resizable(true)
                .show(ui, |ui| {
                    // Titolo e ricerca restano fissi: solo gli elementi scorrono.
                    ui.heading(name);
                    if name == "Leagues" {
                        if ui
                            .add_enabled(self.world.is_some(), egui::Button::new("Nuova lega…"))
                            .clicked()
                        {
                            self.begin_creation(true);
                        }
                        ui.add(
                            egui::TextEdit::singleline(&mut self.league_search)
                                .hint_text("Cerca paese o file…")
                                .desired_width(f32::INFINITY),
                        );
                    } else if name == "Teams" {
                        ui.label("Seleziona la lega");
                    } else {
                        let mut search_placeholder = String::new();
                        ui.add_enabled(
                            false,
                            egui::TextEdit::singleline(&mut search_placeholder)
                                .hint_text("Cerca…")
                                .desired_width(f32::INFINITY),
                        );
                    }
                    ui.separator();
                    egui::ScrollArea::vertical()
                        .id_salt(("sidebar_items", name))
                        .show(ui, |ui| {
                            if name == "Leagues" {
                                self.show_league_list(ui);
                            } else if name == "Teams" {
                                self.show_team_league_list(ui);
                            } else {
                                ui.label("Nessun elemento");
                                ui.label("Qui comparirà l'elenco del mondo aperto.");
                            }
                        });
                });
        }
        if name == "Leagues" {
            egui::Panel::left("division_list")
                .default_size(270.0)
                .resizable(true)
                .show(ui, |ui| {
                    ui.heading("Divisioni");
                    ui.separator();
                    egui::ScrollArea::vertical()
                        .id_salt("division_items")
                        .show(ui, |ui| {
                            if let Some(league) = self.world.as_ref().and_then(|world| {
                                self.selected_league
                                    .and_then(|index| world.leagues.get(index))
                            }) {
                                let mut divisions: Vec<_> =
                                    league.divisions.iter().enumerate().collect();
                                divisions.sort_by_key(|(_, division)| division.level);
                                for (index, division) in divisions {
                                    ui.selectable_value(
                                        &mut self.selected_division,
                                        index,
                                        format!(
                                            "{} · {}",
                                            division
                                                .level
                                                .map_or("?".into(), |value| value.to_string()),
                                            division.name
                                        ),
                                    );
                                }
                            } else {
                                ui.label("Seleziona un campionato.");
                            }
                        });
                });
        }
        if name == "Teams" {
            egui::Panel::left("team_list")
                .default_size(300.0)
                .resizable(true)
                .show(ui, |ui| {
                    ui.heading("Squadre");
                    ui.add(
                        egui::TextEdit::singleline(&mut self.team_search)
                            .hint_text("Cerca squadra…")
                            .desired_width(f32::INFINITY),
                    );
                    ui.separator();
                    egui::ScrollArea::vertical()
                        .id_salt("team_items")
                        .show(ui, |ui| self.show_team_list(ui));
                });
        }
        egui::CentralPanel::default().show(ui, |ui| {
            if name == "Leagues" {
                self.show_structure_toolbar(ui);
                self.show_club_toolbar(ui);
                ui.separator();
            }
            egui::ScrollArea::both()
                .id_salt(("details_content", name))
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.add_space(12.0);
                    ui.heading(name);
                    ui.label(description);
                    ui.add_space(16.0);
                    ui.separator();
                    ui.add_space(16.0);
                    if name == "Leagues" {
                        self.show_league_details(ui);
                        self.show_division_editor(ui);
                    } else if name == "Teams" {
                        self.show_team_details(ui);
                    } else if name == "Transfers" {
                        ui.columns(2, |columns| {
                            Self::show_empty_roster(&mut columns[0], "Squadra di origine");
                            Self::show_empty_roster(&mut columns[1], "Squadra di destinazione");
                        });
                    } else {
                        ui.heading("Il tuo mondo comincia qui");
                        ui.label("Questa sezione sarà disponibile in un prossimo micro-step.");
                        ui.label("Selezionando un elemento vedrai qui i suoi dettagli.");
                    }
                });
        });
        self.show_swap_window(ui.ctx());
        self.show_structure_dialogs(ui.ctx());
        if self.confirm_discard {
            egui::Window::new("Scartare le modifiche?")
                .collapsible(false)
                .show(ui.ctx(), |ui| {
                    ui.label("Leghe, divisioni e club torneranno all’ultimo stato salvato.");
                    if ui.button("Scarta tutte le modifiche").clicked() {
                        self.world = self.saved_world.clone();
                        self.selected_league = self
                            .world
                            .as_ref()
                            .and_then(|world| (!world.leagues.is_empty()).then_some(0));
                        self.selected_division = 0;
                        self.selected_international_country = None;
                        self.creation = None;
                        self.removal = None;
                        self.club_addition = None;
                        self.selected_club = None;
                        self.selected_team = None;
                        self.team_search.clear();
                        self.selected_kit = 0;
                        self.team_edits.clear();
                        self.swap_source = None;
                        self.confirm_discard = false;
                    }
                    if ui.button("Continua a modificare").clicked() {
                        self.confirm_discard = false;
                    }
                });
        }
        if let Some(error) = self.open_error.clone() {
            egui::Window::new("Operazione non completata")
                .collapsible(false)
                .resizable(true)
                .show(ui.ctx(), |ui| {
                    ui.label(error);
                    if ui.button("Chiudi").clicked() {
                        self.open_error = None;
                    }
                });
        }
        if self.font_size != previous_font_size {
            // Non aspettiamo l'uscita normale: anche interrompendo cargo run la scelta resta salvata.
            if let Some(storage) = frame.storage_mut() {
                storage.set_string(FONT_SIZE_KEY, self.font_size.to_string());
                storage.flush();
            }
        }
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        storage.set_string(FONT_SIZE_KEY, self.font_size.to_string());
    }
}
