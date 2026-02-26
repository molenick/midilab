use std::ops::RangeInclusive;

use eframe::egui;
use eframe::egui::Align;
use eframe::egui::Button;
use eframe::egui::CentralPanel;
use eframe::egui::Color32;
use eframe::egui::ComboBox;
use eframe::egui::Context;
use eframe::egui::DragValue;
use eframe::egui::Grid;
use eframe::egui::Layout;
use eframe::egui::MenuBar;
use eframe::egui::Rect;
use eframe::egui::Sense;
use eframe::egui::TextEdit;
use eframe::egui::TopBottomPanel;
use eframe::egui::Ui;
use eframe::egui::Vec2;
use eframe::egui::containers::Modal;
use eframe::egui::vec2;
use midilab::IntoEnumIterator;
use midilab::manufacturer::akai::mpd226::ColorPattern;
use midilab::manufacturer::akai::mpd226::ColorSequence;
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
use midilab::message::UiEffect;
use midilab::midi::MidiNote;
use midilab::midi::MidiValue;
use midilab::music::generation::ChordRowSequence;
use midilab::music::generation::PitchPattern;
use midilab::music::generation::ScaleSequence;
use midilab::music::theory::PitchClass;

use crate::akai_mpd226_editor::state::UiState;
use crate::akai_mpd226_editor::state::UserSelection;
use crate::spacing::spacing;

// todo: further organization for a rainy day
// modal can be a module, look at others

mod palette {
    use eframe::egui::Color32;

    pub(crate) const CONTROL_BACKGROUND: Color32 = Color32::from_rgb(8, 8, 8);

    // todo: maybe the pad colors live here one day
}

const DEFAULT_CONTROL_X: f32 = spacing(4);
const DEFAULT_CONTROL_SPACING: f32 = spacing(0);

const PAD_X: f32 = spacing(4);
const PAD_Y: f32 = spacing(4);
const PAD_DIMENSIONS: Vec2 = Vec2 { x: PAD_X, y: PAD_Y };

const PAD_X_SPACING: f32 = spacing(0);
const PAD_Y_SPACING: f32 = 8.;

const DIAL_X: f32 = DEFAULT_CONTROL_X;
const DIAL_Y: f32 = DEFAULT_CONTROL_X;
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

const PAD_BANKS: [&str; 4] = ["A", "B", "C", "D"];
const CONTROL_BANKS: [&str; 3] = ["A", "B", "C"];
const CONTROLS_PER_CONTROL_BANK: usize = 4;

const CONTROL_BANK_LABEL_X: f32 = DEFAULT_CONTROL_X * 4. + DEFAULT_CONTROL_SPACING * 4.;

pub fn ui(ctx: &Context, ui_state: &mut UiState, outbox: &mut Vec<UiEffect>) {
    TopBottomPanel::top("top_panel").show(ctx, |ui| {
        MenuBar::new().ui(ui, |ui| {
            ui.menu_button("File", |ui| {
                if ui.button("Save preset").clicked() {
                    outbox.push(UiEffect::ShowPresetSaveDialog);
                }

                if ui.button("Load preset").clicked() {
                    outbox.push(UiEffect::ShowPresetLoadDialog);
                }

                ui.separator();

                if ui.button("Quit").clicked() {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
            });
            ui.menu_button("Edit", |ui| {
                ui.menu_button("Pad Mapping", |ui| {
                    if ui.button("Notes").clicked() {
                        ui_state.selected_item = Some(UserSelection::PadNoteMapping);
                    }

                    if ui.button("LED Off Color").clicked() {
                        ui_state.selected_item = Some(UserSelection::PadOffColorMapping);
                    }

                    if ui.button("LED On Color").clicked() {
                        ui_state.selected_item = Some(UserSelection::PadOnColorMapping);
                    }
                });

                if ui.button("Edit Preset settings").clicked() {
                    ui_state.selected_item = Some(UserSelection::PresetSettings);
                }
                if ui.button("Edit Global settings").clicked() {
                    ui_state.selected_item = Some(UserSelection::GlobalSettings);
                }

                if ui.button("Blank Preset").clicked() {
                    ui_state.preset = Preset::blank();
                }

                if ui.button("Default Preset").clicked() {
                    ui_state.preset = Preset::default();
                }
            });
        });
    });

    if ui_state.selected_item.is_some() {
        modal_editor(ctx, ui_state, outbox);
    } else {
        CentralPanel::default().show(ctx, |ui| {
            editor_actions(ui, ui_state, outbox);

            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    controls(ui, ui_state);
                });
            });
        });
    }
}

