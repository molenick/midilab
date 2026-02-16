use std::fmt::Display;
use std::ops::RangeInclusive;
use std::path::PathBuf;

use eframe::egui;
use eframe::egui::Align;
use eframe::egui::CentralPanel;
use eframe::egui::Color32;
use eframe::egui::ComboBox;
use eframe::egui::Context;
use eframe::egui::DragValue;
use eframe::egui::Grid;
use eframe::egui::Layout;
use eframe::egui::MenuBar;
use eframe::egui::ScrollArea;
use eframe::egui::TextEdit;
use eframe::egui::TopBottomPanel;
use eframe::egui::Ui;
use eframe::egui::Vec2;
use eframe::egui::containers::Modal;
use eframe::egui::vec2;
use midilab::IntoEnumIterator;
use midilab::manufacturer::akai::mpd226::ColorPattern;
use midilab::manufacturer::akai::mpd226::ColorSequence;
use midilab::manufacturer::akai::mpd226::Global;
use midilab::manufacturer::akai::mpd226::NoteColorMap;
use midilab::manufacturer::akai::mpd226::Preset;
use midilab::manufacturer::akai::mpd226::control::Dial;
use midilab::manufacturer::akai::mpd226::control::Fader;
use midilab::manufacturer::akai::mpd226::control::Pad;
use midilab::manufacturer::akai::mpd226::control::PresetSettings;
use midilab::manufacturer::akai::mpd226::control::Switch;
use midilab::manufacturer::akai::mpd226::control::value_kind::PadColor;
use midilab::manufacturer::akai::mpd226::control::value_kind::PresetName;
use midilab::manufacturer::akai::mpd226::repository::DialRepository;
use midilab::manufacturer::akai::mpd226::repository::FaderRepository;
use midilab::manufacturer::akai::mpd226::repository::PadRepository;
use midilab::manufacturer::akai::mpd226::repository::SwitchRepository;
use midilab::message::AppMsg;
use midilab::message::PendingFileAction;
use midilab::message::UiEffect;
use midilab::message::UiMsg;
use midilab::message::UserMsg;
use midilab::midi::MidiValue;
use midilab::midi::Note;
use midilab::music::ChordRowSequence;
use midilab::music::NotePattern;
use midilab::music::Octave;
use midilab::music::PitchClass;
use midilab::music::ScaleKind;
use midilab::music::ScaleSequence;
use midilab::music::SequenceDirection;
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::sync::mpsc::UnboundedSender;

const APP_X: f32 = 800.;
const APP_Y: f32 = 600.;
pub const APP_DIMENSIONS: Vec2 = Vec2 { x: APP_X, y: APP_Y };

const DEFAULT_CONTROL_X: f32 = 42.;

const PAD_X: f32 = 48.;
const PAD_Y: f32 = 48.;
const PAD_DIMENSIONS: Vec2 = Vec2 { x: PAD_X, y: PAD_Y };

const DIAL_X: f32 = DEFAULT_CONTROL_X;
const DIAL_Y: f32 = 40.;
const DIAL_DIMENSIONS: Vec2 = Vec2 {
    x: DIAL_X,
    y: DIAL_Y,
};

const FADER_X: f32 = DEFAULT_CONTROL_X;
const FADER_Y: f32 = 80.;
const FADER_DIMENSIONS: Vec2 = Vec2 {
    x: FADER_X,
    y: FADER_Y,
};

const SWITCH_X: f32 = DEFAULT_CONTROL_X;
const SWITCH_Y: f32 = 24.;
const SWITCH_DIMENSIONS: Vec2 = Vec2 {
    x: SWITCH_X,
    y: SWITCH_Y,
};

const BANKS: [&str; 4] = ["A", "B", "C", "D"];
const CONTROL_BANKS: [&str; 3] = ["A", "B", "C"];

// There is some extra space I haven't tracked down yet that happens
// to be 24., which is also half of the control x.
const CONTROL_BANK_X_ADJUSTMENT: f32 = DEFAULT_CONTROL_X * 0.5;
const CONTROL_BANK_X: f32 = DEFAULT_CONTROL_X * 4. + CONTROL_BANK_X_ADJUSTMENT;

#[expect(
    unused,
    reason = "its hip, its cool, we're going to want to make the app look nice one day"
)]
fn spacing(n: i32) -> f32 {
    let phi = (1.0 + 5_f32.sqrt()) / 2.0;
    8_f32 * phi.powi(n)
}

pub struct AkaiMpd226Editor {
    ui_state: UiState,
    outbox: Vec<AppMsg>,
    app_tx: UnboundedSender<AppMsg>,
    ui_rx: UnboundedReceiver<UiMsg>,
}

impl AkaiMpd226Editor {
    pub fn new(app_tx: UnboundedSender<AppMsg>, ui_rx: UnboundedReceiver<UiMsg>) -> Self {
        Self {
            ui_state: UiState::default(),
            outbox: Vec::new(),
            app_tx,
            ui_rx,
        }
    }

