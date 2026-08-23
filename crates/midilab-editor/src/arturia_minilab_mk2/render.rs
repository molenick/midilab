use std::fmt::Display;

use eframe::egui;
use eframe::egui::Color32;
use eframe::egui::Context;
use eframe::egui::DragValue;
use eframe::egui::Grid;
use eframe::egui::RichText;
use eframe::egui::ScrollArea;
use eframe::egui::Ui;
use eframe::egui::containers::Modal;
use midilab::IntoEnumIterator;
use midilab::manufacturer::arturia::minilab_mk2::Global;
use midilab::manufacturer::arturia::minilab_mk2::Preset;
use midilab::manufacturer::arturia::minilab_mk2::control::Button;
use midilab::manufacturer::arturia::minilab_mk2::control::Knob;
use midilab::manufacturer::arturia::minilab_mk2::control::Pad;
use midilab::manufacturer::arturia::minilab_mk2::control::value_kind::ButtonMode;
use midilab::manufacturer::arturia::minilab_mk2::control::value_kind::KnobAcceleration;
use midilab::manufacturer::arturia::minilab_mk2::control::value_kind::KnobMode;
use midilab::manufacturer::arturia::minilab_mk2::control::value_kind::KnobOption;
use midilab::manufacturer::arturia::minilab_mk2::control::value_kind::MemorySlot;
use midilab::manufacturer::arturia::minilab_mk2::control::value_kind::MidiChannel;
use midilab::manufacturer::arturia::minilab_mk2::control::value_kind::ModWheelMode;
use midilab::manufacturer::arturia::minilab_mk2::control::value_kind::NrpnRpn;
use midilab::manufacturer::arturia::minilab_mk2::control::value_kind::PadColor;
use midilab::manufacturer::arturia::minilab_mk2::control::value_kind::PadMode;
use midilab::manufacturer::arturia::minilab_mk2::control::value_kind::PitchBendMode;
use midilab::manufacturer::arturia::minilab_mk2::control::value_kind::PitchBendOption;
use midilab::manufacturer::arturia::minilab_mk2::control::value_kind::SustainPedalMode;
use midilab::manufacturer::arturia::minilab_mk2::control::value_kind::SwitchBehavior;
use midilab::manufacturer::arturia::minilab_mk2::control::value_kind::ToggleState;
use midilab::manufacturer::arturia::minilab_mk2::control::value_kind::VelocityCurve;
use midilab::midi::Value;

use crate::arturia_minilab_mk2::message::UiEffect;
use crate::arturia_minilab_mk2::message::UserMsgKind;
use crate::arturia_minilab_mk2::state::EditorTab;
use crate::arturia_minilab_mk2::state::UiState;

pub fn ui(ui: &mut Ui, ui_state: &mut UiState, outbox: &mut Vec<UiEffect>) {
    egui::Panel::top("top_panel").show(ui, |ui| {
        egui::MenuBar::new().ui(ui, |ui| {
            ui.menu_button("File", |ui| {
                if ui.button("Read Preset").clicked() {
                    outbox.push(UiEffect::ReadPreset);
                }
                if ui.button("Save Preset").clicked() {
                    outbox.push(UiEffect::ShowPresetSaveDialog);
                }
                if ui.button("Load Preset").clicked() {
                    outbox.push(UiEffect::ShowPresetLoadDialog);
                }
                ui.separator();
                if ui.button("Read Global").clicked() {
                    outbox.push(UiEffect::ReadGlobal);
                }
                if ui.button("Save Global").clicked() {
                    outbox.push(UiEffect::ShowGlobalSaveDialog);
                }
                if ui.button("Load Global").clicked() {
                    outbox.push(UiEffect::ShowGlobalLoadDialog);
                }
                ui.separator();
                if ui.button("Settings...").clicked() {
                    outbox.push(UiEffect::ShowSettingsModal);
                }
                ui.separator();
                if ui.button("Quit").clicked() {
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                }
            });
            ui.menu_button("Edit", |ui| {
                if ui.button("Blank Preset").clicked() {
                    ui_state.preset = Preset::default();
                }
                if ui.button("Blank Global").clicked() {
                    ui_state.global = Global::default();
                }
            });
        });
    });

    if ui_state.show_settings {
        render_settings_modal(ui.ctx(), ui_state, outbox);
    } else {
        render_main_editor(ui, ui_state, outbox);
    }
}

