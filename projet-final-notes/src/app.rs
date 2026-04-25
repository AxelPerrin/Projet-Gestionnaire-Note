use crate::api::fetch_sample_notes;
use crate::dao::{Dao, JsonDao, SqliteDao};
use crate::model::{AppState, Backend, Note};
use chrono::Utc;
use egui::{Context, Key, Modifiers, ScrollArea, TextEdit, TopBottomPanel};
use std::sync::mpsc::{self, Receiver};
use uuid::Uuid;

/// Application principale.
pub struct NotesApp {
    state: AppState,
    dao: Box<dyn Dao>,
    backend: Backend,
    selected_note: Option<Uuid>,
    erreur: Option<String>,
    rx_fetch: Option<Receiver<Result<Vec<Note>, String>>>,
    theme_sombre: bool,
    // Champs d'édition temporaires
    edit_titre: String,
    edit_contenu: String,
    edit_tags: String,
    // Dashboard
    afficher_dashboard: bool,
    // Export
    export_statut: Option<String>,
}

impl NotesApp {
    pub fn nouveau() -> Self {
        let (dao, backend) = Self::creer_dao_json();
        let mut app = Self {
            state: AppState::default(),
            dao,
            backend,
            selected_note: None,
            erreur: None,
            rx_fetch: None,
            theme_sombre: false,
            edit_titre: String::new(),
            edit_contenu: String::new(),
            edit_tags: String::new(),
            afficher_dashboard: false,
            export_statut: None,
        };
        app.charger_notes();
        app
    }

    fn creer_dao_json() -> (Box<dyn Dao>, Backend) {
        let chemin = Self::chemin_donnees("notes.json");
        (Box::new(JsonDao::nouveau(chemin)), Backend::Json)
    }

    fn creer_dao_sqlite() -> Result<(Box<dyn Dao>, Backend), String> {
        let chemin = Self::chemin_donnees("notes.db");
        let dao = SqliteDao::nouveau(chemin)?;
        Ok((Box::new(dao), Backend::Sqlite))
    }

    fn chemin_donnees(fichier: &str) -> std::path::PathBuf {
        let base = dirs::data_local_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."));
        base.join("projet-final-notes").join(fichier)
    }

    fn charger_notes(&mut self) {
        match self.dao.lister() {
            Ok(notes) => self.state.notes = notes,
            Err(e) => self.erreur = Some(e),
        }
    }

    fn nouvelle_note(&mut self) {
        let note = Note::nouveau("Nouvelle note", "", vec![]);
        match self.dao.creer(&note) {
            Ok(()) => {
                self.selected_note = Some(note.id);
                self.edit_titre = note.titre.clone();
                self.edit_contenu = note.contenu.clone();
                self.edit_tags = String::new();
                self.state.notes.push(note);
                self.erreur = None;
            }
            Err(e) => self.erreur = Some(e),
        }
    }

    fn sauvegarder_note(&mut self) {
        let id = match self.selected_note {
            Some(id) => id,
            None => return,
        };
        let tags: Vec<String> = self
            .edit_tags
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        if let Some(note) = self.state.notes.iter_mut().find(|n| n.id == id) {
            note.titre = self.edit_titre.clone();
            note.contenu = self.edit_contenu.clone();
            note.tags = tags;
            note.modifie_le = Utc::now();
            let note_clone = note.clone();
            match self.dao.mettre_a_jour(&note_clone) {
                Ok(()) => self.erreur = None,
                Err(e) => self.erreur = Some(e),
            }
        }
    }

    fn supprimer_note(&mut self) {
        let id = match self.selected_note {
            Some(id) => id,
            None => return,
        };
        match self.dao.supprimer(id) {
            Ok(()) => {
                self.state.notes.retain(|n| n.id != id);
                self.selected_note = None;
                self.edit_titre.clear();
                self.edit_contenu.clear();
                self.edit_tags.clear();
                self.erreur = None;
            }
            Err(e) => self.erreur = Some(e),
        }
    }

    fn basculer_backend(&mut self) {
        let resultat = match self.backend {
            Backend::Json => Self::creer_dao_sqlite(),
            Backend::Sqlite => Ok(Self::creer_dao_json()),
        };
        match resultat {
            Ok((dao, backend)) => {
                self.dao = dao;
                self.backend = backend;
                self.charger_notes();
                self.selected_note = None;
                self.erreur = None;
            }
            Err(e) => self.erreur = Some(e),
        }
    }

    fn selectionner_note(&mut self, id: Uuid) {
        self.selected_note = Some(id);
        if let Some(note) = self.state.notes.iter().find(|n| n.id == id) {
            self.edit_titre = note.titre.clone();
            self.edit_contenu = note.contenu.clone();
            self.edit_tags = note.tags.join(", ");
        }
    }

