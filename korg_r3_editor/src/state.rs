use std::path::PathBuf;
use std::sync::Arc;

use eframe::egui::Context;
use midilab::manufacturer::korg::r3::wrappers::FormantMotion;
use midilab::manufacturer::korg::r3::wrappers::Global;
use midilab::manufacturer::korg::r3::wrappers::Program;
use midilab::manufacturer::korg::r3::wrappers::ProgramSlot;
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::sync::mpsc::UnboundedSender;

use crate::config::AppConfig;
use crate::config::UserSettings;
use crate::message::AppMsg;
use crate::message::UiEffect;
use crate::message::UiMsg;
use crate::message::UserMsg;

pub struct KorgR3Editor {
    ui_state: UiState,
    outbox: Vec<UiEffect>,
    app_tx: UnboundedSender<AppMsg>,
    ui_rx: UnboundedReceiver<UiMsg>,
    config: Arc<AppConfig>,
}

impl KorgR3Editor {
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
                formant_frame_count: 60,
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
                UiMsg::UpdateProgram(program) => {
                    self.ui_state.program = *program;
                    self.ui_state.program.slot = self.ui_state.selected_slot;
                }
                UiMsg::UpdateGlobal(global) => {
                    self.ui_state.global = *global;
                }
                UiMsg::UpdateFormantMotion(motion) => {
                    if let Some(n) = motion.motion_no {
                        self.ui_state.selected_formant_no = n;
                        if let Some(slot) = self.ui_state.formant_motions.get_mut(n as usize) {
                            *slot = Some((*motion).clone());
                        }
                    }
                    self.ui_state.formant_motion = Some(*motion);
                }
                UiMsg::UserMsg(e) => {
                    self.ui_state.user_msg = Some(e);
                }
                UiMsg::DirectoryConfigured(path) => {
                    self.ui_state.configured_directory = Some(path);
                }
                UiMsg::SaveProgramDialog(path) => {
                    self.spawn_program_save_dialog_with_path(path);
                }
                UiMsg::LoadProgramDialog => {
                    self.spawn_program_load_dialog();
                }
                UiMsg::SaveGlobalDialog(path) => {
                    self.spawn_global_save_dialog_with_path(path);
                }
                UiMsg::LoadGlobalDialog => {
                    self.spawn_global_load_dialog();
                }
                UiMsg::SaveFormantMotionDialog(path) => {
                    self.spawn_formant_save_dialog_with_path(path);
                }
                UiMsg::LoadFormantMotionDialog => {
                    self.spawn_formant_load_dialog();
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
                    self.outbox.push(UiEffect::DumpCurrentProgram);
                    self.outbox.push(UiEffect::RequestGlobalFromDevice);
                }
            }

            ctx.request_repaint();
        }
    }

    fn spawn_program_load_dialog(&self) {
        let app_tx = self.app_tx.clone();
        let config = self.config.clone();
        tokio::spawn(async move {
            let dialog = rfd::AsyncFileDialog::new()
                .set_title("Load Program")
                .add_filter("Program files", &["program"]);

            let dialog = if let Some(ref dir) = config.persistence_path {
                dialog.set_directory(dir)
            } else {
                dialog
            };

            let handle: Option<_> = dialog.pick_file().await;
            if let Some(handle) = handle {
                let path = handle.path().to_path_buf();
                let _ = app_tx.send(AppMsg::Ui(UiEffect::LoadProgramFromFile { path }));
            }
        });
    }

    fn spawn_program_save_dialog_with_path(&self, path: PathBuf) {
        let app_tx = self.app_tx.clone();
        let program = self.ui_state.program.clone();
        let config = self.config.clone();
        tokio::spawn(async move {
            let dialog = rfd::AsyncFileDialog::new()
                .set_title("Save Program")
                .set_directory(path.parent().unwrap_or(&path))
                .set_file_name(
                    path.file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_string(),
                )
                .add_filter("Program files", &["program"]);

            let dialog = if let Some(ref dir) = config.persistence_path {
                dialog.set_directory(dir)
            } else {
                dialog
            };

            if let Some(handle) = dialog.save_file().await {
                let _ = app_tx.send(AppMsg::Ui(UiEffect::PersistProgram {
                    program: Box::new(program),
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
        let global = self.ui_state.global.clone();
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

    fn spawn_formant_load_dialog(&self) {
        let app_tx = self.app_tx.clone();
        let config = self.config.clone();
        tokio::spawn(async move {
            let dialog = rfd::AsyncFileDialog::new()
                .set_title("Load Formant Motion")
                .add_filter("Formant motion files", &["formant"]);

            let dialog = if let Some(ref dir) = config.persistence_path {
                dialog.set_directory(dir)
            } else {
                dialog
            };

            let handle: Option<_> = dialog.pick_file().await;
            if let Some(handle) = handle {
                let path = handle.path().to_path_buf();
                let _ = app_tx.send(AppMsg::Ui(UiEffect::LoadFormantMotionFromFile { path }));
            }
        });
    }

    fn spawn_formant_save_dialog_with_path(&self, path: PathBuf) {
        let app_tx = self.app_tx.clone();
        let motion = self.ui_state.formant_motion.clone();
        let config = self.config.clone();
        tokio::spawn(async move {
            let Some(motion) = motion else {
                return;
            };
            let dialog = rfd::AsyncFileDialog::new()
                .set_title("Save Formant Motion")
                .set_directory(path.parent().unwrap_or(&path))
                .set_file_name(
                    path.file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_string(),
                )
                .add_filter("Formant motion files", &["formant"]);

            let dialog = if let Some(ref dir) = config.persistence_path {
                dialog.set_directory(dir)
            } else {
                dialog
            };

            if let Some(handle) = dialog.save_file().await {
                let _ = app_tx.send(AppMsg::Ui(UiEffect::PersistFormantMotion {
                    motion,
                    path: handle.path().to_path_buf(),
                }));
            }
        });
    }

    #[doc(hidden)]
    pub fn set_tab_for_test(&mut self, tab: EditorTab) {
        self.ui_state.editor_tab = tab;
    }

    pub fn render(&mut self, ctx: &Context) {
        crate::render::ui(ctx, &mut self.ui_state, &mut self.outbox);

        for msg in self.outbox.drain(..) {
            let _ = self.app_tx.send(AppMsg::Ui(msg));
        }
    }
}

impl eframe::App for KorgR3Editor {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        self.poll_ui_msgs(ctx);
        self.render(ctx);
        ctx.request_repaint_after_secs(0.064);
    }
}

#[derive(Default)]
pub struct UiState {
    pub program: Program,
    pub global: Global,
    pub user_msg: Option<UserMsg>,
    pub configured_directory: Option<PathBuf>,
    pub user_settings: UserSettings,
    pub show_settings: bool,
    pub editor_tab: EditorTab,
    pub selected_slot: ProgramSlot,
    pub selected_timbre: TimbreSelect,
    pub live_edit: bool,
    pub formant_motion: Option<FormantMotion>,
    pub formant_motions: [Option<FormantMotion>; 16],
    pub selected_formant_no: u8,
    pub formant_edit: bool,
    pub formant_selected_band: u8,
    pub formant_selected_frame: usize,
    pub formant_frame_count: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EditorTab {
    #[default]
    Program,
    Synth,
    Vocoder,
    Fx,
    Arp,
    Global,
    Formant,
}

impl EditorTab {
    pub const ALL: [EditorTab; 7] = [
        EditorTab::Program,
        EditorTab::Synth,
        EditorTab::Vocoder,
        EditorTab::Fx,
        EditorTab::Arp,
        EditorTab::Global,
        EditorTab::Formant,
    ];

    pub fn label(self) -> &'static str {
        match self {
            EditorTab::Program => "PROGRAM",
            EditorTab::Synth => "SYNTH",
            EditorTab::Vocoder => "VOCODER",
            EditorTab::Fx => "FX",
            EditorTab::Arp => "ARP",
            EditorTab::Global => "GLOBAL",
            EditorTab::Formant => "FORMANT",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TimbreSelect {
    #[default]
    One,
    Two,
}