fn render_settings_modal(ctx: &Context, ui_state: &mut UiState, outbox: &mut Vec<UiEffect>) {
    let response = Modal::new(egui::Id::new("settings_modal")).show(ctx, |ui| {
        ui.heading("Settings");
        ui.separator();
        let mut checked = ui_state.user_settings.auto_sync_enabled;
        if ui
            .checkbox(
                &mut checked,
                "AutoSync: Sync Preset & Global from device on app start",
            )
            .changed()
        {
            ui_state.user_settings.auto_sync_enabled = checked;
            let config = crate::arturia_minilab_mk2::config::AppConfig {
                persistence_path: ui_state.configured_directory.clone(),
                user: ui_state.user_settings.clone(),
            };
            let config_path = crate::arturia_minilab_mk2::config::AppConfig::config_path()
                .expect("Failed to get config path");
            outbox.push(UiEffect::PersistUserSettings {
                config,
                path: config_path,
            });
        }
        ui.separator();
        if ui.button("Close").clicked() {
            ui_state.show_settings = false;
        }
    });
    if response.should_close() {
        ui_state.show_settings = false;
    }
}

fn render_main_editor(ui: &mut Ui, ui_state: &mut UiState, outbox: &mut Vec<UiEffect>) {
    egui::Panel::left("device_panel")
        .default_size(170.)
        .min_size(140.)
        .max_size(220.)
        .show(ui, |ui| {
            ui.add_space(8.);
            ui.heading("Device");
            ui.separator();

            if ui.button("Read Preset").clicked() {
                outbox.push(UiEffect::ReadPreset);
            }
            if ui.button("Write Preset").clicked() {
                outbox.push(UiEffect::WritePreset(Box::new(ui_state.preset)));
            }
            ui.separator();
            if ui.button("Read Global").clicked() {
                outbox.push(UiEffect::ReadGlobal);
            }
            if ui.button("Write Global").clicked() {
                outbox.push(UiEffect::WriteGlobal(ui_state.global));
            }

            ui.separator();
            for tab in EditorTab::ALL {
                if ui
                    .selectable_label(ui_state.editor_tab == tab, tab.label())
                    .clicked()
                {
                    ui_state.editor_tab = tab;
                }
            }
        });

    egui::Panel::bottom("status_panel").show(ui, |ui| {
        if let Some(msg) = &ui_state.user_msg {
            let color = match msg.kind {
                UserMsgKind::Status => Color32::LIGHT_GREEN,
                UserMsgKind::Error => Color32::LIGHT_RED,
            };
            ui.label(RichText::new(&msg.msg).color(color));
        } else {
            ui.label("");
        }
    });

    egui::CentralPanel::default().show(ui, |ui| {
        ScrollArea::vertical().show(ui, |ui| match ui_state.editor_tab {
            EditorTab::Knobs => render_knobs(ui, ui_state),
            EditorTab::Pads => render_pads(ui, ui_state, outbox),
            EditorTab::Controller => render_controller(ui, ui_state),
            EditorTab::Global => render_global(ui, ui_state),
            EditorTab::Memory => render_memory(ui, ui_state, outbox),
        });
    });
}

fn enum_combo<T>(ui: &mut Ui, id: impl std::hash::Hash + std::fmt::Debug, current: &mut T)
where
    T: IntoEnumIterator + Display + PartialEq + Copy,
{
    egui::ComboBox::from_id_salt(id)
        .selected_text(current.to_string())
        .show_ui(ui, |ui| {
            for variant in T::iter() {
                ui.selectable_value(current, variant, variant.to_string());
            }
        });
}

fn value_drag(ui: &mut Ui, value: &mut Value) {
    let mut v = value.as_u8();
    if ui
        .add(DragValue::new(&mut v).range(Value::MIN..=Value::MAX))
        .changed()
    {
        *value = v.into();
    }
}

