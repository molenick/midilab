use std::ops::RangeInclusive;
use std::path::PathBuf;

use eframe::egui;
use eframe::egui::Align;
use eframe::egui::CentralPanel;
use eframe::egui::CollapsingHeader;
use eframe::egui::Color32;
use eframe::egui::ComboBox;
use eframe::egui::Context;
use eframe::egui::DragValue;
use eframe::egui::Grid;
use eframe::egui::Layout;
use eframe::egui::ScrollArea;
use eframe::egui::TextEdit;
use eframe::egui::TopBottomPanel;
use eframe::egui::Ui;
use eframe::egui::Vec2;
use eframe::egui::vec2;
use midilab::IntoEnumIterator;
use midilab::manufacturer::akai::mpd226::ColorPattern;
use midilab::manufacturer::akai::mpd226::ColorSequence;
use midilab::manufacturer::akai::mpd226::Global;
use midilab::manufacturer::akai::mpd226::NotePattern;
use midilab::manufacturer::akai::mpd226::Preset;
use midilab::manufacturer::akai::mpd226::control::Dial;
use midilab::manufacturer::akai::mpd226::control::Fader;
use midilab::manufacturer::akai::mpd226::control::Pad;
use midilab::manufacturer::akai::mpd226::control::Switch;
use midilab::manufacturer::akai::mpd226::control::value_kind::ActiveState;
use midilab::manufacturer::akai::mpd226::control::value_kind::AfterTouchKind;
use midilab::manufacturer::akai::mpd226::control::value_kind::DialKind;
use midilab::manufacturer::akai::mpd226::control::value_kind::FaderKind;
use midilab::manufacturer::akai::mpd226::control::value_kind::GateValue;
use midilab::manufacturer::akai::mpd226::control::value_kind::MidiChannel;
use midilab::manufacturer::akai::mpd226::control::value_kind::MidiClock;
use midilab::manufacturer::akai::mpd226::control::value_kind::NoteDisplay;
use midilab::manufacturer::akai::mpd226::control::value_kind::PadColor;
use midilab::manufacturer::akai::mpd226::control::value_kind::PadCurve;
use midilab::manufacturer::akai::mpd226::control::value_kind::PadKind;
use midilab::manufacturer::akai::mpd226::control::value_kind::PresetName;
use midilab::manufacturer::akai::mpd226::control::value_kind::PresetSlot;
use midilab::manufacturer::akai::mpd226::control::value_kind::SwingKind;
use midilab::manufacturer::akai::mpd226::control::value_kind::SwitchKind;
use midilab::manufacturer::akai::mpd226::control::value_kind::TapAverage;
use midilab::manufacturer::akai::mpd226::control::value_kind::Tempo;
use midilab::manufacturer::akai::mpd226::control::value_kind::TimeDivision;
use midilab::manufacturer::akai::mpd226::control::value_kind::TransportKind;
use midilab::manufacturer::akai::mpd226::control::value_kind::TriggerKind;
use midilab::manufacturer::akai::mpd226::control::value_kind::UsbChannel;
use midilab::manufacturer::akai::mpd226::repository::DialRepository;
use midilab::manufacturer::akai::mpd226::repository::FaderRepository;
use midilab::manufacturer::akai::mpd226::repository::PadRepository;
use midilab::manufacturer::akai::mpd226::repository::SwitchRepository;
use midilab::message::AppMsg;
use midilab::message::PendingFileAction;
use midilab::message::UiEffect;
use midilab::message::UiMsg;
use midilab::message::UserMsg;
use midilab::midi::Note;
use midilab::scale::Octave;
use midilab::scale::PitchClass;
use midilab::scale::ScaleKind;
use midilab::scale::ScaleSequence;
use midilab::scale::SequenceDirection;
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::sync::mpsc::UnboundedSender;

