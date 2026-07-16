use std::path::PathBuf;
use std::sync::Arc;

use eframe::egui::Context;
use midilab::manufacturer::arturia::minilab_mk2::Global;
use midilab::manufacturer::arturia::minilab_mk2::Preset;
use midilab::manufacturer::arturia::minilab_mk2::control::value_kind::MemorySlot;
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::sync::mpsc::UnboundedSender;

use crate::config::AppConfig;
use crate::config::UserSettings;
use crate::message::AppMsg;
use crate::message::UiEffect;
use crate::message::UiMsg;
use crate::message::UserMsg;

pub struct MinilabMk2Editor {
    ui_state: UiState,
    outbox: Vec<UiEffect>,
    app_tx: UnboundedSender<AppMsg>,
    ui_rx: UnboundedReceiver<UiMsg>,
    config: Arc<AppConfig>,
}

impl MinilabMk2Editor {
    pub fn new(
        app_tx: UnboundedSender<AppMsg>,
        ui_rx: UnboundedReceiver<UiMsg>,
        config: Arc<AppConfig>,
    ) -> Self {
        let user_settings = UserSettings {
            auto_sync_enabled: config.user.auto_sync_enabled,
        };
        Self {
            ui_state: UiState {
                user_settings,
                ..Default::default()
            },
            outbox: Vec::new(),
            app_tx,
            ui_rx,
            config,
        }
    }

    fn poll_ui_msgs(&mut self, ctx: &Context) {
        while let Ok(msg) = self.ui_rx.try_recv() {
            match msg {
                UiMsg::UpdatePreset(preset) => {
                    self.ui_state.preset = *preset;
                }
                UiMsg::UpdateGlobal(global) => {
                    self.ui_state.global = *global;
                }
                UiMsg::UserMsg(e) => {
                    self.ui_state.user_msg = Some(e);
                }
                UiMsg::DirectoryConfigured(path) => {
                    self.ui_state.configured_directory = Some(path);
                }
                UiMsg::SavePresetDialog(path) => {
                    self.spawn_preset_save_dialog_with_path(path);
                }
                UiMsg::LoadPresetDialog => {
                    self.spawn_preset_load_dialog();
                }
                UiMsg::SaveGlobalDialog(path) => {
                    self.spawn_global_save_dialog_with_path(path);
                }
                UiMsg::LoadGlobalDialog => {
                    self.spawn_global_load_dialog();
                }
                UiMsg::ShowSettingsModal => {
                    self.ui_state.show_settings = true;
                }
                UiMsg::UpdateUserSettings(settings) => {
                    self.ui_state.user_settings = settings.clone();

                    let config = AppConfig {
                        persistence_path: self.config.persistence_path.clone(),
                        user: settings,
                    };
                    let config_path = AppConfig::config_path().expect("Failed to get config path");
                    self.outbox.push(UiEffect::PersistUserSettings {
                        config,
                        path: config_path,
                    });
                }
                UiMsg::AutoSync => {
                    self.outbox.push(UiEffect::AutoSync);
                }
            }

            ctx.request_repaint();
        }
    }

    fn spawn_preset_load_dialog(&self) {
        let app_tx = self.app_tx.clone();
        let config = self.config.clone();
        tokio::spawn(async move {
            let dialog = rfd::AsyncFileDialog::new()
                .set_title("Load Preset")
                .add_filter("Preset files", &["preset"]);

            let dialog = if let Some(ref dir) = config.persistence_path {
                dialog.set_directory(dir)
            } else {
                dialog
            };

            let handle: Option<_> = dialog.pick_file().await;
            if let Some(handle) = handle {
                let path = handle.path().to_path_buf();
                let _ = app_tx.send(AppMsg::Ui(UiEffect::LoadPresetFromFile { path }));
            }
        });
    }