    fn traiter_reception_fetch(&mut self) {
        let notes_recues = if let Some(rx) = &self.rx_fetch {
            match rx.try_recv() {
                Ok(resultat) => Some(resultat),
                Err(mpsc::TryRecvError::Empty) => None,
                Err(mpsc::TryRecvError::Disconnected) => {
                    Some(Err("Canal de réception déconnecté".to_string()))
                }
            }
        } else {
            None
        };

        if let Some(resultat) = notes_recues {
            self.rx_fetch = None;
            match resultat {
                Ok(notes) => {
                    for note in notes {
                        match self.dao.creer(&note) {
                            Ok(()) => self.state.notes.push(note),
                            Err(e) => {
                                self.erreur = Some(e);
                                break;
                            }
                        }
                    }
                    self.erreur = None;
                }
                Err(e) => self.erreur = Some(format!("Import REST : {e}")),
            }
        }
    }

    fn exporter_json(&mut self) {
        let horodatage = Utc::now().format("%Y%m%d_%H%M%S");
        let nom = format!("export_notes_{horodatage}.json");
        let chemin = Self::chemin_donnees(&nom);
        match serde_json::to_string_pretty(&self.state.notes) {
            Ok(json) => {
                if let Some(parent) = chemin.parent() {
                    let _ = std::fs::create_dir_all(parent);
                }
                match std::fs::write(&chemin, json) {
                    Ok(()) => {
                        self.export_statut =
                            Some(format!("Exporté : {}", chemin.display()));
                    }
                    Err(e) => self.erreur = Some(format!("Export échoué : {e}")),
                }
            }
            Err(e) => self.erreur = Some(format!("Sérialisation export : {e}")),
        }
    }

    // ── Panneaux UI ────────────────────────────────────────────────────────