#[allow(unused)]
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

    fn poll_ui_msgs(&mut self) {
        while let Ok(msg) = self.ui_rx.try_recv() {
            match msg {
                UiMsg::UpdatePreset(preset) => {
                    self.ui_state.preset = *preset;
                }

                UiMsg::UserMsg(e) => {
                    self.ui_state.user_error = Some(e);
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
}

impl eframe::App for AkaiMpd226Editor {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        self.poll_ui_msgs();

        TopBottomPanel::top("selected_item_panel")
            .exact_height(180.0)
            .show(ctx, |ui| {
                selection_compare_table(ui, &mut self.ui_state);
            });

        CentralPanel::default().show(ctx, |ui| {
            ScrollArea::vertical().show(ui, |ui| {
                render_preset_settings(ui, &mut self.ui_state, &mut self.outbox);
                ui.add_space(16.0);
                render_global_settings(ui, &mut self.ui_state, &mut self.outbox);
                ui.add_space(16.0);
                render_controls(ui, &mut self.ui_state);
                ui.add_space(16.0);
                render_pad_patterns(ui, &mut self.ui_state);
            });
        });

        for msg in self.outbox.drain(..) {
            let _ = self.app_tx.send(msg);
        }

        ctx.request_repaint_after(std::time::Duration::from_millis(17));
    }
}

pub fn render_controls(ui: &mut Ui, ui_state: &mut UiState) {
    CollapsingHeader::new("Controls")
        .default_open(true)
        .show(ui, |ui| {
            render_banks(ui, ui_state);
        });
}

const APP_X: f32 = 1290. * 0.35;
const APP_Y: f32 = 2796. * 0.35;
pub const APP_DIMENSIONS: Vec2 = Vec2 { x: APP_X, y: APP_Y };

const PAD_X: f32 = 64.;
const PAD_Y: f32 = 64.;
const PAD_DIMENSIONS: Vec2 = Vec2 { x: PAD_X, y: PAD_Y };

const DIAL_X: f32 = 48.;
const DIAL_Y: f32 = 48.;
const DIAL_DIMENSIONS: Vec2 = Vec2 {
    x: DIAL_X,
    y: DIAL_Y,
};

const FADER_X: f32 = 48.;
const FADER_Y: f32 = 80.;
const FADER_DIMENSIONS: Vec2 = Vec2 {
    x: FADER_X,
    y: FADER_Y,
};

const SWITCH_X: f32 = 48.;
const SWITCH_Y: f32 = 24.;
const SWITCH_DIMENSIONS: Vec2 = Vec2 {
    x: SWITCH_X,
    y: SWITCH_Y,
};

const BANKS: [&str; 4] = ["A", "B", "C", "D"];
const CONTROL_BANKS: [&str; 3] = ["A", "B", "C"];

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum UserSelection {
    Pad { id: usize },
    Dial { id: usize },
    Fader { id: usize },
    Switch { id: usize },
}

#[derive(Default)]
pub struct UiState {
    pub selected_item: Option<UserSelection>,
    pub preset: Preset,
    pub global: Global,
    pub note_mapping: NoteMappingState,
    pub off_color_mapping: ColorMappingState,
    pub on_color_mapping: ColorMappingState,
    pub user_error: Option<UserMsg>,
    pub pending_action: Option<PendingFileAction>,
    pub configured_directory: Option<PathBuf>,
}

pub struct NoteMappingState {
    pub scale_seq: ScaleSequence,
    pub starting_from_pad: usize,
    pub tonic_highlighting_enabled: bool,
    pub tonic_color: PadColor,
}
impl Default for NoteMappingState {
    fn default() -> Self {
        Self {
            scale_seq: ScaleSequence {
                tonic: PitchClass::C,
                scale: ScaleKind::Chromatic,
                direction: SequenceDirection::Ascending,
                octave: Octave::O4,
                length: 64,
            },
            starting_from_pad: 0,
            tonic_highlighting_enabled: true,
            tonic_color: PadColor::Red,
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

fn render_preset_settings(ui: &mut Ui, ui_state: &mut UiState, outbox: &mut Vec<AppMsg>) {
    CollapsingHeader::new("Preset Settings")
        .default_open(true)
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                if ui.button("Load from device").clicked() {
                    ui_state.user_error = None;
                    outbox.push(AppMsg::Ui(UiEffect::DumpPreset(
                        ui_state.preset.settings.preset_slot,
                    )));
                }

                if ui.button("Send to device").clicked() {
                    ui_state.user_error = None;
                    outbox.push(AppMsg::Ui(UiEffect::WritePreset(Box::new(ui_state.preset))));
                }

                if let Some(status) = &ui_state.user_error {
                    let color = match status.kind {
                        midilab::message::UserMsgKind::Status => Color32::GREEN,
                        midilab::message::UserMsgKind::Error => Color32::RED,
                    };

                    ui.colored_label(color, &status.msg);
                }
            });

            Grid::new("preset_settings_grid")
                .striped(true)
                .spacing([16.0, 6.0])
                .show(ui, |ui| {
                    row_edit_preset_slot(
                        ui,
                        "Preset Slot",
                        &mut ui_state.preset.settings.preset_slot,
                    );
                    row_edit_preset_name(
                        ui,
                        "Preset Name",
                        &mut ui_state.preset.settings.preset_name,
                    );
                    row_edit_tempo(ui, "Tempo", &mut ui_state.preset.settings.tempo);
                    row_edit_time_division(
                        ui,
                        "Division",
                        &mut ui_state.preset.settings.time_division,
                    );
                    row_edit_trigger_kind(
                        ui,
                        "Div Switch",
                        &mut ui_state.preset.settings.time_division_switch,
                    );
                    row_edit_trigger_kind(
                        ui,
                        "Note Repeat",
                        &mut ui_state.preset.settings.note_repeat_switch,
                    );
                    row_edit_gate(ui, "Gate", &mut ui_state.preset.settings.gate);
                    row_edit_swing_kind(ui, "Swing", &mut ui_state.preset.settings.swing);
                    row_edit_transport_kind(
                        ui,
                        "Transport",
                        &mut ui_state.preset.settings.transport,
                    );
                });
        });
}

fn render_global_settings(ui: &mut Ui, ui_state: &mut UiState, outbox: &mut Vec<AppMsg>) {
    CollapsingHeader::new("Global Settings")
        .default_open(true)
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                if ui.button("Load from device").clicked() {
                    ui_state.user_error = None;
                    outbox.push(AppMsg::Ui(UiEffect::RequestGlobalFromDevice));
                }
                if ui.button("Send to device").clicked() {
                    ui_state.user_error = None;
                    outbox.push(AppMsg::Ui(UiEffect::SendGlobalToDevice(Box::new(
                        ui_state.global,
                    ))));
                }
            });

            Grid::new("global_settings_grid")
                .striped(true)
                .spacing([16.0, 6.0])
                .show(ui, |ui| {
                    row_edit_usb_channel(ui, "Common Channel", &mut ui_state.global.common_channel);
                    row_edit_u8_clamped(
                        ui,
                        "LCD Contrast",
                        &mut ui_state.global.lcd_contrast,
                        0..=100,
                    );
                    row_edit_tap_average(ui, "Tap Average", &mut ui_state.global.tap_average);
                    row_edit_active_state(ui, "Tempo LED", &mut ui_state.global.tempo_led);
                    row_edit_note_display(ui, "Note Display", &mut ui_state.global.note_display);
                    row_edit_u8_clamped(
                        ui,
                        "Pad Threshold (todo: off by -1 from device ui)",
                        &mut ui_state.global.pad_threshold,
                        0..=9,
                    );
                    row_edit_pad_curve(ui, "Pad Curve", &mut ui_state.global.pad_curve);
                    row_edit_u8_clamped(ui, "Pad Gain", &mut ui_state.global.pad_gain, 0..=20);
                    row_edit_midi_clock(ui, "MIDI Clock", &mut ui_state.global.midi_clock);
                });
        });
}