    fn poll_ui_msgs(&mut self, ctx: &Context) {
        while let Ok(msg) = self.ui_rx.try_recv() {
            match msg {
                UiMsg::UpdatePreset(preset) => {
                    self.ui_state.preset = *preset;
                    self.ui_state.selected_item = None;
                }

                UiMsg::UserMsg(e) => {
                    self.ui_state.user_msg = Some(e);
                }

                UiMsg::ShowDirectoryPicker { for_action } => {
                    self.ui_state.pending_action = Some(for_action);
                    self.spawn_directory_picker();
                }

                UiMsg::DirectoryConfigured(path) => {
                    self.ui_state.configured_directory = Some(path);
                    self.ui_state.pending_action = None;
                }

                UiMsg::UpdateGlobal(global) => {
                    self.ui_state.global = *global;
                }
            }

            ctx.request_repaint();
        }
    }

    fn spawn_directory_picker(&self) {
        let app_tx = self.app_tx.clone();
        tokio::spawn(async move {
            let dialog = rfd::AsyncFileDialog::new()
                .set_title("Select Preset Save Directory")
                .pick_folder()
                .await;
            if let Some(handle) = dialog {
                let _ = app_tx.send(AppMsg::Ui(UiEffect::PresetDirectorySelected(
                    handle.path().to_path_buf(),
                )));
            }
        });
    }

    pub fn render_ui(&mut self, ctx: &Context) {
        TopBottomPanel::top("top_panel").show(ctx, |ui| {
            MenuBar::new().ui(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Quit").clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                });
                ui.menu_button("Edit", |ui| {
                    ui.menu_button("Pad Mapping", |ui| {
                        if ui.button("Notes").clicked() {
                            self.ui_state.selected_item = Some(UserSelection::PadNoteMapping);
                        }

                        if ui.button("LED Off Color").clicked() {
                            self.ui_state.selected_item = Some(UserSelection::PadOffColorMapping);
                        }

                        if ui.button("LED On Color").clicked() {
                            self.ui_state.selected_item = Some(UserSelection::PadOnColorMapping);
                        }
                    });

                    if ui.button("Edit Preset settings").clicked() {
                        self.ui_state.selected_item = Some(UserSelection::PresetSettings);
                    }
                    if ui.button("Edit Global settings").clicked() {
                        self.ui_state.selected_item = Some(UserSelection::GlobalSettings);
                    }

                    if ui.button("Blank Preset").clicked() {
                        self.ui_state.preset = Preset::blank();
                    }

                    if ui.button("Default Preset").clicked() {
                        self.ui_state.preset = Preset::default();
                    }
                });
            });
        });

        if self.ui_state.selected_item.is_some() {
            render_modal_editor(ctx, &mut self.ui_state);
        } else {
            CentralPanel::default().show(ctx, |ui| {
                render_editor_actions(ui, &mut self.ui_state, &mut self.outbox);

                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        render_controls(ui, &mut self.ui_state);
                    });
                });
            });
        }

        for msg in self.outbox.drain(..) {
            let _ = self.app_tx.send(msg);
        }
    }
}

impl eframe::App for AkaiMpd226Editor {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        self.poll_ui_msgs(ctx);
        self.render_ui(ctx);
        ctx.request_repaint_after_secs(0.064);
    }
}

mod accessibility {
    use super::*;

