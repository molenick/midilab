use std::fmt::Display;
use std::path::PathBuf;
use std::sync::Arc;

use eframe::egui::Context;
use midilab::config::AppConfig;
use midilab::manufacturer::akai::mpd226::ColorPattern;
use midilab::manufacturer::akai::mpd226::ColorSequence;
use midilab::manufacturer::akai::mpd226::Global;
use midilab::manufacturer::akai::mpd226::NoteColorMap;
use midilab::manufacturer::akai::mpd226::Preset;
use midilab::manufacturer::akai::mpd226::control::ControlId;
use midilab::manufacturer::akai::mpd226::control::value_kind::AfterTouchKind;
use midilab::manufacturer::akai::mpd226::control::value_kind::PadColor;
use midilab::message::AppMsg;
use midilab::message::UiEffect;
use midilab::message::UiMsg;
use midilab::message::UserMsg;
use midilab::music::generation::PitchPattern;
use midilab::music::generation::ScaleSequence;
use midilab::music::generation::SequenceDirection;
use midilab::music::theory::Octave as MusicOctave;
use midilab::music::theory::PitchClass;
use midilab::music::theory::ScaleKind;
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::sync::mpsc::UnboundedSender;

use crate::akai_mpd226_editor::render::ui;

pub struct AkaiMpd226Editor {
    ui_state: UiState,
    outbox: Vec<UiEffect>,
    app_tx: UnboundedSender<AppMsg>,
    ui_rx: UnboundedReceiver<UiMsg>,
    config: Arc<AppConfig>,
}

impl AkaiMpd226Editor {
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
                UiMsg::UpdatePreset(preset) => {
                    self.ui_state.preset = *preset;
                    self.ui_state.selected_item = None;
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
                UiMsg::DirectoryConfiguredGlobal(path) => {
                    self.ui_state.configured_directory_global = Some(path);
                }
                UiMsg::SaveGlobalDialog(path) => {
                    self.spawn_global_save_dialog_with_path(path);
                }
                UiMsg::LoadGlobalDialog => {
                    self.spawn_global_load_dialog();
                }
                UiMsg::UserMsg(e) => {
                    self.ui_state.user_msg = Some(e);
                }
                UiMsg::UpdateGlobal(global) => {
                    self.ui_state.global = *global;
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
                    global: Box::new(global),
                    path: handle.path().to_path_buf(),
                }));
            }
        });
    }

    pub fn render(&mut self, ctx: &Context) {
        ui(ctx, &mut self.ui_state, &mut self.outbox);

        for msg in self.outbox.drain(..) {
            let _ = self.app_tx.send(msg.as_app_msg());
        }
    }
}

impl eframe::App for AkaiMpd226Editor {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        self.poll_ui_msgs(ctx);
        self.render(ctx);
        ctx.request_repaint_after_secs(0.064);
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum UserSelection {
    Pad { id: ControlId },
    Dial { id: ControlId },
    Fader { id: ControlId },
    Switch { id: ControlId },
    PresetSettings,
    GlobalSettings,
    PadNoteMapping,
    PadOffColorMapping,
    PadOnColorMapping,
}
impl Display for UserSelection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UserSelection::Pad { id } => write!(f, "Edit Pad {}", id),
            UserSelection::Dial { id } => {
                write!(f, "Edit Dial {}", id)
            }
            UserSelection::Fader { id } => {
                write!(f, "Edit Fader {}", id)
            }
            UserSelection::Switch { id } => {
                write!(f, "Edit Switch {}", id)
            }
            UserSelection::PresetSettings => write!(f, "Edit Preset Settings"),
            UserSelection::GlobalSettings => write!(f, "Edit Global Settings"),
            UserSelection::PadNoteMapping => write!(f, "Pad Note Mapping"),
            UserSelection::PadOffColorMapping => write!(f, "Pad Off LED Color Mapping"),
            UserSelection::PadOnColorMapping => write!(f, "Pad On LED Color Mapping"),
        }
    }
}

#[derive(Default)]
pub struct UiState {
    pub selected_item: Option<UserSelection>,
    pub preset: Preset,
    pub global: Global,
    pub note_mapping: NoteMappingState,
    pub off_color_mapping: ColorMappingState,
    pub on_color_mapping: ColorMappingState,
    pub aftertouch_kind: AfterTouchKind,
    pub user_msg: Option<UserMsg>,
    pub configured_directory: Option<PathBuf>,
    pub configured_directory_global: Option<PathBuf>,
}

pub struct NoteMappingState {
    pub pattern: PitchPattern,
    pub starting_from_pad: usize,
    pub tonic_color: PadColor,
    pub color_map: NoteColorMap,
}
impl Default for NoteMappingState {
    fn default() -> Self {
        Self {
            pattern: PitchPattern::Scale(ScaleSequence {
                tonic: PitchClass::C,
                scale: ScaleKind::Chromatic,
                direction: SequenceDirection::Ascending,
                octave: MusicOctave(4),
                length: 64,
            }),
            starting_from_pad: 0,
            tonic_color: PadColor::Red,
            color_map: NoteColorMap::default(),
        }
    }
}

pub struct ColorMappingState {
    pub pattern: ColorPattern,
    pub length: usize,
    pub starting_from_pad: usize,
}
impl Default for ColorMappingState {
    fn default() -> Self {
        let pattern = ColorPattern::Repeating(vec![ColorSequence {
            len: 64,
            color: PadColor::Off,
        }]);

        Self {
            pattern,
            length: 64,
            starting_from_pad: 0,
        }
    }
}