fn note_drag(ui: &mut Ui, note: &mut midilab::midi::Note) {
    let mut v: u8 = (*note).into();
    let response = ui.add(DragValue::new(&mut v).range(0..=127));
    if response.changed() {
        *note = midilab::midi::Note::from(v);
    }
    ui.label(note.to_string());
}

fn knob_option_combo(ui: &mut Ui, id: impl std::hash::Hash + std::fmt::Debug, knob: &mut Knob) {
    match knob.mode {
        KnobMode::Control => {
            let mut option = KnobOption::try_from(knob.option.as_u8()).unwrap_or_default();
            let before = option;
            enum_combo(ui, id, &mut option);
            if option != before {
                knob.option = u8::from(option).into();
            }
        }
        KnobMode::Nrpn => {
            let mut option = NrpnRpn::try_from(knob.option.as_u8()).unwrap_or_default();
            let before = option;
            enum_combo(ui, id, &mut option);
            if option != before {
                knob.option = u8::from(option).into();
            }
        }
        KnobMode::Off => {
            ui.label("-");
        }
    }
}

fn behavior_combo(ui: &mut Ui, id: impl std::hash::Hash + std::fmt::Debug, option: &mut Value) {
    let mut behavior = SwitchBehavior::try_from(option.as_u8()).unwrap_or_default();
    let before = behavior;
    enum_combo(ui, id, &mut behavior);
    if behavior != before {
        *option = u8::from(behavior).into();
    }
}

fn knob_row(ui: &mut Ui, index: usize, knob: &mut Knob) {
    ui.label(knob.id.to_string());
    enum_combo(ui, ("knob_mode", index), &mut knob.mode);
    enum_combo(ui, ("knob_channel", index), &mut knob.channel);
    value_drag(ui, &mut knob.cc);
    value_drag(ui, &mut knob.min);
    value_drag(ui, &mut knob.max);
    knob_option_combo(ui, ("knob_option", index), knob);
    ui.end_row();
}

fn button_row(ui: &mut Ui, index: usize, button: &mut Button) {
    ui.label(button.id.to_string());
    enum_combo(ui, ("button_mode", index), &mut button.mode);
    enum_combo(ui, ("button_channel", index), &mut button.channel);
    note_drag(ui, &mut button.note);
    value_drag(ui, &mut button.off_value);
    value_drag(ui, &mut button.on_value);
    if button.mode == ButtonMode::Off {
        ui.label("-");
    } else {
        behavior_combo(ui, ("button_option", index), &mut button.option);
    }
    ui.end_row();
}

fn render_knobs(ui: &mut Ui, ui_state: &mut UiState) {
    ui.heading("Knobs");
    ui.separator();

    Grid::new("knobs_grid").striped(true).show(ui, |ui| {
        ui.label("");
        ui.label("Mode");
        ui.label("Channel");
        ui.label("CC / Data");
        ui.label("Min / LSB");
        ui.label("Max / MSB");
        ui.label("Option");
        ui.end_row();

        for (index, knob) in ui_state.preset.knobs.knobs.iter_mut().enumerate() {
            knob_row(ui, index, knob);
        }
    });

    ui.add_space(12.);
    ui.heading("Shift Layer");
    ui.separator();

    Grid::new("shift_knobs_grid").striped(true).show(ui, |ui| {
        for (index, knob) in ui_state.preset.knobs.shift_knobs.iter_mut().enumerate() {
            knob_row(ui, index + 100, knob);
        }
    });

    ui.add_space(12.);
    ui.heading("Knob Switches & Octave Buttons");
    ui.separator();

    Grid::new("buttons_grid").striped(true).show(ui, |ui| {
        ui.label("");
        ui.label("Mode");
        ui.label("Channel");
        ui.label("Note");
        ui.label("");
        ui.label("Off");
        ui.label("On");
        ui.label("Option");
        ui.end_row();

        for (index, button) in ui_state.preset.buttons.buttons.iter_mut().enumerate() {
            button_row(ui, index, button);
        }
    });
}