    pub fn draw_focus_indicator(
        ui: &egui::Ui,
        rect: egui::Rect,
        has_focus: bool,
        corner_radius: f32,
    ) {
        if has_focus {
            ui.painter().rect_stroke(
                rect.expand(2.0),
                corner_radius,
                egui::Stroke::new(2.0, egui::Color32::from_rgb(100, 150, 255)),
                egui::StrokeKind::Outside,
            );
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum UserSelection {
    Pad { id: usize },
    Dial { id: usize },
    Fader { id: usize },
    Switch { id: usize },
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
                let bank = CONTROL_BANKS[id / 4];
                let num = (id % 4) + 1;
                write!(f, "Edit Dial {}{}", bank, num)
            }
            UserSelection::Fader { id } => {
                let bank = CONTROL_BANKS[id / 4];
                let num = (id % 4) + 1;
                write!(f, "Edit Fader {}{}", bank, num)
            }
            UserSelection::Switch { id } => {
                let bank = CONTROL_BANKS[id / 4];
                let num = (id % 4) + 1;
                write!(f, "Edit Switch {}{}", bank, num)
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
    pub user_msg: Option<UserMsg>,
    pub pending_action: Option<PendingFileAction>,
    pub configured_directory: Option<PathBuf>,
}

pub struct NoteMappingState {
    pub pattern: NotePattern,
    pub starting_from_pad: usize,
    pub tonic_color: PadColor,
    pub color_map: NoteColorMap,
}
impl Default for NoteMappingState {
    fn default() -> Self {
        Self {
            pattern: NotePattern::Scale(ScaleSequence {
                tonic: PitchClass::C,
                scale: ScaleKind::Chromatic,
                direction: SequenceDirection::Ascending,
                octave: Octave::O4,
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

fn render_editor_actions(ui: &mut Ui, ui_state: &mut UiState, outbox: &mut Vec<AppMsg>) {
    ui.vertical(|ui| {
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.label("Preset Quick Actions");
                ui.horizontal(|ui| {
                    row_edit_enum(ui, "Slot", &mut ui_state.preset.settings.preset_slot);

                    if ui.button("Dump").clicked() {
                        ui_state.user_msg = None;
                        outbox.push(AppMsg::Ui(UiEffect::DumpPreset(
                            ui_state.preset.settings.preset_slot,
                        )));
                    }

                    if ui.button("Write").clicked() {
                        ui_state.user_msg = None;
                        outbox.push(AppMsg::Ui(UiEffect::WritePreset(Box::new(ui_state.preset))));
                    }
                });

                ui.separator();

                ui.horizontal(|ui| {
                    ui.label("Status:");

                    if let Some(status) = &ui_state.user_msg {
                        let color = match status.kind {
                            midilab::message::UserMsgKind::Status => Color32::GREEN,
                            midilab::message::UserMsgKind::Error => Color32::RED,
                        };

                        ui.colored_label(color, &status.msg);
                    } else {
                        ui.label("None");
                    }
                });

                ui.separator();
            });
        });
    });
}

fn render_preset_settings(ui: &mut Ui, preset_settings: &mut PresetSettings) {
    Grid::new("preset_settings_grid")
        .striped(true)
        .show(ui, |ui| {
            row_edit_enum(ui, "Preset Slot", &mut preset_settings.preset_slot);
            row_edit_preset_name(ui, "Preset Name", &mut preset_settings.preset_name);
            row_edit_u16_clamped(ui, "Tempo", &mut preset_settings.tempo.0, 30..=300);
            row_edit_enum(ui, "Division", &mut preset_settings.time_division);
            row_edit_enum(ui, "Div Switch", &mut preset_settings.time_division_switch);
            row_edit_enum(ui, "Note Repeat", &mut preset_settings.note_repeat_switch);
            row_edit_u8_clamped(ui, "Gate", &mut preset_settings.gate.value, 1..=99);
            row_edit_enum(ui, "Swing", &mut preset_settings.swing);
            row_edit_enum(ui, "Transport", &mut preset_settings.transport);
        });
}

fn render_global_settings(ui: &mut Ui, ui_state: &mut UiState) {
    Grid::new("global_settings_grid")
        .striped(true)
        .show(ui, |ui| {
            row_edit_enum(ui, "Common Channel", &mut ui_state.global.common_channel);
            row_edit_u8_clamped(
                ui,
                "LCD Contrast",
                &mut ui_state.global.lcd_contrast,
                0..=100,
            );
            row_edit_enum(ui, "Tap Average", &mut ui_state.global.tap_average);
            row_edit_enum(ui, "Tempo LED", &mut ui_state.global.tempo_led);
            row_edit_enum(ui, "Note Display", &mut ui_state.global.note_display);
            row_edit_u8_clamped(
                ui,
                "Pad Threshold*",
                &mut ui_state.global.pad_threshold,
                0..=9,
            );
            row_edit_enum(ui, "Pad Curve", &mut ui_state.global.pad_curve);
            row_edit_u8_clamped(ui, "Pad Gain", &mut ui_state.global.pad_gain, 0..=20);
            row_edit_enum(ui, "MIDI Clock", &mut ui_state.global.midi_clock);
        });
}

fn render_note_mapping(ui: &mut Ui, ui_state: &mut UiState) {
    ui.vertical(|ui| {
        ui.label("Pattern");
        ComboBox::from_id_salt("note_pattern_kind")
            .selected_text(ui_state.note_mapping.pattern.to_string())
            .show_ui(ui, |ui| {
                ui.selectable_value(
                    &mut ui_state.note_mapping.pattern,
                    NotePattern::Scale(ScaleSequence::default()),
                    "Scale",
                );
                ui.selectable_value(
                    &mut ui_state.note_mapping.pattern,
                    NotePattern::ChordRow(ChordRowSequence::default()),
                    "Chord Row",
                );
            });

        match &mut ui_state.note_mapping.pattern {
            NotePattern::Scale(seq) => {
                ui.horizontal(|ui| {
                    ui.label("Tonic");
                    enum_combo_box(ui, "note_mapping_tonic", &mut seq.tonic, None);
                });

                ui.horizontal(|ui| {
                    ui.label("Scale");
                    enum_combo_box(ui, "note_mapping_scale", &mut seq.scale, None);
                });

                ui.horizontal(|ui| {
                    ui.label("Octave");
                    enum_combo_box(ui, "note_mapping_octave", &mut seq.octave, None);
                });

                ui.horizontal(|ui| {
                    ui.label("Direction");
                    enum_combo_box(ui, "note_mapping_direction", &mut seq.direction, None);
                });

                ui.horizontal(|ui| {
                    ui.label("Length");
                    ui.add(DragValue::new(&mut seq.length).range(1..=64))
                });
            }

            NotePattern::ChordRow(seq) => {
                ui.horizontal(|ui| {
                    ui.label("Tonic");
                    enum_combo_box(ui, "chord_row_tonic", &mut seq.tonic, None);
                });

                ui.horizontal(|ui| {
                    ui.label("Scale");
                    enum_combo_box(ui, "chord_row_scale", &mut seq.scale, None);
                });

                ui.horizontal(|ui| {
                    ui.label("Octave");
                    enum_combo_box(ui, "chord_row_octave", &mut seq.octave, None);
                });

                ui.horizontal(|ui| {
                    ui.label("Voicing");
                    enum_combo_box(ui, "chord_row_voicing", &mut seq.voicing, None);
                });

                ui.horizontal(|ui| {
                    ui.label("Direction");
                    enum_combo_box(ui, "chord_row_direction", &mut seq.direction, None);
                });

                ui.horizontal(|ui| {
                    ui.label("Length");
                    ui.add(DragValue::new(&mut seq.length).range(1..=64))
                });
            }
        }

        ui.horizontal(|ui| {
            ui.label("Starting from Pad");
            ui.add(DragValue::new(&mut ui_state.note_mapping.starting_from_pad).range(0..=63))
        });

        render_note_color_map_editor(ui, &mut ui_state.note_mapping.color_map);

        let resp = ui.button("Set pattern");
        if resp.clicked() {
            let pattern = ui_state.note_mapping.pattern;
            ui_state.preset.pads.set_note_pattern_with_off_colors(
                ui_state.note_mapping.starting_from_pad,
                pattern,
                ui_state.note_mapping.color_map.clone(),
            );
        }
    });
}

fn render_note_color_map_editor(ui: &mut Ui, color_map: &mut NoteColorMap) {
    Grid::new("note_color_map_grid")
        .striped(true)
        .show(ui, |ui| {
            for pitch_class in PitchClass::iter() {
                ui.label(pitch_class.to_string());

                let current_color = color_map
                    .0
                    .get(&pitch_class)
                    .copied()
                    .unwrap_or(PadColor::Off);

                let (r, g, b) = *current_color.as_rgb_color();
                let swatch_size = vec2(24.0, 18.0);
                let (swatch_rect, _) = ui.allocate_exact_size(swatch_size, egui::Sense::hover());
                ui.painter()
                    .rect_filled(swatch_rect, 3.0, Color32::from_rgb(r, g, b));

                let mut selected_color = current_color;
                ComboBox::from_id_salt(format!("color_map_{:?}", pitch_class))
                    .selected_text(selected_color.to_string())
                    .width(100.0)
                    .show_ui(ui, |ui| {
                        for color in PadColor::iter() {
                            ui.selectable_value(&mut selected_color, color, color.to_string());
                        }
                    });

                if selected_color != current_color {
                    color_map.0.insert(pitch_class, selected_color);
                }

                ui.end_row();
            }
        });
}

fn render_off_color_mapping(ui: &mut Ui, ui_state: &mut UiState) {
    render_color_pattern_editor(ui, "off", &mut ui_state.off_color_mapping.pattern);

    ui.horizontal(|ui| {
        ui.label("Start Pad");
        ui.add(DragValue::new(&mut ui_state.off_color_mapping.starting_from_pad).range(0..=63));
    });

    ui.horizontal(|ui| {
        ui.label("Length");
        ui.add(DragValue::new(&mut ui_state.off_color_mapping.length).range(1..=64));
    });

    if ui.button("Apply").clicked() {
        ui_state.preset.pads.set_off_color_pattern(
            ui_state.off_color_mapping.starting_from_pad,
            ui_state.off_color_mapping.length,
            ui_state.off_color_mapping.pattern.clone(),
        );
    }
}

fn render_on_color_mapping(ui: &mut Ui, ui_state: &mut UiState) {
    render_color_pattern_editor(ui, "on", &mut ui_state.on_color_mapping.pattern);

    ui.horizontal(|ui| {
        ui.label("Start Pad");
        ui.add(DragValue::new(&mut ui_state.on_color_mapping.starting_from_pad).range(0..=63));
    });

    ui.horizontal(|ui| {
        ui.label("Length");
        ui.add(DragValue::new(&mut ui_state.on_color_mapping.length).range(1..=64));
    });

    if ui.button("Apply").clicked() {
        ui_state.preset.pads.set_on_color_pattern(
            ui_state.on_color_mapping.starting_from_pad,
            ui_state.on_color_mapping.length,
            ui_state.on_color_mapping.pattern.clone(),
        );
    }
}

fn render_color_pattern_editor(ui: &mut Ui, id_prefix: &str, pattern: &mut ColorPattern) {
    match pattern {
        ColorPattern::Repeating(sequences) => {
            if ui.button("+ Add").clicked() {
                sequences.push(ColorSequence {
                    len: 4,
                    color: PadColor::Off,
                });
            }

            let mut to_remove: Option<usize> = None;
            for (idx, seq) in sequences.iter_mut().enumerate() {
                ui.horizontal(|ui| {
                    enum_combo_box(
                        ui,
                        format!("{}_{}_color", id_prefix, idx),
                        &mut seq.color,
                        Some(80.0),
                    );

                    ui.label("x");
                    ui.add(DragValue::new(&mut seq.len).range(1..=64));

                    if ui.button("X").clicked() {
                        to_remove = Some(idx);
                    }
                });
            }

            if let Some(idx) = to_remove {
                sequences.remove(idx);
            }
        }
    }
}

fn render_all_pad_banks(
    ui: &mut Ui,
    selected_item: &mut Option<UserSelection>,
    pad_repo: PadRepository,
) {
    let banks: Vec<Vec<Pad>> = pad_repo
        .pads
        .chunks(16)
        .map(|chunk| chunk.to_vec())
        .collect();

    ui.horizontal(|ui| {
        ScrollArea::horizontal().show(ui, |ui| {
            for (bank_id, bank) in banks.into_iter().enumerate() {
                let bank_label = BANKS[bank_id].to_string();
                render_pad_bank(ui, selected_item, bank, bank_label);
            }
        });
    });
}

fn render_pad_bank(
    ui: &mut Ui,
    selected_item: &mut Option<UserSelection>,
    pads: Vec<Pad>,
    label: String,
) {
    ui.vertical(|ui| {
        ui.label(format!("Pad Bank {label}"));

        let grid_rect = ui
            .allocate_exact_size(Vec2::new(PAD_X * 4., PAD_Y * 4.), egui::Sense::hover())
            .0;
        let top_left = grid_rect.min;

        for (i, pad) in pads.into_iter().enumerate() {
            let col = i % 4;
            let row = i / 4;
            let visual_row = 3 - row;

            let x = top_left.x + col as f32 * (PAD_X);
            let y = top_left.y + visual_row as f32 * (PAD_Y);

            let rect = egui::Rect::from_min_size(egui::Pos2::new(x, y), PAD_DIMENSIONS);

            let mut child_ui = ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(rect)
                    .layout(egui::Layout::default()),
            );
            render_pad(&mut child_ui, selected_item, pad);
        }
    });
}

fn render_pad(ui: &mut Ui, selected_item: &mut Option<UserSelection>, pad: Pad) {
    let cursor_pos = ui.cursor().min;
    let rect = egui::Rect::from_min_size(cursor_pos, PAD_DIMENSIONS);

    let label = format!("Pad {}", pad.id);
    let (resp, clicked) = {
        let label: &str = &label;
        let button = egui::Button::new(label)
            .fill(egui::Color32::TRANSPARENT)
            .stroke(egui::Stroke::NONE)
            .min_size(PAD_DIMENSIONS);

        let resp = ui.add(button);
        let clicked = resp.clicked();

        (resp, clicked)
    };

    let resp = resp.on_hover_text(format!(
        "Pad {} - Note: {}\nOff: {}, On: {}\nChannel: {}",
        pad.id, pad.note, pad.off_color, pad.on_color, pad.channel
    ));

    accessibility::draw_focus_indicator(ui, rect, resp.has_focus(), 4.0);

    let half_w = rect.width() * 0.5;

    let left_rect = egui::Rect::from_min_size(rect.min, vec2(half_w, rect.height()));
    let right_rect =
        egui::Rect::from_min_size(rect.min + vec2(half_w, 0.0), vec2(half_w, rect.height()));

    let (r, g, b) = *pad.off_color.as_rgb_color();
    let off_color = Color32::from_rgb(r, g, b);
    ui.painter().rect_filled(left_rect, 0.0, off_color);

    let (r, g, b) = *pad.on_color.as_rgb_color();
    let on_color = Color32::from_rgb(r, g, b);
    ui.painter().rect_filled(right_rect, 0.0, on_color);

    ui.painter()
        .rect_filled(rect.shrink(3.0), 4.0, Color32::from_rgb(32, 32, 32));

    if let Some(UserSelection::Pad { id }) = selected_item
        && pad.id == *id
    {
        ui.painter().rect_stroke(
            rect,
            4.0,
            egui::Stroke::new(1.5, Color32::WHITE),
            egui::StrokeKind::Outside,
        );
    }

    let half_h = rect.height() * 0.5;
    let top_half = egui::Rect::from_min_size(rect.min, vec2(rect.width(), half_h));
    let bottom_half =
        egui::Rect::from_min_size(rect.min + vec2(0.0, half_h), vec2(rect.width(), half_h));

    ui.painter().text(
        top_half.center(),
        egui::Align2::CENTER_CENTER,
        pad.id.to_string(),
        egui::FontId::proportional(12.0),
        Color32::from_rgb(231, 231, 231),
    );

    ui.painter().text(
        bottom_half.center(),
        egui::Align2::CENTER_CENTER,
        format!("♩{}", pad.note),
        egui::FontId::proportional(12.0),
        Color32::from_rgb(231, 231, 231),
    );

    if clicked {
        *selected_item = Some(UserSelection::Pad { id: pad.id });
    }
}

fn render_controls(ui: &mut Ui, ui_state: &mut UiState) {
    let pads = ui_state.preset.pads;
    let dials = ui_state.preset.dials;
    let faders = ui_state.preset.faders;
    let switches = ui_state.preset.switches;

    render_all_pad_banks(ui, &mut ui_state.selected_item, pads);
    render_all_control_banks(ui, &mut ui_state.selected_item, dials, faders, switches);
}

fn render_all_control_banks(
    ui: &mut Ui,
    selected_item: &mut Option<UserSelection>,
    dial_repo: DialRepository,
    fader_repo: FaderRepository,
    switch_repo: SwitchRepository,
) {
    ui.horizontal(|ui| {
        for bank_label in CONTROL_BANKS.iter() {
            let (rect, _) =
                ui.allocate_exact_size(vec2(CONTROL_BANK_X, 20.0), egui::Sense::hover());
            ui.painter().text(
                rect.left_center() + vec2(4.0, 0.0),
                egui::Align2::LEFT_CENTER,
                format!("Control Bank {}", bank_label),
                egui::FontId::default(),
                ui.style().visuals.text_color(),
            );
        }
    });

    ui.horizontal_top(|ui| {
        for bank_idx in 0..3 {
            for dial_offset in 0..4 {
                let dial_id = bank_idx * 4 + dial_offset;
                let dial = dial_repo.0[dial_id];
                render_dial(ui, selected_item, dial, dial_id);
            }
        }
    });

    ui.horizontal_top(|ui| {
        for bank_idx in 0..3 {
            for fader_offset in 0..4 {
                let fader_id = bank_idx * 4 + fader_offset;
                let fader = fader_repo.0[fader_id];
                render_fader(ui, selected_item, fader, fader_id);
            }
        }
    });

    ui.horizontal_top(|ui| {
        for bank_idx in 0..3 {
            for switch_offset in 0..4 {
                let switch_id = bank_idx * 4 + switch_offset;
                let switch = switch_repo.0[switch_id];
                render_switch(ui, selected_item, switch, switch_id);
            }
        }
    });
}

fn render_dial(ui: &mut Ui, selected_item: &mut Option<UserSelection>, dial: Dial, dial_id: usize) {
    let bank = CONTROL_BANKS[dial_id / 4];
    let num = (dial_id % 4) + 1;
    let full_label = format!("Dial {}{}", bank, num);

    let mut button = egui::Button::new(full_label.clone())
        .min_size(DIAL_DIMENSIONS)
        .fill(Color32::DARK_GRAY)
        .corner_radius(24.0)
        .wrap();

    if let Some(UserSelection::Dial { id }) = selected_item
        && dial_id == *id
    {
        button = button.stroke(egui::Stroke::new(1.5, Color32::WHITE));
    }

    let resp = ui.add_sized(DIAL_DIMENSIONS, button);

    let cc_val: u8 = dial.midicc.into();
    let min_val: u8 = dial.min.into();
    let max_val: u8 = dial.max.into();
    let resp = resp.on_hover_text(format!(
        "Dial {} - CC: {}\nChannel: {}\nRange: {}-{}",
        full_label, cc_val, dial.channel, min_val, max_val
    ));

    accessibility::draw_focus_indicator(ui, resp.rect, resp.has_focus(), 24.0);

    if resp.clicked() {
        *selected_item = Some(UserSelection::Dial { id: dial_id });
    }
}

fn render_fader(
    ui: &mut Ui,
    selected_item: &mut Option<UserSelection>,
    fader: Fader,
    fader_id: usize,
) {
    let bank = CONTROL_BANKS[fader_id / 4];
    let num = (fader_id % 4) + 1;
    let full_label = format!("Fader {}{}", bank, num);

    let mut button: egui::Button<'_> = egui::Button::new(full_label.clone())
        .min_size(FADER_DIMENSIONS)
        .fill(Color32::DARK_GRAY)
        .corner_radius(4.0)
        .wrap();

    if let Some(UserSelection::Fader { id }) = selected_item
        && fader_id == *id
    {
        button = button.stroke(egui::Stroke::new(1.5, Color32::WHITE));
    }

    let resp = ui.add_sized(FADER_DIMENSIONS, button);

    let cc_val: u8 = fader.midicc.into();
    let min_val: u8 = fader.min.into();
    let max_val: u8 = fader.max.into();
    let resp = resp.on_hover_text(format!(
        "Fader {} - CC: {}\nChannel: {}\nRange: {}-{}",
        full_label, cc_val, fader.channel, min_val, max_val
    ));

    accessibility::draw_focus_indicator(ui, resp.rect, resp.has_focus(), 4.0);

    if resp.clicked() {
        *selected_item = Some(UserSelection::Fader { id: fader_id });
    }
}

fn render_switch(
    ui: &mut Ui,
    selected_item: &mut Option<UserSelection>,
    switch: Switch,
    switch_id: usize,
) {
    let bank = CONTROL_BANKS[switch_id / 4];
    let num = (switch_id % 4) + 1;
    let full_label = format!("Switch {}{}", bank, num);

    let mut button = egui::Button::new(full_label.clone())
        .min_size(SWITCH_DIMENSIONS)
        .fill(Color32::DARK_GRAY)
        .corner_radius(4.0)
        .wrap();

    if let Some(UserSelection::Switch { id }) = selected_item
        && switch_id == *id
    {
        button = button.stroke(egui::Stroke::new(1.5, Color32::WHITE));
    }

    let resp = ui.add_sized(SWITCH_DIMENSIONS, button);

    let cc_val: u8 = switch.midicc.into();
    let resp = resp.on_hover_text(format!(
        "Switch {} - Mode: {}\nChannel: {}\nCC: {}",
        full_label, switch.mode, switch.channel, cc_val
    ));

    accessibility::draw_focus_indicator(ui, resp.rect, resp.has_focus(), 4.0);

    if resp.clicked() {
        *selected_item = Some(UserSelection::Switch { id: switch_id });
    }
}

fn render_modal_editor(ctx: &Context, ui_state: &mut UiState) {
    let Some(selected_item) = ui_state.selected_item else {
        return;
    };

    let modal_response = Modal::new(egui::Id::new("control_editor_modal")).show(ctx, |ui| {
        ui.set_min_width(400.0);

        ui.heading(selected_item.to_string());
        ui.separator();

        match selected_item {
            UserSelection::Pad { id: index } => {
                if let Some(pad) = ui_state.preset.pads.pads.iter_mut().find(|p| p.id == index) {
                    render_pad_editor(ui, pad);
                }
            }
            UserSelection::Dial { id: index } => {
                if let Some(dial) = ui_state.preset.dials.0.get_mut(index) {
                    render_dial_editor(ui, dial);
                }
            }
            UserSelection::Fader { id: index } => {
                if let Some(fader) = ui_state.preset.faders.0.get_mut(index) {
                    render_fader_editor(ui, fader);
                }
            }
            UserSelection::Switch { id: index } => {
                if let Some(switch) = ui_state.preset.switches.0.get_mut(index) {
                    render_switch_editor(ui, switch);
                }
            }
            UserSelection::PresetSettings => {
                render_preset_settings(ui, &mut ui_state.preset.settings);
            }
            UserSelection::GlobalSettings => render_global_settings(ui, ui_state),
            UserSelection::PadNoteMapping => render_note_mapping(ui, ui_state),
            UserSelection::PadOffColorMapping => render_off_color_mapping(ui, ui_state),
            UserSelection::PadOnColorMapping => render_on_color_mapping(ui, ui_state),
        };

        ui.separator();

        // todo: I want this button to have an ambidextrous design.
        ui.button("Close")
    });

    if modal_response.should_close() || modal_response.inner.clicked() {
        ui_state.selected_item = None;
    }
}

fn render_pad_editor(ui: &mut Ui, pad: &mut Pad) {
    ui.with_layout(Layout::left_to_right(Align::Min), |ui| {
        Grid::new("pad_compare_grid_l")
            .striped(true)
            .show(ui, |ui| {
                row_edit_enum(ui, "kind", &mut pad.kind);
                row_edit_enum(ui, "channel", &mut pad.channel);
                row_edit_note(ui, "note", &mut pad.note);
                row_edit_enum(ui, "midi to din", &mut pad.midi2din);
                row_edit_enum(ui, "trigger", &mut pad.trigger);
                row_edit_enum(ui, "aftertouch", &mut pad.aftertouch);
                row_edit_midi_value(ui, "program", &mut pad.program);
                row_edit_midi_value(ui, "msb", &mut pad.msb);
                row_edit_midi_value(ui, "lsb", &mut pad.lsb);
                row_edit_enum(ui, "off color", &mut pad.off_color);
                row_edit_enum(ui, "on color", &mut pad.on_color);
            });
    });
}

fn render_dial_editor(ui: &mut Ui, dial: &mut Dial) {
    ui.with_layout(Layout::left_to_right(Align::Min), |ui| {
        Grid::new("dial_compare_grid_l")
            .striped(true)
            .show(ui, |ui| {
                row_edit_enum(ui, "kind", &mut dial.kind);
                row_edit_enum(ui, "channel", &mut dial.channel);
                row_edit_u8(ui, "midicc", &mut dial.midicc.into());
                row_edit_u8(ui, "min", &mut dial.min.into());
                row_edit_u8(ui, "max", &mut dial.max.into());
                row_edit_enum(ui, "midi to din", &mut dial.midi2din);
                row_edit_u8(ui, "msb", &mut dial.msb.into());
                row_edit_u8(ui, "lsb", &mut dial.lsb.into());
                row_edit_u8(ui, "value", &mut dial.value.into());
            });
    });
}

fn render_fader_editor(ui: &mut Ui, fader: &mut Fader) {
    ui.with_layout(Layout::left_to_right(Align::Min), |ui| {
        Grid::new("fader_compare_grid_l")
            .striped(true)
            .show(ui, |ui| {
                row_edit_enum(ui, "kind", &mut fader.kind);
                row_edit_enum(ui, "channel", &mut fader.channel);
                row_edit_u8(ui, "midicc", &mut fader.midicc.into());
                row_edit_u8(ui, "min", &mut fader.min.into());
                row_edit_u8(ui, "max", &mut fader.max.into());
                row_edit_enum(ui, "midi to din", &mut fader.midi2din);
            });
    });
}

fn render_switch_editor(ui: &mut Ui, switch: &mut Switch) {
    ui.with_layout(Layout::left_to_right(Align::Min), |ui| {
        Grid::new("switch_compare_grid_l")
            .striped(true)
            .show(ui, |ui| {
                row_edit_enum(ui, "kind", &mut switch.kind);
                row_edit_enum(ui, "channel", &mut switch.channel);
                row_edit_u8(ui, "midicc", &mut switch.midicc.into());
                row_edit_enum(ui, "mode", &mut switch.mode);
                row_edit_u8(ui, "prog", &mut switch.prog.into());
                row_edit_u8(ui, "msb", &mut switch.msb.into());
                row_edit_u8(ui, "lsb", &mut switch.lsb.into());
                row_edit_enum(ui, "midi to din", &mut switch.midi2din);
                row_edit_u8(ui, "note", &mut switch.note);
                row_edit_u8(ui, "velo", &mut switch.velo.into());
                row_edit_enum(ui, "invert", &mut switch.invert);
            });
    });
}

fn enum_combo_box<T>(ui: &mut Ui, id: impl Into<String>, value: &mut T, width: Option<f32>)
where
    T: IntoEnumIterator + std::fmt::Display + Clone + Copy + PartialEq,
{
    let mut combo = ComboBox::from_id_salt(id.into()).selected_text(format!("{}", value));

    if let Some(w) = width {
        combo = combo.width(w);
    }

    combo.show_ui(ui, |ui| {
        for variant in T::iter() {
            ui.selectable_value(value, variant, format!("{}", variant));
        }
    });
}

fn row_edit_enum<T>(ui: &mut Ui, name: &str, value: &mut T)
where
    T: IntoEnumIterator + std::fmt::Display + Clone + Copy + PartialEq,
{
    ui.label(name);

    ComboBox::from_id_salt(name)
        .selected_text(format!("{}", value))
        .show_ui(ui, |ui| {
            for variant in T::iter() {
                ui.selectable_value(value, variant, format!("{}", variant));
            }
        });
    ui.end_row();
}

fn row_edit_u8(ui: &mut Ui, name: &str, value: &mut u8) {
    ui.label(name);

    ui.add(DragValue::new(value).range(0..=127));

    ui.end_row();
}

fn row_edit_midi_value(ui: &mut Ui, name: &str, value: &mut MidiValue) {
    ui.label(name);
    let val: u8 = (*value).into();
    let mut val = val;

    if ui.add(DragValue::new(&mut val).range(0..=127)).changed() {
        *value = MidiValue::from(val);
    }
    ui.end_row();
}

fn row_edit_note(ui: &mut Ui, name: &str, value: &mut Note) {
    ui.label(name);
    let mut val = *value as u32;
    if ui.add(DragValue::new(&mut val).range(0..=127)).changed()
        && let Ok(note) = Note::try_from(val as u8)
    {
        *value = note;
    }
    ui.end_row();
}

fn row_edit_u8_clamped(ui: &mut Ui, name: &str, value: &mut u8, range: RangeInclusive<u8>) {
    ui.label(name);
    ui.add(DragValue::new(value).range(range));
    ui.end_row();
}

fn row_edit_u16_clamped(ui: &mut Ui, name: &str, value: &mut u16, range: RangeInclusive<u16>) {
    ui.label(name);
    ui.add(DragValue::new(value).range(range));
    ui.end_row();
}

fn row_edit_preset_name(ui: &mut Ui, name: &str, value: &mut PresetName) {
    ui.label(name);

    let mut text = value
        .0
        .iter()
        .map(|&b| if b.is_ascii() { b as char } else { ' ' })
        .collect::<String>()
        .trim_end()
        .to_string();

    ui.add(
        TextEdit::singleline(&mut text)
            .char_limit(8)
            .desired_width(80.0),
    );

    let mut buf = [b' '; 8];
    for (i, b) in text.bytes().filter(|b| b.is_ascii()).take(8).enumerate() {
        buf[i] = b;
    }
    *value = PresetName(buf);

    ui.end_row();
}