    fn afficher_top_panel(&mut self, ctx: &Context) {
        TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.button("➕ Nouvelle note").clicked() {
                    self.nouvelle_note();
                }
                if ui.button("🌐 Importer REST").clicked() && self.rx_fetch.is_none() {
                    let (tx, rx) = mpsc::channel();
                    self.rx_fetch = Some(rx);
                    fetch_sample_notes(tx);
                }
                if self.rx_fetch.is_some() {
                    ui.spinner();
                    ui.label("Chargement…");
                }
                ui.separator();

                let label_backend = format!("Backend : {}", self.backend);
                if ui.button(&label_backend).on_hover_text("Cliquer pour basculer").clicked() {
                    self.basculer_backend();
                }
                ui.separator();

                let label_theme = if self.theme_sombre { "☀ Clair" } else { "🌙 Sombre" };
                if ui.button(label_theme).clicked() {
                    self.theme_sombre = !self.theme_sombre;
                    if self.theme_sombre {
                        ctx.set_visuals(egui::Visuals::dark());
                    } else {
                        ctx.set_visuals(egui::Visuals::light());
                    }
                }
                ui.separator();

                if ui.button("📊 Dashboard").clicked() {
                    self.afficher_dashboard = !self.afficher_dashboard;
                }

                if ui.button("💾 Exporter JSON").clicked() {
                    self.exporter_json();
                }
            });
        });
    }

    fn afficher_status_bar(&self, ctx: &Context) {
        TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                let total = self.state.notes.len();
                let filtrees = self.state.notes_filtrees().len();
                ui.label(format!("Notes : {filtrees} / {total}"));
                ui.separator();
                ui.label(format!("Backend : {}", self.backend));
                if let Some(statut) = &self.export_statut {
                    ui.separator();
                    ui.label(statut);
                }
            });
        });
    }

    fn afficher_bandeau_erreur(&mut self, ctx: &Context) {
        if self.erreur.is_some() {
            TopBottomPanel::top("bandeau_erreur").show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.colored_label(egui::Color32::RED, "⚠");
                    if let Some(err) = &self.erreur {
                        ui.label(err);
                    }
                    if ui.button("✖").clicked() {
                        self.erreur = None;
                    }
                });
            });
        }
    }

    fn afficher_sidebar(&mut self, ctx: &Context) {
        egui::SidePanel::left("sidebar")
            .resizable(true)
            .min_width(180.0)
            .default_width(220.0)
            .show(ctx, |ui| {
                ui.heading("Notes");
                ui.separator();

                // Filtre texte
                ui.label("Rechercher :");
                ui.add(TextEdit::singleline(&mut self.state.filtre).hint_text("Titre ou contenu…"));

                // Filtre tag
                ui.label("Filtrer par tag :");
                let tags = self.state.tous_les_tags();
                egui::ComboBox::from_id_salt("filtre_tag")
                    .selected_text(
                        self.state.tag_filtre.clone().unwrap_or_else(|| "Tous".to_string()),
                    )
                    .show_ui(ui, |ui| {
                        if ui
                            .selectable_value(&mut self.state.tag_filtre, None, "Tous")
                            .clicked()
                        {}
                        for tag in &tags {
                            let tag_opt = Some(tag.clone());
                            ui.selectable_value(&mut self.state.tag_filtre, tag_opt, tag);
                        }
                    });

                ui.separator();

                // Liste des notes filtrées
                ScrollArea::vertical().show(ui, |ui| {
                    let notes_filtrees: Vec<(Uuid, String)> = self
                        .state
                        .notes_filtrees()
                        .iter()
                        .map(|n| (n.id, n.titre.clone()))
                        .collect();

                    for (id, titre) in notes_filtrees {
                        let selected = self.selected_note == Some(id);
                        if ui.selectable_label(selected, &titre).clicked() {
                            self.selectionner_note(id);
                        }
                    }
                });
            });
    }

    fn afficher_panneau_central(&mut self, ctx: &Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            if self.selected_note.is_none() {
                ui.centered_and_justified(|ui| {
                    ui.label("Sélectionnez ou créez une note (Ctrl+N)");
                });
                return;
            }

            ui.heading("Édition");
            ui.separator();

            ui.label("Titre :");
            ui.add(
                TextEdit::singleline(&mut self.edit_titre)
                    .hint_text("Titre de la note")
                    .desired_width(f32::INFINITY),
            );

            ui.label("Tags (séparés par virgule) :");
            ui.add(
                TextEdit::singleline(&mut self.edit_tags)
                    .hint_text("ex: travail, important")
                    .desired_width(f32::INFINITY),
            );

            ui.label("Contenu :");
            ScrollArea::vertical().id_salt("contenu_scroll").show(ui, |ui| {
                ui.add(
                    TextEdit::multiline(&mut self.edit_contenu)
                        .hint_text("Contenu de la note…")
                        .desired_width(f32::INFINITY)
                        .desired_rows(20),
                );
            });

            ui.separator();
            ui.horizontal(|ui| {
                if ui.button("💾 Sauvegarder (Ctrl+S)").clicked() {
                    self.sauvegarder_note();
                }
                if ui.button("🗑 Supprimer").clicked() {
                    self.supprimer_note();
                }
            });
        });
    }

    fn afficher_dashboard(&mut self, ctx: &Context) {
        if !self.afficher_dashboard {
            return;
        }
        let total = self.state.notes.len();
        let nb_tags = self.state.tous_les_tags().len();
        let derniere = self
            .state
            .derniere_modification()
            .map(|d| d.format("%d/%m/%Y %H:%M").to_string())
            .unwrap_or_else(|| "—".to_string());

        let mut ouvert = self.afficher_dashboard;
        egui::Window::new("📊 Dashboard")
            .open(&mut ouvert)
            .resizable(false)
            .show(ctx, |ui| {
                egui::Grid::new("dashboard_grid")
                    .num_columns(2)
                    .spacing([20.0, 8.0])
                    .show(ui, |ui| {
                        ui.label("Total de notes :");
                        ui.label(total.to_string());
                        ui.end_row();

                        ui.label("Tags uniques :");
                        ui.label(nb_tags.to_string());
                        ui.end_row();

                        ui.label("Dernière modification :");
                        ui.label(&derniere);
                        ui.end_row();
                    });
            });
        self.afficher_dashboard = ouvert;
    }

    fn gerer_raccourcis(&mut self, ctx: &Context) {
        let ctrl = Modifiers::CTRL;

        if ctx.input_mut(|i| i.consume_key(ctrl, Key::N)) {
            self.nouvelle_note();
        }
        if ctx.input_mut(|i| i.consume_key(ctrl, Key::S)) {
            self.sauvegarder_note();
        }
        if ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape)) {
            self.selected_note = None;
            self.edit_titre.clear();
            self.edit_contenu.clear();
            self.edit_tags.clear();
        }
    }
}

impl eframe::App for NotesApp {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        // Traiter les données reçues du thread REST
        self.traiter_reception_fetch();

        // Raccourcis clavier
        self.gerer_raccourcis(ctx);

        // Panneaux UI (ordre important : top avant bottom avant les panels latéraux)
        self.afficher_bandeau_erreur(ctx);
        self.afficher_top_panel(ctx);
        self.afficher_status_bar(ctx);
        self.afficher_sidebar(ctx);
        self.afficher_panneau_central(ctx);
        self.afficher_dashboard(ctx);

        // Demander un rafraîchissement si import en cours
        if self.rx_fetch.is_some() {
            ctx.request_repaint();
        }
    }
}

// Signature nécessaire pour que le trait Dao soit utilisable avec Box<dyn Dao>
fn _assert_dao_send(_: &dyn Dao) {}
