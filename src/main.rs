use eframe::egui;
mod leagues;

const DEFAULT_FONT_SIZE: f32 = 20.0;
const FONT_SIZE_KEY: &str = "editor_font_size";
const TABS: [(&str, &str); 6] = [
    ("Nations", "Paesi, nazionalità e archivi dei nomi"),
    ("Leagues", "Divisioni, squadre, promozioni e retrocessioni"),
    ("Cups", "Partecipanti, turni e calendario delle coppe"),
    ("Teams", "Club e nazionali, divise e rose"),
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
    selected_tab: usize,
    font_size: f32,
    font_name: &'static str,
    world: Option<leagues::World>,
    selected_league: Option<usize>,
    league_search: String,
    open_error: Option<String>,
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
        context.egui_ctx.set_visuals(egui::Visuals::dark());
        let editor = Self {
            selected_tab: 0,
            font_size,
            font_name,
            world: None,
            selected_league: None,
            league_search: String::new(),
            open_error: None,
        };
        editor.apply_font_size(&context.egui_ctx);
        editor
    }

    fn apply_font_size(&self, context: &egui::Context) {
        let mut style = (*context.global_style()).clone();
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
        style.spacing.item_spacing = egui::vec2(14.0, 12.0);
        style.spacing.button_padding = egui::vec2(14.0, 8.0);
        style.spacing.interact_size.y = self.font_size + 16.0;
        context.set_global_style(style);
    }

    fn show_menu(&mut self, ui: &mut egui::Ui) {
        egui::MenuBar::new().ui(ui, |ui| {
            ui.menu_button("File", |ui| {
                if ui.button("Apri mondo…").clicked() {
                    ui.close();
                    if let Some(path) = rfd::FileDialog::new()
                        .set_title("Seleziona la cartella del gioco, del pacchetto o Data")
                        .pick_folder()
                    {
                        // Sostituiamo il mondo solo dopo un caricamento riuscito.
                        match leagues::load_world(&path) {
                            Ok(world) => {
                                self.selected_league = (!world.leagues.is_empty()).then_some(0);
                                self.world = Some(world);
                                self.league_search.clear();
                                self.open_error = None;
                                self.selected_tab = 1;
                            }
                            Err(error) => self.open_error = Some(error),
                        }
                    }
                }
                for label in ["Salva", "Esporta…"] {
                    ui.add_enabled(false, egui::Button::new(label))
                        .on_disabled_hover_text("Disponibile in un prossimo micro-step");
                }
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
        ui.add(
            egui::TextEdit::singleline(&mut self.league_search)
                .hint_text("Cerca paese o file…")
                .desired_width(f32::INFINITY),
        );
        ui.separator();
        let Some(world) = &self.world else {
            ui.label("Usa File → Apri mondo per caricare i campionati.");
            return;
        };
        let query = self.league_search.to_lowercase();
        let mut matches = 0;
        for (index, league) in world.leagues.iter().enumerate() {
            let filename = league
                .path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy();
            if !league.country.to_lowercase().contains(&query)
                && !filename.to_lowercase().contains(&query)
            {
                continue;
            }
            matches += 1;
            let label = if league.country.is_empty() {
                filename.as_ref()
            } else {
                &league.country
            };
            if ui
                .selectable_label(self.selected_league == Some(index), label)
                .on_hover_text(league.path.display().to_string())
                .clicked()
            {
                self.selected_league = Some(index);
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
        if world.international_file.is_some() {
            ui.label("File internazionale riconosciuto: sarà gestito nella sezione Cups.");
        }
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
        let Some(league) = self
            .selected_league
            .and_then(|index| world.leagues.get(index))
        else {
            ui.label("Seleziona un campionato dall'elenco.");
            return;
        };
        ui.heading(&league.country);
        ui.label(league.path.display().to_string());
        ui.label(format!(
            "{} divisioni • Sola lettura",
            league.divisions.len()
        ));
        let value = |number: Option<u32>| {
            number.map_or_else(|| "non definito".into(), |number| number.to_string())
        };
        // Gli indici identificano anche divisioni con nomi ripetuti; niente ordinamento dei club.
        for (index, division) in league.divisions.iter().enumerate() {
            egui::CollapsingHeader::new(format!(
                "{} · {} · {} squadre",
                value(division.level),
                division.name,
                division.teams.len()
            ))
            .id_salt((&league.path, index))
            .default_open(index == 0)
            .show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.label(format!("Promozioni: {}", value(division.promotions)));
                    ui.label(format!("Retrocessioni: {}", value(division.relegations)));
                    ui.label(format!("Reputazione: {}", value(division.reputation)));
                });
                ui.separator();
                for (team_index, team) in division.teams.iter().enumerate() {
                    ui.label(format!("{:>2}. {}", team_index + 1, team));
                }
            });
        }
    }
}

impl eframe::App for WorldEditor {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::Panel::top("menu").show(ui, |ui| {
            self.show_menu(ui);
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
                            "{} file nazionali • {} segnalazioni • Sola lettura",
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
                .default_size(340.0)
                .resizable(true)
                .show(ui, |ui| {
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        ui.heading(name);
                        if name == "Leagues" {
                            self.show_league_list(ui);
                            return;
                        }
                        let mut search_placeholder = String::new();
                        ui.add_enabled(
                            false,
                            egui::TextEdit::singleline(&mut search_placeholder)
                                .hint_text("Cerca…")
                                .desired_width(f32::INFINITY),
                        );
                        ui.separator();
                        ui.label("Nessun elemento");
                        ui.label("Qui comparirà l'elenco del mondo aperto.");
                    });
                });
        }
        egui::CentralPanel::default().show(ui, |ui| {
            egui::ScrollArea::both().show(ui, |ui| {
                ui.add_space(12.0);
                ui.heading(name);
                ui.label(description);
                ui.add_space(16.0);
                ui.separator();
                ui.add_space(16.0);
                if name == "Leagues" {
                    self.show_league_details(ui);
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
        if let Some(error) = self.open_error.clone() {
            egui::Window::new("Impossibile aprire il mondo")
                .collapsible(false)
                .resizable(true)
                .show(ui.ctx(), |ui| {
                    ui.label(error);
                    if ui.button("Chiudi").clicked() {
                        self.open_error = None;
                    }
                });
        }
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        storage.set_string(FONT_SIZE_KEY, self.font_size.to_string());
    }
}