fn pad_color_swatch(ui: &mut Ui, index: usize, pad: &mut Pad, outbox: &mut Vec<UiEffect>) {
    let (r, g, b) = *pad.color.as_rgb_color();
    let color = Color32::from_rgb(r, g, b);

    egui::ComboBox::from_id_salt(("pad_color", index))
        .selected_text(RichText::new(pad.color.to_string()).color(color))
        .show_ui(ui, |ui| {
            for variant in PadColor::iter() {
                let (r, g, b) = *variant.as_rgb_color();
                ui.selectable_value(
                    &mut pad.color,
                    variant,
                    RichText::new(variant.to_string()).color(Color32::from_rgb(r, g, b)),
                );
            }
        });

    if ui
        .button("Preview")
        .on_hover_text("Light this pad on the device with the selected color")
        .clicked()
    {
        outbox.push(UiEffect::LivePadColor {
            pad: pad.id,
            color: pad.color,
        });
    }
}

fn render_pads(ui: &mut Ui, ui_state: &mut UiState, outbox: &mut Vec<UiEffect>) {
    ui.heading("Pads");
    ui.separator();

    Grid::new("pads_grid").striped(true).show(ui, |ui| {
        ui.label("");
        ui.label("Mode");
        ui.label("Channel");
        ui.label("Note / Data");
        ui.label("");
        ui.label("Off / LSB");
        ui.label("On / MSB");
        ui.label("Option");
        ui.label("Color");
        ui.end_row();

        for (index, pad) in ui_state.preset.pads.pads.iter_mut().enumerate() {
            ui.label(pad.id.to_string());
            enum_combo(ui, ("pad_mode", index), &mut pad.mode);
            enum_combo(ui, ("pad_channel", index), &mut pad.channel);
            note_drag(ui, &mut pad.note);
            value_drag(ui, &mut pad.off_value);
            value_drag(ui, &mut pad.on_value);
            match pad.mode {
                PadMode::SwitchedControl | PadMode::Note => {
                    behavior_combo(ui, ("pad_option", index), &mut pad.option);
                }
                PadMode::Mmc => {
                    value_drag(ui, &mut pad.option);
                }
                PadMode::Off | PadMode::PatchChange => {
                    ui.label("-");
                }
            }
            pad_color_swatch(ui, index, pad, outbox);
            ui.end_row();
        }
    });
}

fn render_controller(ui: &mut Ui, ui_state: &mut UiState) {
    ui.heading("Mod Wheel");
    ui.separator();

    let wheel = &mut ui_state.preset.mod_wheel;
    Grid::new("mod_wheel_grid").show(ui, |ui| {
        ui.label("Mode");
        enum_combo(ui, "wheel_mode", &mut wheel.mode);
        ui.end_row();
        ui.label("Channel");
        enum_combo(ui, "wheel_channel", &mut wheel.channel);
        ui.end_row();
        ui.label("CC Number");
        value_drag(ui, &mut wheel.cc);
        ui.end_row();
        ui.label("Min Value");
        value_drag(ui, &mut wheel.min);
        ui.end_row();
        ui.label("Max Value");
        value_drag(ui, &mut wheel.max);
        ui.end_row();
        if wheel.mode == ModWheelMode::Nrpn {
            ui.label("NRPN/RPN");
            let mut option = NrpnRpn::try_from(wheel.option.as_u8()).unwrap_or_default();
            let before = option;
            enum_combo(ui, "wheel_option", &mut option);
            if option != before {
                wheel.option = u8::from(option).into();
            }
            ui.end_row();
        }
    });

    ui.add_space(12.);
    ui.heading("Pitch Bend");
    ui.separator();

    let bend = &mut ui_state.preset.pitch_bend;
    Grid::new("pitch_bend_grid").show(ui, |ui| {
        ui.label("Mode");
        enum_combo(ui, "bend_mode", &mut bend.mode);
        ui.end_row();
        ui.label("Channel");
        enum_combo(ui, "bend_channel", &mut bend.channel);
        ui.end_row();
        if bend.mode == PitchBendMode::PitchBend {
            ui.label("Option");
            let mut option = PitchBendOption::try_from(bend.option.as_u8()).unwrap_or_default();
            let before = option;
            enum_combo(ui, "bend_option", &mut option);
            if option != before {
                bend.option = u8::from(option).into();
            }
            ui.end_row();
        }
    });

    ui.add_space(12.);
    ui.heading("Sustain Pedal");
    ui.separator();

    let pedal = &mut ui_state.preset.sustain_pedal;
    Grid::new("sustain_grid").show(ui, |ui| {
        ui.label("Mode");
        enum_combo(ui, "pedal_mode", &mut pedal.mode);
        ui.end_row();
        ui.label("Channel");
        enum_combo(ui, "pedal_channel", &mut pedal.channel);
        ui.end_row();
        ui.label("CC / Note");
        value_drag(ui, &mut pedal.cc);
        ui.end_row();
        ui.label("Off Value");
        value_drag(ui, &mut pedal.off_value);
        ui.end_row();
        ui.label("On Value");
        value_drag(ui, &mut pedal.on_value);
        ui.end_row();
        if pedal.mode == SustainPedalMode::SwitchedControl || pedal.mode == SustainPedalMode::Note {
            ui.label("Option");
            behavior_combo(ui, "pedal_option", &mut pedal.option);
            ui.end_row();
        }
    });
}

