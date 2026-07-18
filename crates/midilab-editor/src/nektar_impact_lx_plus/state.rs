use std::path::PathBuf;
use std::sync::Arc;

use eframe::egui::Context;
use midilab::manufacturer::nektar::impact_lx_plus::Dump;
use midilab::manufacturer::nektar::impact_lx_plus::control::PadMapId;
use midilab::manufacturer::nektar::impact_lx_plus::control::PresetId;
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::sync::mpsc::UnboundedSender;

use crate::nektar_impact_lx_plus::config::AppConfig;
use crate::nektar_impact_lx_plus::message::AppMsg;
use crate::nektar_impact_lx_plus::message::UiEffect;
use crate::nektar_impact_lx_plus::message::UiMsg;
use crate::nektar_impact_lx_plus::message::UserMsg;

pub struct ImpactLxPlusEditor {
    ui_state: UiState,
    outbox: Vec<UiEffect>,
    app_tx: UnboundedSender<AppMsg>,
    ui_rx: UnboundedReceiver<UiMsg>,
    config: Arc<AppConfig>,
}

impl ImpactLxPlusEditor {
    pub fn new(
        app_tx: UnboundedSender<AppMsg>,
        ui_rx: UnboundedReceiver<UiMsg>,
        config: Arc<AppConfig>,
    ) -> Self {
        Self {
            ui_state: UiState::default(),
            outbox: Vec::new(),
            app_tx,
            ui_rx,
            config,
        }
    }

    fn poll_ui_msgs(&mut self, ctx: &Context) {
        while let Ok(msg) = self.ui_rx.try_recv() {
            match msg {
                UiMsg::UpdateDump(dump) => {
                    self.ui_state.dump = *dump;
                }
                UiMsg::UserMsg(e) => {
                    self.ui_state.user_msg = Some(e);
                }
                UiMsg::SaveDumpDialog(path) => {
                    self.spawn_dump_save_dialog_with_path(path);
                }
                UiMsg::LoadDumpDialog => {
                    self.spawn_dump_load_dialog();
                }
            }

            ctx.request_repaint();
        }
    }

    fn spawn_dump_load_dialog(&self) {
        let app_tx = self.app_tx.clone();
        let config = self.config.clone();
        tokio::spawn(async move {
            let dialog = rfd::AsyncFileDialog::new()
                .set_title("Load Dump")
                .add_filter("Dump files", &["dump"]);

            let dialog = if let Some(ref dir) = config.persistence_path {
                dialog.set_directory(dir)
            } else {
                dialog
            };

            let handle: Option<_> = dialog.pick_file().await;
            if let Some(handle) = handle {
                let path = handle.path().to_path_buf();
                let _ = app_tx.send(AppMsg::Ui(UiEffect::LoadDumpFromFile { path }));
            }
        });
    }

    fn spawn_dump_save_dialog_with_path(&self, path: PathBuf) {
        let app_tx = self.app_tx.clone();
        let dump = self.ui_state.dump;
        let config = self.config.clone();
        tokio::spawn(async move {
            let dialog = rfd::AsyncFileDialog::new()
                .set_title("Save Dump")
                .set_directory(path.parent().unwrap_or(&path))
                .set_file_name(
                    path.file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_string(),
                )
                .add_filter("Dump files", &["dump"]);

            let dialog = if let Some(ref dir) = config.persistence_path {
                dialog.set_directory(dir)
            } else {
                dialog
            };

            if let Some(handle) = dialog.save_file().await {
                let _ = app_tx.send(AppMsg::Ui(UiEffect::PersistDump {
                    dump: Box::new(dump),
                    path: handle.path().to_path_buf(),
                }));
            }
        });
    }

    pub fn render(&mut self, ui: &mut eframe::egui::Ui) {
        crate::nektar_impact_lx_plus::render::ui(ui, &mut self.ui_state, &mut self.outbox);

        for msg in self.outbox.drain(..) {
            let _ = self.app_tx.send(AppMsg::Ui(msg));
        }
    }
}

impl eframe::App for ImpactLxPlusEditor {
    fn ui(&mut self, ui: &mut eframe::egui::Ui, _frame: &mut eframe::Frame) {
        self.poll_ui_msgs(ui.ctx());
        self.render(ui);
        ui.ctx().request_repaint_after_secs(0.064);
    }
}

pub struct UiState {
    pub dump: Dump,
    pub user_msg: Option<UserMsg>,
    pub editor_tab: EditorTab,
    pub selected_preset: PresetId,
    pub selected_pad_map: PadMapId,
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            dump: Dump::default(),
            user_msg: None,
            editor_tab: EditorTab::default(),
            selected_preset: PresetId::Preset1,
            selected_pad_map: PadMapId::Map1,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EditorTab {
    #[default]
    Presets,
    Pads,
    Controller,
    Settings,
}

impl EditorTab {
    pub const ALL: [EditorTab; 4] = [
        EditorTab::Presets,
        EditorTab::Pads,
        EditorTab::Controller,
        EditorTab::Settings,
    ];

    pub fn label(self) -> &'static str {
        match self {
            EditorTab::Presets => "PRESETS",
            EditorTab::Pads => "PADS",
            EditorTab::Controller => "WHEELS & TRANSPORT",
            EditorTab::Settings => "SETTINGS",
        }
    }
}