fn render_pad_patterns(ui: &mut Ui, ui_state: &mut UiState) {
    CollapsingHeader::new("Pattern Mapping")
        .default_open(true)
        .show(ui, |ui| {
            render_note_mapping(ui, ui_state);
            render_off_color_mapping(ui, ui_state);
            render_on_color_mapping(ui, ui_state);
        });
}

fn render_note_mapping(ui: &mut Ui, ui_state: &mut UiState) {
    CollapsingHeader::new("Note Mapping")
        .default_open(true)
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label("Tonic");
                ComboBox::from_id_salt("note_mapping_tonic")
                    .selected_text(ui_state.note_mapping.scale_seq.tonic.to_string())
                    .show_ui(ui, |ui| {
                        for p in PitchClass::iter() {
                            if ui
                                .selectable_value(
                                    &mut ui_state.note_mapping.scale_seq.tonic,
                                    p,
                                    p.to_string(),
                                )
                                .clicked()
                            {
                                ui_state.note_mapping.scale_seq.tonic = p;
                            }
                        }
                    });
            });

            ui.horizontal(|ui| {
                ui.label("Scale");
                ComboBox::from_id_salt("note_mapping_scale")
                    .selected_text(ui_state.note_mapping.scale_seq.scale.to_string())
                    .show_ui(ui, |ui| {
                        for s in ScaleKind::iter() {
                            if ui
                                .selectable_value(
                                    &mut ui_state.note_mapping.scale_seq.scale,
                                    s,
                                    s.to_string(),
                                )
                                .clicked()
                            {
                                ui_state.note_mapping.scale_seq.scale = s;
                            }
                        }
                    });
            });

            ui.horizontal(|ui| {
                ui.label("Octave");
                ComboBox::from_id_salt("note_mapping_octave")
                    .selected_text(ui_state.note_mapping.scale_seq.octave.to_string())
                    .show_ui(ui, |ui| {
                        for s in Octave::iter() {
                            if ui
                                .selectable_value(
                                    &mut ui_state.note_mapping.scale_seq.octave,
                                    s,
                                    s.to_string(),
                                )
                                .clicked()
                            {
                                ui_state.note_mapping.scale_seq.octave = s;
                            }
                        }
                    });
            });

            ui.horizontal(|ui| {
                ui.label("Direction");
                ComboBox::from_id_salt("note_mapping_direction")
                    .selected_text(ui_state.note_mapping.scale_seq.direction.to_string())
                    .show_ui(ui, |ui| {
                        for s in SequenceDirection::iter() {
                            if ui
                                .selectable_value(
                                    &mut ui_state.note_mapping.scale_seq.direction,
                                    s,
                                    s.to_string(),
                                )
                                .clicked()
                            {
                                ui_state.note_mapping.scale_seq.direction = s;
                            }
                        }
                    });
            });

            ui.horizontal(|ui| {
                ui.label("Starting from Pad");

                ui.add(DragValue::new(&mut ui_state.note_mapping.starting_from_pad).range(0..=63))
            });

            ui.horizontal(|ui| {
                ui.label("Length");

                ui.add(DragValue::new(&mut ui_state.note_mapping.scale_seq.length).range(1..=64))
            });

            ui.add_space(8.0);

            ui.checkbox(
                &mut ui_state.note_mapping.tonic_highlighting_enabled,
                "Tonic highlighting",
            );
            if ui_state.note_mapping.tonic_highlighting_enabled {
                ui.horizontal(|ui| {
                    ui.label("Tonic color");
                    ComboBox::from_id_salt("tonic_highlight_color")
                        .selected_text(ui_state.note_mapping.tonic_color.to_string())
                        .show_ui(ui, |ui| {
                            for c in PadColor::iter() {
                                if ui
                                    .selectable_value(
                                        &mut ui_state.note_mapping.tonic_color,
                                        c,
                                        c.to_string(),
                                    )
                                    .clicked()
                                {
                                    ui_state.note_mapping.tonic_color = c;
                                }
                            }
                        });
                });
            }

            ui.add_space(8.0);

            let resp = ui.button("Set pattern");
            if resp.clicked() {
                let scale_seq = ui_state.note_mapping.scale_seq;
                ui_state.preset.pads.set_note_pattern(
                    ui_state.note_mapping.starting_from_pad,
                    NotePattern::Scale(scale_seq),
                );

                if ui_state.note_mapping.tonic_highlighting_enabled {
                    let tonic_color = (scale_seq.tonic, ui_state.note_mapping.tonic_color);
                    ui_state.preset.pads.highlight_tonics(
                        ui_state.note_mapping.starting_from_pad,
                        scale_seq.length,
                        tonic_color,
                    );
                }
            }
        });
}