fn editor_actions(ui: &mut Ui, ui_state: &mut UiState, outbox: &mut Vec<UiEffect>) {
    ui.vertical(|ui| {
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.label("Preset Quick Actions");
                ui.horizontal(|ui| {
                    if ui.button("Dump").clicked() {
                        ui_state.user_msg = None;
                        outbox.push(UiEffect::DumpPreset(ui_state.preset.settings.slot));
                    }

                    if ui.button("Write").clicked() {
                        ui_state.user_msg = None;
                        outbox.push(UiEffect::WritePreset(Box::new(ui_state.preset)));
                    }

                    ui.separator();

                    row_edit_enum(ui, "Slot", &mut ui_state.preset.settings.slot);
                    row_edit_preset_name(ui, "Name", &mut ui_state.preset.settings.name);

                    ui.separator();

                    ui.label("Pads:");
                    ui.label("Aftertouch");
                    enum_combo_box(ui, "aftertouch", &mut ui_state.aftertouch_kind, None);
                    // todo: this means 2 taps, but we prob want to reduce to 1 later
                    if ui.button("Apply").clicked() {
                        ui_state
                            .preset
                            .pads
                            .map_aftertouch(ui_state.aftertouch_kind);
                    }

                    if ui.button("Note Mapping").clicked() {
                        ui_state.selected_item = Some(UserSelection::PadNoteMapping);
                    }
                });

                ui.separator();

                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label("Global Quick Actions");
                        ui.horizontal(|ui| {
                            if ui.button("Dump").clicked() {
                                ui_state.user_msg = None;
                                outbox.push(UiEffect::RequestGlobalFromDevice);
                            }

                            if ui.button("Write").clicked() {
                                ui_state.user_msg = None;
                                outbox
                                    .push(UiEffect::SendGlobalToDevice(Box::new(ui_state.global)));
                            }
                            ui.separator();

                            row_edit_u8_clamped(
                                ui,
                                "Threshold",
                                &mut ui_state.global.pad_threshold,
                                1..=10,
                            );

                            row_edit_enum(ui, "Curve", &mut ui_state.global.pad_curve);
                            row_edit_u8_clamped(ui, "Gain", &mut ui_state.global.pad_gain, 0..=20);
                        });
                    });
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

fn preset_settings_editor(ui: &mut Ui, preset_settings: &mut PresetSettings) {
    modal_control_editor_help(ui, "preset settings");

    Grid::new("preset_settings_grid")
        .striped(true)
        .show(ui, |ui| {
            row_edit_enum(ui, "Preset Slot", &mut preset_settings.slot);
            row_edit_preset_name(ui, "Preset Name", &mut preset_settings.name);
            row_edit_u16_clamped(ui, "Tempo", &mut preset_settings.tempo.0, 30..=300);
            row_edit_enum(ui, "Division", &mut preset_settings.time_division);
            row_edit_enum(ui, "Div Switch", &mut preset_settings.time_division_switch);
            row_edit_enum(ui, "Note Repeat", &mut preset_settings.note_repeat_switch);
            row_edit_u8_clamped(ui, "Gate", &mut preset_settings.gate.value, 1..=99);
            row_edit_enum(ui, "Swing", &mut preset_settings.swing);
            row_edit_enum(ui, "Transport", &mut preset_settings.transport);
        });
}

fn global_settings_editor(ui: &mut Ui, ui_state: &mut UiState) {
    modal_control_editor_help(ui, "global settings");

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

fn note_mapping(ui: &mut Ui, ui_state: &mut UiState, outbox: &mut Vec<UiEffect>) {
    modal_note_mapper_help(ui);

    ui.label("Note Pattern");
    ComboBox::from_id_salt("note_pattern_kind")
        .selected_text(ui_state.note_mapping.pattern.to_string())
        .show_ui(ui, |ui| {
            ui.selectable_value(
                &mut ui_state.note_mapping.pattern,
                PitchPattern::Scale(ScaleSequence::default()),
                "Scale",
            );
            ui.selectable_value(
                &mut ui_state.note_mapping.pattern,
                PitchPattern::ChordRow(ChordRowSequence::default()),
                "Chord Row",
            );
        });

    match &mut ui_state.note_mapping.pattern {
        PitchPattern::Scale(seq) => {
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
                ui.add(egui::DragValue::new(&mut seq.octave.0).range(-2_i8..=9_i8));
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

        PitchPattern::ChordRow(seq) => {
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
                ui.add(egui::DragValue::new(&mut seq.octave.0).range(-2_i8..=9_i8));
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

    ui.separator();

    ui.label("Color Mapping");
    note_color_map_editor(ui, &mut ui_state.note_mapping.color_map);

    ui.separator();

    ui.horizontal(|ui| {
        if ui.button("Update editor").clicked() {
            let pattern = ui_state.note_mapping.pattern;
            ui_state.preset.pads.set_note_pattern_with_off_colors(
                ui_state.note_mapping.starting_from_pad,
                pattern,
                ui_state.note_mapping.color_map.clone(),
            );

            ui_state.selected_item = None;
        }

        if ui.button("Update device").clicked() {
            let pattern = ui_state.note_mapping.pattern;
            ui_state.preset.pads.set_note_pattern_with_off_colors(
                ui_state.note_mapping.starting_from_pad,
                pattern,
                ui_state.note_mapping.color_map.clone(),
            );

            outbox.push(UiEffect::WritePreset(ui_state.preset.into()));

            ui_state.selected_item = None;
        }
    });
}

fn note_color_map_editor(ui: &mut Ui, color_map: &mut NoteColorMap) {
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
                let (swatch_rect, _) = ui.allocate_exact_size(swatch_size, egui::Sense::empty());
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

fn off_color_mapping(ui: &mut Ui, ui_state: &mut UiState) {
    modal_color_mapper_help(ui);

    color_pattern_editor(ui, "off", &mut ui_state.off_color_mapping.pattern);

    ui.horizontal(|ui| {
        ui.label("Start Pad");
        ui.add(DragValue::new(&mut ui_state.off_color_mapping.starting_from_pad).range(0..=63));
    });

    ui.horizontal(|ui| {
        ui.label("Length");
        ui.add(DragValue::new(&mut ui_state.off_color_mapping.length).range(1..=64));
    });

    ui.separator();

    if ui.button("Apply to Editor").clicked() {
        ui_state.preset.pads.set_off_color_pattern(
            ui_state.off_color_mapping.starting_from_pad,
            ui_state.off_color_mapping.length,
            ui_state.off_color_mapping.pattern.clone(),
        );
    }
}

fn on_color_mapping(ui: &mut Ui, ui_state: &mut UiState) {
    modal_color_mapper_help(ui);

    color_pattern_editor(ui, "on", &mut ui_state.on_color_mapping.pattern);

    ui.horizontal(|ui| {
        ui.label("Start Pad");
        ui.add(DragValue::new(&mut ui_state.on_color_mapping.starting_from_pad).range(0..=63));
    });

    ui.horizontal(|ui| {
        ui.label("Length");
        ui.add(DragValue::new(&mut ui_state.on_color_mapping.length).range(1..=64));
    });

    ui.separator();

    if ui.button("Apply to Editor").clicked() {
        ui_state.preset.pads.set_on_color_pattern(
            ui_state.on_color_mapping.starting_from_pad,
            ui_state.on_color_mapping.length,
            ui_state.on_color_mapping.pattern.clone(),
        );
    }
}

fn color_pattern_editor(ui: &mut Ui, id_prefix: &str, pattern: &mut ColorPattern) {
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

fn all_pad_banks(ui: &mut Ui, selected_item: &mut Option<UserSelection>, pad_repo: PadRepository) {
    let banks: Vec<Vec<Pad>> = pad_repo
        .pads
        .chunks(16)
        .map(|chunk| chunk.to_vec())
        .collect();

    ui.horizontal(|ui| {
        for (bank_id, bank) in banks.into_iter().enumerate() {
            let bank_label = PAD_BANKS[bank_id].to_string();
            pad_bank(ui, selected_item, bank, bank_label);
        }
    });
}

fn pad_bank(ui: &mut Ui, selected_item: &mut Option<UserSelection>, pads: Vec<Pad>, label: String) {
    ui.vertical(|ui| {
        ui.add_space(spacing(0));
        ui.label(format!("Pad Bank {label}"));
        ui.add_space(spacing(0));

        let grid_rect = ui
            .allocate_exact_size(
                Vec2::new((PAD_X + PAD_X_SPACING) * 4., (PAD_Y + PAD_Y_SPACING) * 4.),
                Sense::empty(),
            )
            .0;
        let top_left = grid_rect.min;

        let rows = pads.chunks(4);
        for (row_idx, row) in rows.enumerate() {
            for (pad_idx, pad) in row.iter().enumerate() {
                let visual_row = 3 - row_idx;
                let x = top_left.x + pad_idx as f32 * (PAD_X) + (pad_idx as f32 * PAD_X_SPACING);
                let y =
                    top_left.y + visual_row as f32 * (PAD_Y) + (visual_row as f32 * PAD_Y_SPACING);

                let rect = egui::Rect::from_min_size(egui::Pos2::new(x, y), PAD_DIMENSIONS);
                let mut child_ui = ui.new_child(
                    egui::UiBuilder::new()
                        .max_rect(rect)
                        .layout(egui::Layout::default()),
                );
                pad_button(&mut child_ui, selected_item, *pad);
            }
        }
    });
}

fn pad_button(ui: &mut Ui, selected_item: &mut Option<UserSelection>, pad: Pad) {
    // paint on/off colors for pad
    let cursor_pos = ui.cursor().min;
    let rect = Rect::from_min_size(cursor_pos, PAD_DIMENSIONS);
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

    // this determines the color and size of the rectangular mask overlaying the on/off colors
    ui.painter()
        .rect_filled(rect.shrink(3.0), 0.0, palette::CONTROL_BACKGROUND);

    ui.with_layout(
        Layout::centered_and_justified(egui::Direction::TopDown),
        |ui| {
            let label = format!("Pad {}", pad.id);
            let button = Button::new(label)
                .fill(egui::Color32::TRANSPARENT)
                .min_size(PAD_DIMENSIONS);

            let resp = ui.add(button);

            if resp.clicked() {
                *selected_item = Some(UserSelection::Pad { id: pad.id });
            }
        },
    );
}

fn controls(ui: &mut Ui, ui_state: &mut UiState) {
    let pads = ui_state.preset.pads;
    let dials = ui_state.preset.dials;
    let faders = ui_state.preset.faders;
    let switches = ui_state.preset.switches;

    all_pad_banks(ui, &mut ui_state.selected_item, pads);
    ui.separator();
    ui.add_space(spacing(0));
    all_control_banks(ui, &mut ui_state.selected_item, dials, faders, switches);
}

fn all_control_banks(
    ui: &mut Ui,
    selected_item: &mut Option<UserSelection>,
    dial_repo: DialRepository,
    fader_repo: FaderRepository,
    switch_repo: SwitchRepository,
) {
    ui.horizontal(|ui| {
        for bank_label in CONTROL_BANKS.iter() {
            let (rect, _) =
                ui.allocate_exact_size(vec2(CONTROL_BANK_LABEL_X, 20.0), egui::Sense::empty());
            ui.painter().text(
                rect.left_center() + vec2(4.0, 0.0),
                egui::Align2::LEFT_CENTER,
                format!("Control Bank {}", bank_label),
                egui::FontId::default(),
                ui.style().visuals.text_color(),
            );
        }
    });

    ui.add_space(DEFAULT_CONTROL_SPACING);

    ui.horizontal_top(|ui| {
        for dial_row in dial_repo.0.chunks(CONTROLS_PER_CONTROL_BANK) {
            for dial in dial_row.iter() {
                dial_button(ui, selected_item, *dial)
            }
            ui.add_space(DEFAULT_CONTROL_SPACING);
        }
    });

    ui.add_space(DEFAULT_CONTROL_SPACING);

    ui.horizontal_top(|ui| {
        for fader_row in fader_repo.0.chunks(CONTROLS_PER_CONTROL_BANK) {
            for fader in fader_row.iter() {
                fader_button(ui, selected_item, *fader)
            }
            ui.add_space(DEFAULT_CONTROL_SPACING);
        }
    });

    ui.add_space(DEFAULT_CONTROL_SPACING);

    ui.horizontal_top(|ui| {
        for switch_row in switch_repo.0.chunks(CONTROLS_PER_CONTROL_BANK) {
            for switch in switch_row.iter() {
                switch_button(ui, selected_item, *switch)
            }
            ui.add_space(DEFAULT_CONTROL_SPACING);
        }
    });
}

fn dial_button(ui: &mut Ui, selected_item: &mut Option<UserSelection>, dial: Dial) {
    let full_label = format!("Dial {}", dial.id);

    let button = egui::Button::new(full_label.clone())
        .min_size(DIAL_DIMENSIONS)
        .fill(palette::CONTROL_BACKGROUND)
        .corner_radius(200.0)
        .wrap();

    let resp = ui.add_sized(DIAL_DIMENSIONS, button);

    if resp.clicked() {
        *selected_item = Some(UserSelection::Dial { id: dial.id });
    }
}

fn fader_button(ui: &mut Ui, selected_item: &mut Option<UserSelection>, fader: Fader) {
    let full_label = format!("Fader {}", fader.id);

    let button: egui::Button<'_> = egui::Button::new(full_label.clone())
        .min_size(FADER_DIMENSIONS)
        .fill(palette::CONTROL_BACKGROUND)
        .corner_radius(4.0)
        .wrap();

    let resp = ui.add_sized(FADER_DIMENSIONS, button);

    if resp.clicked() {
        *selected_item = Some(UserSelection::Fader { id: fader.id });
    }
}

fn switch_button(ui: &mut Ui, selected_item: &mut Option<UserSelection>, switch: Switch) {
    let full_label = format!("Switch {}", switch.id);

    let button = egui::Button::new(full_label.clone())
        .min_size(SWITCH_DIMENSIONS)
        .fill(palette::CONTROL_BACKGROUND)
        .corner_radius(4.0)
        .wrap();

    let resp = ui.add_sized(SWITCH_DIMENSIONS, button);

    if resp.clicked() {
        *selected_item = Some(UserSelection::Switch { id: switch.id });
    }
}

fn modal_editor(ctx: &Context, ui_state: &mut UiState, outbox: &mut Vec<UiEffect>) {
    let Some(selected_item) = ui_state.selected_item else {
        return;
    };

    let modal_response = Modal::new(egui::Id::new("control_editor_modal")).show(ctx, |ui| {
        ui.heading(selected_item.to_string());
        ui.separator();

        match selected_item {
            UserSelection::Pad { id } => {
                if let Some(pad) = ui_state.preset.pads.pads.iter_mut().find(|p| p.id == id) {
                    pad_editor(ui, pad);
                }
            }
            UserSelection::Dial { id } => {
                if let Some(dial) = ui_state.preset.dials.0.get_mut(id.0) {
                    dial_editor(ui, dial);
                }
            }
            UserSelection::Fader { id } => {
                if let Some(fader) = ui_state.preset.faders.0.get_mut(id.0) {
                    fader_editor(ui, fader);
                }
            }
            UserSelection::Switch { id } => {
                if let Some(switch) = ui_state.preset.switches.0.get_mut(id.0) {
                    switch_editor(ui, switch);
                }
            }
            UserSelection::PresetSettings => {
                preset_settings_editor(ui, &mut ui_state.preset.settings);
            }
            UserSelection::GlobalSettings => global_settings_editor(ui, ui_state),
            UserSelection::PadNoteMapping => note_mapping(ui, ui_state, outbox),
            UserSelection::PadOffColorMapping => off_color_mapping(ui, ui_state),
            UserSelection::PadOnColorMapping => on_color_mapping(ui, ui_state),
        };
    });

    if modal_response.should_close() {
        ui_state.selected_item = None;
    }
}

fn pad_editor(ui: &mut Ui, pad: &mut Pad) {
    modal_control_editor_help(ui, "pad");

    ui.with_layout(Layout::left_to_right(Align::Min), |ui| {
        Grid::new("pad_compare_grid_l")
            .striped(true)
            .show(ui, |ui| {
                row_edit_enum(ui, "kind", &mut pad.kind);
                row_edit_enum(ui, "channel", &mut pad.channel);
                row_edit_midi_note(ui, "note", &mut pad.note);
                row_edit_enum(ui, "midi to din", &mut pad.midi2din);
                row_edit_enum(ui, "trigger mode", &mut pad.mode);
                row_edit_enum(ui, "aftertouch", &mut pad.aftertouch);
                row_edit_midi_value(ui, "program", &mut pad.program);
                row_edit_midi_value(ui, "msb", &mut pad.msb);
                row_edit_midi_value(ui, "lsb", &mut pad.lsb);
                row_edit_enum(ui, "off color", &mut pad.off_color);
                row_edit_enum(ui, "on color", &mut pad.on_color);
            });
    });
}

fn dial_editor(ui: &mut Ui, dial: &mut Dial) {
    modal_control_editor_help(ui, "dial");

    ui.with_layout(Layout::left_to_right(Align::Min), |ui| {
        Grid::new("dial_compare_grid_l")
            .striped(true)
            .show(ui, |ui| {
                row_edit_enum(ui, "kind", &mut dial.kind);
                row_edit_enum(ui, "channel", &mut dial.channel);
                row_edit_midi_value(ui, "midicc", &mut dial.midicc);
                row_edit_midi_value(ui, "min", &mut dial.min);
                row_edit_midi_value(ui, "max", &mut dial.max);
                row_edit_enum(ui, "midi to din", &mut dial.midi2din);
                row_edit_midi_value(ui, "msb", &mut dial.msb);
                row_edit_midi_value(ui, "lsb", &mut dial.lsb);
                row_edit_midi_value(ui, "value", &mut dial.value);
            });
    });
}

fn fader_editor(ui: &mut Ui, fader: &mut Fader) {
    modal_control_editor_help(ui, "fader");

    ui.with_layout(Layout::left_to_right(Align::Min), |ui| {
        Grid::new("fader_compare_grid_l")
            .striped(true)
            .show(ui, |ui| {
                row_edit_enum(ui, "kind", &mut fader.kind);
                row_edit_enum(ui, "channel", &mut fader.channel);
                row_edit_midi_value(ui, "midicc", &mut fader.midicc);
                row_edit_midi_value(ui, "min", &mut fader.min);
                row_edit_midi_value(ui, "max", &mut fader.max);
                row_edit_enum(ui, "midi to din", &mut fader.midi2din);
            });
    });
}

fn switch_editor(ui: &mut Ui, switch: &mut Switch) {
    modal_control_editor_help(ui, "switch");
    ui.with_layout(Layout::left_to_right(Align::Min), |ui| {
        Grid::new("switch_compare_grid_l")
            .striped(true)
            .show(ui, |ui| {
                row_edit_enum(ui, "kind", &mut switch.kind);
                row_edit_enum(ui, "channel", &mut switch.channel);
                row_edit_midi_value(ui, "midicc", &mut switch.midicc);
                row_edit_enum(ui, "trigger mode", &mut switch.mode);
                row_edit_midi_value(ui, "prog", &mut switch.prog);
                row_edit_midi_value(ui, "msb", &mut switch.msb);
                row_edit_midi_value(ui, "lsb", &mut switch.lsb);
                row_edit_enum(ui, "midi to din", &mut switch.midi2din);
                row_edit_midi_note(ui, "note", &mut switch.note);
                row_edit_midi_value(ui, "velo", &mut switch.velo);
                row_edit_enum(ui, "invert", &mut switch.invert);
            });
    });
}

fn modal_control_editor_help(ui: &mut Ui, control_name: impl Into<String>) {
    ui.label("Help");
    ui.label(format!("Applies changes to {}", control_name.into()));
    ui.label("Press Esc key or tap outside modal to exit");
    ui.separator();
}

fn modal_note_mapper_help(ui: &mut Ui) {
    ui.label("Help");
    ui.label("Apply note mapping to editor or device");
    ui.label("Press Esc key or tap outside modal to exit");
    ui.separator();
}

fn modal_color_mapper_help(ui: &mut Ui) {
    ui.label("Help");
    ui.label("Create color mappings and apply to editor");
    ui.label("Press Esc key or tap outside modal to exit");
    ui.separator();
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

fn row_edit_midi_note(ui: &mut Ui, name: &str, value: &mut MidiNote) {
    ui.label(name);
    let mut val: u8 = (*value).into();
    if ui.add(DragValue::new(&mut val).range(0..=127)).changed() {
        *value = MidiNote::from(val);
    }
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