fn render_global(ui: &mut Ui, ui_state: &mut UiState) {
    ui.heading("Global Settings");
    ui.separator();

    let global = &mut ui_state.global;
    Grid::new("global_grid").show(ui, |ui| {
        ui.label("Keyboard Channel");
        {
            let mut channel: MidiChannel = global.keyboard_channel;
            enum_combo(ui, "global_channel", &mut channel);
            global.keyboard_channel = channel;
        }
        ui.end_row();

        ui.label("Key Velocity Curve");
        {
            let mut curve: VelocityCurve = global.key_velocity_curve;
            enum_combo(ui, "global_key_curve", &mut curve);
            global.key_velocity_curve = curve;
        }
        ui.end_row();

        ui.label("Pad Velocity Curve");
        {
            let mut curve: VelocityCurve = global.pad_velocity_curve;
            enum_combo(ui, "global_pad_curve", &mut curve);
            global.pad_velocity_curve = curve;
        }
        ui.end_row();

        ui.label("Knob Acceleration");
        {
            let mut accel: KnobAcceleration = global.knob_acceleration;
            enum_combo(ui, "global_accel", &mut accel);
            global.knob_acceleration = accel;
        }
        ui.end_row();

        ui.label("Octave Button Blink");
        {
            let mut blink: ToggleState = global.octave_button_blink;
            enum_combo(ui, "global_blink", &mut blink);
            global.octave_button_blink = blink;
        }
        ui.end_row();

        ui.label("Pad Off Backlight");
        {
            let mut backlight: ToggleState = global.pad_off_backlight;
            enum_combo(ui, "global_backlight", &mut backlight);
            global.pad_off_backlight = backlight;
        }
        ui.end_row();
    });
}

fn render_memory(ui: &mut Ui, ui_state: &mut UiState, outbox: &mut Vec<UiEffect>) {
    ui.heading("Memory");
    ui.separator();

    ui.label("The MiniLab mkII has 8 memory slots. Slot 1 is reserved for Analog Lab.");
    ui.add_space(8.);

    Grid::new("memory_grid").show(ui, |ui| {
        ui.label("Slot");
        enum_combo(ui, "memory_slot", &mut ui_state.selected_memory_slot);
        ui.end_row();
    });

    ui.add_space(8.);
    ui.horizontal(|ui| {
        if ui
            .button("Recall")
            .on_hover_text("Load this memory into the device's working memory, then read it")
            .clicked()
        {
            outbox.push(UiEffect::RecallMemory(ui_state.selected_memory_slot));
        }

        let store_enabled = ui_state.selected_memory_slot != MemorySlot::Slot1;
        if ui
            .add_enabled(store_enabled, egui::Button::new("Store"))
            .on_hover_text("Store the device's working memory into this slot")
            .on_disabled_hover_text("Slot 1 is reserved for Analog Lab")
            .clicked()
        {
            outbox.push(UiEffect::StoreMemory(ui_state.selected_memory_slot));
        }
    });
}