fn render_off_color_mapping(ui: &mut Ui, ui_state: &mut UiState) {
    CollapsingHeader::new("Off Color Mapping")
        .default_open(true)
        .show(ui, |ui| {
            render_color_pattern_editor(ui, "off", &mut ui_state.off_color_mapping.pattern);

            ui.add_space(4.0);

            ui.horizontal(|ui| {
                ui.label("Start Pad");
                ui.add(
                    DragValue::new(&mut ui_state.off_color_mapping.starting_from_pad).range(0..=63),
                );
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
        });
}

fn render_on_color_mapping(ui: &mut Ui, ui_state: &mut UiState) {
    CollapsingHeader::new("On Color Mapping")
        .default_open(true)
        .show(ui, |ui| {
            ui.add_space(4.0);

            render_color_pattern_editor(ui, "on", &mut ui_state.on_color_mapping.pattern);

            ui.add_space(4.0);

            ui.horizontal(|ui| {
                ui.label("Start Pad");
                ui.add(
                    DragValue::new(&mut ui_state.on_color_mapping.starting_from_pad).range(0..=63),
                );
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
        });
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
                    ComboBox::from_id_salt(format!("{}_{}_color", id_prefix, idx))
                        .width(80.0)
                        .selected_text(seq.color.to_string())
                        .show_ui(ui, |ui| {
                            for c in PadColor::iter() {
                                ui.selectable_value(&mut seq.color, c, c.to_string());
                            }
                        });

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
    pad_repo: &mut PadRepository,
) {
    let banks: Vec<Vec<Pad>> = pad_repo
        .pads
        .chunks(16)
        .map(|chunk| chunk.to_vec())
        .collect();

    CollapsingHeader::new("Pad Banks")
        .default_open(true)
        .show(ui, |ui| {
            for (bank_id, bank) in banks.into_iter().enumerate() {
                let bank_label = BANKS[bank_id].to_string();
                render_pad_bank(ui, selected_item, bank, bank_label);
            }
        });
}

fn render_pad_bank(
    ui: &mut Ui,
    selected_item: &mut Option<UserSelection>,
    pads: Vec<Pad>,
    label: String,
) {
    let reordered: Vec<Pad> = pads.chunks(4).rev().flatten().cloned().collect();

    CollapsingHeader::new(format!("Pad Bank {}", label))
        .default_open(true)
        .show(ui, |ui| {
            Grid::new(format!("pad_bank_{}", label))
                .num_columns(4)
                .spacing([8.0, 8.0])
                .show(ui, |ui| {
                    for (i, pad) in reordered.into_iter().enumerate() {
                        render_pad(ui, selected_item, pad);
                        if (i + 1) % 4 == 0 {
                            ui.end_row();
                        }
                    }
                });
        });
}

fn render_pad(ui: &mut Ui, selected_item: &mut Option<UserSelection>, pad: Pad) {
    let (rect, resp) = ui.allocate_exact_size(PAD_DIMENSIONS, egui::Sense::click());

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
        .rect_filled(rect.shrink(3.0), 4.0, Color32::from_rgb(64, 64, 64));

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

    if resp.clicked() {
        click_pad(pad.id, selected_item);
    }
}

fn render_banks(ui: &mut Ui, ui_state: &mut UiState) {
    render_all_pad_banks(ui, &mut ui_state.selected_item, &mut ui_state.preset.pads);
    render_all_control_banks(
        ui,
        &mut ui_state.selected_item,
        &mut ui_state.preset.dials,
        &mut ui_state.preset.faders,
        &mut ui_state.preset.switches,
    );
}

fn click_pad(id: usize, selected_item: &mut Option<UserSelection>) {
    if *selected_item == Some(UserSelection::Pad { id }) {
        *selected_item = None;
    } else {
        *selected_item = Some(UserSelection::Pad { id });
    }
}

fn render_all_control_banks(
    ui: &mut Ui,
    selected_item: &mut Option<UserSelection>,
    dial_repo: &mut DialRepository,
    fader_repo: &mut FaderRepository,
    switch_repo: &mut SwitchRepository,
) {
    ui.vertical(|ui| {
        for (bank_idx, bank_label) in CONTROL_BANKS.iter().enumerate() {
            render_control_bank(
                ui,
                selected_item,
                dial_repo,
                fader_repo,
                switch_repo,
                bank_idx,
                bank_label,
            );
            ui.add_space(16.0);
        }
    });
}

fn render_control_bank(
    ui: &mut Ui,
    selected_item: &mut Option<UserSelection>,
    dial_repo: &mut DialRepository,
    fader_repo: &mut FaderRepository,
    switch_repo: &mut SwitchRepository,
    bank_idx: usize,
    bank_label: &str,
) {
    CollapsingHeader::new(format!("Control Bank {}", bank_label))
        .default_open(true)
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                for dial_offset in 0..4 {
                    let dial_id = bank_idx * 4 + dial_offset;
                    let dial = dial_repo.0[dial_id];
                    render_dial(ui, selected_item, dial, dial_id);
                    ui.add_space(4.0);
                }
            });

            ui.add_space(8.0);

            ui.horizontal(|ui| {
                for fader_offset in 0..4 {
                    let fader_id = bank_idx * 4 + fader_offset;
                    let fader = fader_repo.0[fader_id];
                    render_fader(ui, selected_item, fader, fader_id);
                    ui.add_space(4.0);
                }
            });

            ui.add_space(8.0);

            ui.horizontal(|ui| {
                for switch_offset in 0..4 {
                    let switch_id = bank_idx * 4 + switch_offset;
                    let switch = switch_repo.0[switch_id];
                    render_switch(ui, selected_item, switch, switch_id);
                    ui.add_space(4.0);
                }
            });
        });
}