    fn spawn_preset_save_dialog_with_path(&self, path: PathBuf) {
        let app_tx = self.app_tx.clone();
        let preset = self.ui_state.preset;
        let config = self.config.clone();
        tokio::spawn(async move {
            let dialog = rfd::AsyncFileDialog::new()
                .set_title("Save Preset")
                .set_directory(path.parent().unwrap_or(&path))
                .set_file_name(
                    path.file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_string(),
                )
                .add_filter("Preset files", &["preset"]);

            let dialog = if let Some(ref dir) = config.persistence_path {
                dialog.set_directory(dir)
            } else {
                dialog
            };

            if let Some(handle) = dialog.save_file().await {
                let _ = app_tx.send(AppMsg::Ui(UiEffect::PersistPreset {
                    preset: Box::new(preset),
                    path: handle.path().to_path_buf(),
                }));
            }
        });
    }

    fn spawn_global_load_dialog(&self) {
        let app_tx = self.app_tx.clone();
        let config = self.config.clone();
        tokio::spawn(async move {
            let dialog = rfd::AsyncFileDialog::new()
                .set_title("Load Global")
                .add_filter("Global files", &["global"]);

            let dialog = if let Some(ref dir) = config.persistence_path {
                dialog.set_directory(dir)
            } else {
                dialog
            };

            let handle: Option<_> = dialog.pick_file().await;
            if let Some(handle) = handle {
                let path = handle.path().to_path_buf();
                let _ = app_tx.send(AppMsg::Ui(UiEffect::LoadGlobalFromFile { path }));
            }
        });
    }

    fn spawn_global_save_dialog_with_path(&self, path: PathBuf) {
        let app_tx = self.app_tx.clone();
        let global = self.ui_state.global;
        let config = self.config.clone();
        tokio::spawn(async move {
            let dialog = rfd::AsyncFileDialog::new()
                .set_title("Save Global")
                .set_directory(path.parent().unwrap_or(&path))
                .set_file_name(
                    path.file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_string(),
                )
                .add_filter("Global files", &["global"]);

            let dialog = if let Some(ref dir) = config.persistence_path {
                dialog.set_directory(dir)
            } else {
                dialog
            };

            if let Some(handle) = dialog.save_file().await {
                let _ = app_tx.send(AppMsg::Ui(UiEffect::PersistGlobal {
                    global,
                    path: handle.path().to_path_buf(),
                }));
            }
        });
    }

    #[doc(hidden)]
    pub fn set_tab_for_test(&mut self, tab: EditorTab) {
        self.ui_state.editor_tab = tab;
    }

    pub fn render(&mut self, ui: &mut eframe::egui::Ui) {
        crate::render::ui(ui, &mut self.ui_state, &mut self.outbox);

        for msg in self.outbox.drain(..) {
            let _ = self.app_tx.send(AppMsg::Ui(msg));
        }
    }
}

impl eframe::App for MinilabMk2Editor {
    fn ui(&mut self, ui: &mut eframe::egui::Ui, _frame: &mut eframe::Frame) {
        self.poll_ui_msgs(ui.ctx());
        self.render(ui);
        ui.ctx().request_repaint_after_secs(0.064);
    }
}

#[derive(Default)]
pub struct UiState {
    pub preset: Preset,
    pub global: Global,
    pub user_msg: Option<UserMsg>,
    pub configured_directory: Option<PathBuf>,
    pub user_settings: UserSettings,
    pub show_settings: bool,
    pub editor_tab: EditorTab,
    pub selected_memory_slot: MemorySlot,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EditorTab {
    #[default]
    Knobs,
    Pads,
    Controller,
    Global,
    Memory,
}

impl EditorTab {
    pub const ALL: [EditorTab; 5] = [
        EditorTab::Knobs,
        EditorTab::Pads,
        EditorTab::Controller,
        EditorTab::Global,
        EditorTab::Memory,
    ];

    pub fn label(self) -> &'static str {
        match self {
            EditorTab::Knobs => "KNOBS",
            EditorTab::Pads => "PADS",
            EditorTab::Controller => "CONTROLLER",
            EditorTab::Global => "GLOBAL",
            EditorTab::Memory => "MEMORY",
        }
    }
}