fn render_dial(
    ui: &mut Ui,
    selected_item: &mut Option<UserSelection>,
    _dial: Dial,
    dial_id: usize,
) {
    let (rect, resp) = ui.allocate_exact_size(DIAL_DIMENSIONS, egui::Sense::click());

    ui.painter().rect_filled(rect, 24.0, Color32::DARK_GRAY);

    if let Some(UserSelection::Dial { id }) = selected_item
        && dial_id == *id
    {
        ui.painter().rect_stroke(
            rect,
            24.0,
            egui::Stroke::new(1.5, Color32::WHITE),
            egui::StrokeKind::Outside,
        );
    }

    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        dial_id.to_string(),
        egui::FontId::proportional(12.0),
        Color32::WHITE,
    );

    if resp.clicked() {
        click_dial(dial_id, selected_item);
    }
}

fn click_dial(id: usize, selected_item: &mut Option<UserSelection>) {
    if *selected_item == Some(UserSelection::Dial { id }) {
        *selected_item = None;
    } else {
        *selected_item = Some(UserSelection::Dial { id });
    }
}

fn render_fader(
    ui: &mut Ui,
    selected_item: &mut Option<UserSelection>,
    _fader: Fader,
    fader_id: usize,
) {
    let (rect, resp) = ui.allocate_exact_size(FADER_DIMENSIONS, egui::Sense::click());

    ui.painter().rect_filled(rect, 4.0, Color32::DARK_GRAY);

    if let Some(UserSelection::Fader { id }) = selected_item
        && fader_id == *id
    {
        ui.painter().rect_stroke(
            rect,
            4.0,
            egui::Stroke::new(1.5, Color32::WHITE),
            egui::StrokeKind::Outside,
        );
    }

    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        fader_id.to_string(),
        egui::FontId::proportional(10.0),
        Color32::WHITE,
    );

    if resp.clicked() {
        click_fader(fader_id, selected_item);
    }
}

fn click_fader(id: usize, selected_item: &mut Option<UserSelection>) {
    if *selected_item == Some(UserSelection::Fader { id }) {
        *selected_item = None;
    } else {
        *selected_item = Some(UserSelection::Fader { id });
    }
}

fn render_switch(
    ui: &mut Ui,
    selected_item: &mut Option<UserSelection>,
    _switch: Switch,
    switch_id: usize,
) {
    let (rect, resp) = ui.allocate_exact_size(SWITCH_DIMENSIONS, egui::Sense::click());

    ui.painter().rect_filled(rect, 4.0, Color32::DARK_GRAY);

    if let Some(UserSelection::Switch { id }) = selected_item
        && switch_id == *id
    {
        ui.painter().rect_stroke(
            rect,
            4.0,
            egui::Stroke::new(1.5, Color32::WHITE),
            egui::StrokeKind::Outside,
        );
    }

    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        switch_id.to_string(),
        egui::FontId::proportional(10.0),
        Color32::WHITE,
    );

    if resp.clicked() {
        click_switch(switch_id, selected_item);
    }
}

fn click_switch(id: usize, selected_item: &mut Option<UserSelection>) {
    if *selected_item == Some(UserSelection::Switch { id }) {
        *selected_item = None;
    } else {
        *selected_item = Some(UserSelection::Switch { id });
    }
}

fn selection_compare_table(ui: &mut Ui, ui_state: &mut UiState) {
    ui.vertical(|ui| {
        let selection_label = match ui_state.selected_item {
            Some(UserSelection::Pad { id }) => format!("Pad {}", id),
            Some(UserSelection::Dial { id }) => {
                let bank = CONTROL_BANKS[id / 4];
                let num = (id % 4) + 1;
                format!("Dial {}{}", bank, num)
            }
            Some(UserSelection::Fader { id }) => {
                let bank = CONTROL_BANKS[id / 4];
                let num = (id % 4) + 1;
                format!("Fader {}{}", bank, num)
            }
            Some(UserSelection::Switch { id }) => {
                let bank = CONTROL_BANKS[id / 4];
                let num = (id % 4) + 1;
                format!("Switch {}{}", bank, num)
            }
            None => "None".to_string(),
        };
        ui.label(format!("Selected: {}", selection_label));

        match ui_state.selected_item {
            Some(UserSelection::Pad { id: index }) => {
                if let Some(pad) = ui_state.preset.pads.pads.iter_mut().find(|p| p.id == index) {
                    render_pad_compare_grid(ui, pad);
                }
            }
            Some(UserSelection::Dial { id: index }) => {
                if let Some(dial) = ui_state.preset.dials.0.get_mut(index) {
                    render_dial_compare_grid(ui, dial);
                }
            }
            Some(UserSelection::Fader { id: index }) => {
                if let Some(fader) = ui_state.preset.faders.0.get_mut(index) {
                    render_fader_compare_grid(ui, fader);
                }
            }
            Some(UserSelection::Switch { id: index }) => {
                if let Some(switch) = ui_state.preset.switches.0.get_mut(index) {
                    render_switch_compare_grid(ui, switch);
                }
            }
            None => {}
        }
    });
}

fn render_pad_compare_grid(ui: &mut Ui, pad: &mut Pad) {
    ui.with_layout(Layout::left_to_right(Align::Min), |ui| {
        Grid::new("pad_compare_grid_l")
            .striped(true)
            .spacing([16.0, 6.0])
            .show(ui, |ui| {
                row_edit_pad_kind(ui, "kind", &mut pad.kind);
                row_edit_midi_channel(ui, "channel", &mut pad.channel);
                row_edit_note(ui, "note", &mut pad.note);
                row_edit_midi2din(ui, "midi to din", &mut pad.midi2din);
                row_edit_trigger_kind(ui, "trigger", &mut pad.trigger);
                row_edit_aftertouch_kind(ui, "aftertouch", &mut pad.aftertouch);
            });

        ui.add_space(32.0);

        Grid::new("pad_compare_grid_r")
            .striped(true)
            .spacing([16.0, 6.0])
            .show(ui, |ui| {
                row_edit_u8(ui, "program", &mut pad.program);
                row_edit_u8(ui, "msb", &mut pad.msb);
                row_edit_u8(ui, "lsb", &mut pad.lsb);
                row_edit_pad_color(ui, "off color", &mut pad.off_color);
                row_edit_pad_color(ui, "on color", &mut pad.on_color);
            });
    });
}

fn render_dial_compare_grid(ui: &mut Ui, dial: &mut Dial) {
    ui.with_layout(Layout::left_to_right(Align::Min), |ui| {
        Grid::new("dial_compare_grid_l")
            .striped(true)
            .spacing([16.0, 6.0])
            .show(ui, |ui| {
                row_edit_dial_kind(ui, "kind", &mut dial.kind);
                row_edit_midi_channel(ui, "channel", &mut dial.channel);
                row_edit_u8(ui, "midicc", &mut dial.midicc);
                row_edit_u8(ui, "min", &mut dial.min);
                row_edit_u8(ui, "max", &mut dial.max);
            });

        ui.add_space(32.0);

        Grid::new("dial_compare_grid_r")
            .striped(true)
            .spacing([16.0, 6.0])
            .show(ui, |ui| {
                row_edit_midi2din(ui, "midi to din", &mut dial.midi2din);
                row_edit_u8(ui, "msb", &mut dial.msb);
                row_edit_u8(ui, "lsb", &mut dial.lsb);
                row_edit_u8(ui, "value", &mut dial.value);
            });
    });
}

fn render_fader_compare_grid(ui: &mut Ui, fader: &mut Fader) {
    ui.with_layout(Layout::left_to_right(Align::Min), |ui| {
        Grid::new("fader_compare_grid_l")
            .striped(true)
            .spacing([16.0, 6.0])
            .show(ui, |ui| {
                row_edit_fader_kind(ui, "kind", &mut fader.kind);
                row_edit_midi_channel(ui, "channel", &mut fader.channel);
                row_edit_u8(ui, "midicc", &mut fader.midicc);
            });

        ui.add_space(32.0);

        Grid::new("fader_compare_grid_r")
            .striped(true)
            .spacing([16.0, 6.0])
            .show(ui, |ui| {
                row_edit_u8(ui, "min", &mut fader.min);
                row_edit_u8(ui, "max", &mut fader.max);
                row_edit_midi2din(ui, "midi to din", &mut fader.midi2din);
            });
    });
}

fn render_switch_compare_grid(ui: &mut Ui, switch: &mut Switch) {
    ui.with_layout(Layout::left_to_right(Align::Min), |ui| {
        Grid::new("switch_compare_grid_l")
            .striped(true)
            .spacing([16.0, 6.0])
            .show(ui, |ui| {
                row_edit_switch_kind(ui, "kind", &mut switch.kind);
                row_edit_midi_channel(ui, "channel", &mut switch.channel);
                row_edit_u8(ui, "midicc", &mut switch.midicc);
                row_edit_trigger_kind(ui, "mode", &mut switch.mode);
                row_edit_u8(ui, "prog", &mut switch.prog);
                row_edit_u8(ui, "msb", &mut switch.msb);
            });

        ui.add_space(32.0);

        Grid::new("switch_compare_grid_r")
            .striped(true)
            .spacing([16.0, 6.0])
            .show(ui, |ui| {
                row_edit_u8(ui, "lsb", &mut switch.lsb);
                row_edit_midi2din(ui, "midi to din", &mut switch.midi2din);
                row_edit_u8(ui, "note", &mut switch.note);
                row_edit_u8(ui, "velo", &mut switch.velo);
                row_edit_midi2din(ui, "invert", &mut switch.invert);
            });
    });
}

fn row_edit_u8(ui: &mut Ui, name: &str, value: &mut u8) {
    ui.label(name);
    let mut val = *value as u32;
    if ui.add(DragValue::new(&mut val).range(0..=127)).changed() {
        *value = val as u8;
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

fn row_edit_pad_kind(ui: &mut Ui, name: &str, value: &mut PadKind) {
    ui.label(name);
    ComboBox::from_id_salt(name)
        .selected_text(format!("{}", value))
        .show_ui(ui, |ui| {
            for variant in PadKind::iter() {
                ui.selectable_value(value, variant, format!("{}", variant));
            }
        });
    ui.end_row();
}

fn row_edit_midi_channel(ui: &mut Ui, name: &str, value: &mut MidiChannel) {
    ui.label(name);
    ComboBox::from_id_salt(name)
        .selected_text(format!("{}", value))
        .show_ui(ui, |ui| {
            for variant in MidiChannel::iter() {
                ui.selectable_value(value, variant, format!("{}", variant));
            }
        });
    ui.end_row();
}

fn row_edit_usb_channel(ui: &mut Ui, name: &str, value: &mut UsbChannel) {
    ui.label(name);
    ComboBox::from_id_salt(name)
        .selected_text(format!("{}", value))
        .show_ui(ui, |ui| {
            for variant in UsbChannel::iter() {
                ui.selectable_value(value, variant, format!("{}", variant));
            }
        });
    ui.end_row();
}

fn row_edit_dial_kind(ui: &mut Ui, name: &str, value: &mut DialKind) {
    ui.label(name);
    ComboBox::from_id_salt(name)
        .selected_text(format!("{:?}", value))
        .show_ui(ui, |ui| {
            for variant in DialKind::iter() {
                ui.selectable_value(value, variant, format!("{:?}", variant));
            }
        });
    ui.end_row();
}

fn row_edit_fader_kind(ui: &mut Ui, name: &str, value: &mut FaderKind) {
    ui.label(name);
    ComboBox::from_id_salt(name)
        .selected_text(format!("{:?}", value))
        .show_ui(ui, |ui| {
            for variant in FaderKind::iter() {
                ui.selectable_value(value, variant, format!("{:?}", variant));
            }
        });
    ui.end_row();
}

fn row_edit_switch_kind(ui: &mut Ui, name: &str, value: &mut SwitchKind) {
    ui.label(name);
    ComboBox::from_id_salt(name)
        .selected_text(format!("{:?}", value))
        .show_ui(ui, |ui| {
            for variant in SwitchKind::iter() {
                ui.selectable_value(value, variant, format!("{:?}", variant));
            }
        });
    ui.end_row();
}

fn row_edit_trigger_kind(ui: &mut Ui, name: &str, value: &mut TriggerKind) {
    ui.label(name);
    ComboBox::from_id_salt(name)
        .selected_text(format!("{}", value))
        .show_ui(ui, |ui| {
            for variant in TriggerKind::iter() {
                ui.selectable_value(value, variant, format!("{}", variant));
            }
        });
    ui.end_row();
}

fn row_edit_aftertouch_kind(ui: &mut Ui, name: &str, value: &mut AfterTouchKind) {
    ui.label(name);
    ComboBox::from_id_salt(name)
        .selected_text(format!("{}", value))
        .show_ui(ui, |ui| {
            for variant in AfterTouchKind::iter() {
                ui.selectable_value(value, variant, format!("{}", variant));
            }
        });
    ui.end_row();
}

fn row_edit_pad_color(ui: &mut Ui, name: &str, value: &mut PadColor) {
    ui.label(name);
    ComboBox::from_id_salt(name)
        .selected_text(format!("{}", value))
        .show_ui(ui, |ui| {
            for variant in PadColor::iter() {
                ui.selectable_value(value, variant, format!("{}", variant));
            }
        });
    ui.end_row();
}

fn row_edit_midi2din(ui: &mut Ui, name: &str, value: &mut ActiveState) {
    ui.label(name);
    ComboBox::from_id_salt(name)
        .selected_text(format!("{}", value))
        .show_ui(ui, |ui| {
            for variant in ActiveState::iter() {
                ui.selectable_value(value, variant, format!("{}", variant));
            }
        });
    ui.end_row();
}

fn row_edit_u8_clamped(ui: &mut Ui, name: &str, value: &mut u8, range: RangeInclusive<u8>) {
    ui.label(name);
    ui.add(DragValue::new(value).range(range));
    ui.end_row();
}

fn row_edit_active_state(ui: &mut Ui, name: &str, value: &mut ActiveState) {
    ui.label(name);
    ComboBox::from_id_salt(name)
        .selected_text(format!("{}", value))
        .show_ui(ui, |ui| {
            for variant in ActiveState::iter() {
                ui.selectable_value(value, variant, format!("{}", variant));
            }
        });
    ui.end_row();
}

fn row_edit_tap_average(ui: &mut Ui, name: &str, value: &mut TapAverage) {
    ui.label(name);
    ComboBox::from_id_salt(name)
        .selected_text(format!("{}", value))
        .show_ui(ui, |ui| {
            for variant in TapAverage::iter() {
                ui.selectable_value(value, variant, format!("{}", variant));
            }
        });
    ui.end_row();
}

fn row_edit_note_display(ui: &mut Ui, name: &str, value: &mut NoteDisplay) {
    ui.label(name);
    ComboBox::from_id_salt(name)
        .selected_text(format!("{}", value))
        .show_ui(ui, |ui| {
            for variant in NoteDisplay::iter() {
                ui.selectable_value(value, variant, format!("{}", variant));
            }
        });
    ui.end_row();
}

fn row_edit_pad_curve(ui: &mut Ui, name: &str, value: &mut PadCurve) {
    ui.label(name);
    ComboBox::from_id_salt(name)
        .selected_text(format!("{}", value))
        .show_ui(ui, |ui| {
            for variant in PadCurve::iter() {
                ui.selectable_value(value, variant, format!("{}", variant));
            }
        });
    ui.end_row();
}

fn row_edit_midi_clock(ui: &mut Ui, name: &str, value: &mut MidiClock) {
    ui.label(name);
    ComboBox::from_id_salt(name)
        .selected_text(format!("{}", value))
        .show_ui(ui, |ui| {
            for variant in MidiClock::iter() {
                ui.selectable_value(value, variant, format!("{}", variant));
            }
        });
    ui.end_row();
}

fn row_edit_preset_slot(ui: &mut Ui, name: &str, value: &mut PresetSlot) {
    ui.label(name);
    ComboBox::from_id_salt(name)
        .selected_text(format!("{:?}", value))
        .show_ui(ui, |ui| {
            for variant in PresetSlot::iter() {
                ui.selectable_value(value, variant, format!("{:?}", variant));
            }
        });
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

fn row_edit_tempo(ui: &mut Ui, name: &str, value: &mut Tempo) {
    ui.label(name);
    let mut tempo_val = value.0 as u32;
    if ui
        .add(DragValue::new(&mut tempo_val).range(30..=300))
        .changed()
    {
        *value = Tempo(tempo_val as u16);
    }
    ui.end_row();
}

fn row_edit_time_division(ui: &mut Ui, name: &str, value: &mut TimeDivision) {
    ui.label(name);
    ComboBox::from_id_salt(name)
        .selected_text(format!("{}", value))
        .show_ui(ui, |ui| {
            for variant in TimeDivision::iter() {
                ui.selectable_value(value, variant, format!("{}", variant));
            }
        });
    ui.end_row();
}

fn row_edit_gate(ui: &mut Ui, name: &str, value: &mut GateValue) {
    ui.label(name);
    let mut gate_val = *value as u8;
    if ui
        .add(DragValue::new(&mut gate_val).range(0..=100))
        .changed()
        && let Ok(g) = GateValue::try_from(gate_val)
    {
        *value = g;
    }
    ui.end_row();
}

fn row_edit_swing_kind(ui: &mut Ui, name: &str, value: &mut SwingKind) {
    ui.label(name);
    ComboBox::from_id_salt(name)
        .selected_text(format!("{}", value))
        .show_ui(ui, |ui| {
            for variant in SwingKind::iter() {
                ui.selectable_value(value, variant, format!("{}", variant));
            }
        });
    ui.end_row();
}

fn row_edit_transport_kind(ui: &mut Ui, name: &str, value: &mut TransportKind) {
    ui.label(name);
    ComboBox::from_id_salt(name)
        .selected_text(format!("{}", value))
        .show_ui(ui, |ui| {
            for variant in TransportKind::iter() {
                ui.selectable_value(value, variant, format!("{}", variant));
            }
        });
    ui.end_row();
}
