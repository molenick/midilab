use std::fmt::Display;

use eframe::egui;
use eframe::egui::Color32;
use eframe::egui::DragValue;
use eframe::egui::Grid;
use eframe::egui::RichText;
use eframe::egui::ScrollArea;
use eframe::egui::Ui;
use midilab::IntoEnumIterator;
use midilab::manufacturer::nektar::impact_lx_plus::Dump;
use midilab::manufacturer::nektar::impact_lx_plus::GlobalSettingId;
use midilab::manufacturer::nektar::impact_lx_plus::control::Button;
use midilab::manufacturer::nektar::impact_lx_plus::control::Continuous;
use midilab::manufacturer::nektar::impact_lx_plus::control::GlobalControlId;
use midilab::midi::Value;

use crate::message::UiEffect;
use crate::message::UserMsgKind;
use crate::state::EditorTab;
use crate::state::UiState;

pub fn ui(ui: &mut Ui, ui_state: &mut UiState, outbox: &mut Vec<UiEffect>) {
    egui::Panel::top("top_panel").show(ui, |ui| {
        egui::MenuBar::new().ui(ui, |ui| {
            ui.menu_button("File", |ui| {
                if ui.button("Save Dump").clicked() {
                    outbox.push(UiEffect::ShowDumpSaveDialog);
                }
                if ui.button("Load Dump").clicked() {
                    outbox.push(UiEffect::ShowDumpLoadDialog);
                }
                ui.separator();
                if ui.button("Quit").clicked() {
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                }
            });
            ui.menu_button("Edit", |ui| {
                if ui.button("Factory Dump").clicked() {
                    ui_state.dump = Dump::default();
                }
            });
        });
    });

    render_main_editor(ui, ui_state, outbox);
}

fn render_main_editor(ui: &mut Ui, ui_state: &mut UiState, outbox: &mut Vec<UiEffect>) {
    egui::Panel::left("device_panel")
        .default_size(190.)
        .min_size(160.)
        .max_size(240.)
        .show(ui, |ui| {
            ui.add_space(8.);
            ui.heading("Device");
            ui.separator();

            if ui
                .button("Write All")
                .on_hover_text("Write all presets, pad maps and globals to the device")
                .clicked()
            {
                outbox.push(UiEffect::WriteDump(Box::new(ui_state.dump)));
            }
            if ui.button("Reconnect").clicked() {
                outbox.push(UiEffect::Reconnect);
            }

            ui.separator();
            ui.label(
                "To read the device, press [Setup] then the Memory Dump key (G2) \
                 on the keyboard. The editor picks the dump up automatically.",
            );

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
            EditorTab::Presets => render_presets(ui, ui_state, outbox),
            EditorTab::Pads => render_pads(ui, ui_state, outbox),
            EditorTab::Controller => render_controller(ui, ui_state, outbox),
            EditorTab::Settings => render_settings(ui, ui_state, outbox),
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

fn continuous_row(ui: &mut Ui, label: String, index: usize, control: &mut Continuous) {
    ui.label(label);
    enum_combo(ui, ("continuous_channel", index), &mut control.channel);
    enum_combo(ui, ("continuous_kind", index), &mut control.kind);
    value_drag(ui, &mut control.cc);
    value_drag(ui, &mut control.min);
    value_drag(ui, &mut control.max);
    ui.end_row();
}

fn button_row(ui: &mut Ui, label: String, index: usize, button: &mut Button) {
    ui.label(label);
    enum_combo(ui, ("button_channel", index), &mut button.channel);
    enum_combo(ui, ("button_kind", index), &mut button.kind);
    value_drag(ui, &mut button.data1);
    value_drag(ui, &mut button.min);
    value_drag(ui, &mut button.max);
    ui.end_row();
}

fn continuous_header(ui: &mut Ui) {
    ui.label("");
    ui.label("Channel");
    ui.label("Type");
    ui.label("CC");
    ui.label("Min");
    ui.label("Max");
    ui.end_row();
}

fn button_header(ui: &mut Ui) {
    ui.label("");
    ui.label("Channel");
    ui.label("Type");
    ui.label("CC / Note");
    ui.label("Off");
    ui.label("On");
    ui.end_row();
}

fn render_presets(ui: &mut Ui, ui_state: &mut UiState, outbox: &mut Vec<UiEffect>) {
    ui.horizontal(|ui| {
        ui.heading("Preset");
        enum_combo(ui, "preset_selector", &mut ui_state.selected_preset);
        if ui
            .button("Write Preset")
            .on_hover_text(
                "Write this preset to the device's stored memory. It takes effect \
                 the next time the preset is loaded on the device.",
            )
            .clicked()
        {
            let id = ui_state.selected_preset;
            outbox.push(UiEffect::WritePreset {
                id,
                preset: Box::new(ui_state.dump.presets[id as usize - 1]),
            });
        }
    });
    ui.separator();

    let preset = &mut ui_state.dump.presets[ui_state.selected_preset as usize - 1];

    ui.heading("Faders");
    Grid::new("faders_grid").striped(true).show(ui, |ui| {
        continuous_header(ui);
        for (index, fader) in preset.faders.iter_mut().enumerate() {
            continuous_row(ui, format!("Fader {}", index + 1), index, fader);
        }
    });

    ui.add_space(12.);
    ui.heading("Pots");
    Grid::new("pots_grid").striped(true).show(ui, |ui| {
        continuous_header(ui);
        for (index, pot) in preset.pots.iter_mut().enumerate() {
            continuous_row(ui, format!("Pot {}", index + 1), index + 100, pot);
        }
    });

    ui.add_space(12.);
    ui.heading("Fader Buttons");
    Grid::new("fader_buttons_grid")
        .striped(true)
        .show(ui, |ui| {
            button_header(ui);
            for (index, button) in preset.fader_buttons.iter_mut().enumerate() {
                button_row(ui, format!("Fader Button {}", index + 1), index, button);
            }
        });
}

fn render_pads(ui: &mut Ui, ui_state: &mut UiState, outbox: &mut Vec<UiEffect>) {
    ui.horizontal(|ui| {
        ui.heading("Pad Map");
        enum_combo(ui, "pad_map_selector", &mut ui_state.selected_pad_map);
        if ui
            .button("Write Pad Map")
            .on_hover_text(
                "Write this pad map to the device's stored memory. It takes effect \
                 the next time the pad map is loaded on the device.",
            )
            .clicked()
        {
            let id = ui_state.selected_pad_map;
            outbox.push(UiEffect::WritePadMap {
                id,
                map: ui_state.dump.pad_maps[id as usize - 1],
            });
        }
    });
    ui.separator();
    ui.label("The physical bottom row is pads 1-4, the top row pads 5-8.");
    ui.add_space(8.);

    let map = &mut ui_state.dump.pad_maps[ui_state.selected_pad_map as usize - 1];

    Grid::new("pads_grid").striped(true).show(ui, |ui| {
        ui.label("");
        ui.label("Channel");
        ui.label("Type");
        ui.label("CC");
        ui.label("Off");
        ui.label("On");
        ui.label("Note");
        ui.end_row();

        for (index, pad) in map.pads.iter_mut().enumerate() {
            ui.label(format!("Pad {}", index + 1));
            enum_combo(ui, ("pad_channel", index), &mut pad.channel);
            enum_combo(ui, ("pad_kind", index), &mut pad.kind);
            value_drag(ui, &mut pad.data1);
            value_drag(ui, &mut pad.min);
            value_drag(ui, &mut pad.max);
            note_drag(ui, &mut pad.note);
            ui.end_row();
        }
    });
}

fn render_controller(ui: &mut Ui, ui_state: &mut UiState, outbox: &mut Vec<UiEffect>) {
    ui.horizontal(|ui| {
        ui.heading("Wheels & Transport");
        if ui
            .button("Write Wheels & Transport")
            .on_hover_text("Writes apply to the live device state instantly")
            .clicked()
        {
            outbox.push(UiEffect::WriteGlobalControls(ui_state.dump.controls));
        }
    });
    ui.separator();
    ui.label(
        "These controls are global: they are not part of presets and survive preset switches.",
    );
    ui.add_space(8.);

    let controls = &mut ui_state.dump.controls;

    ui.heading("Wheels");
    Grid::new("wheels_grid").striped(true).show(ui, |ui| {
        continuous_header(ui);
        continuous_row(
            ui,
            GlobalControlId::PitchWheel.to_string(),
            200,
            &mut controls.pitch_wheel,
        );
        continuous_row(
            ui,
            GlobalControlId::ModWheel.to_string(),
            201,
            &mut controls.mod_wheel,
        );
    });

    ui.add_space(12.);
    ui.heading("Foot Switch");
    Grid::new("foot_switch_grid").striped(true).show(ui, |ui| {
        button_header(ui);
        button_row(
            ui,
            GlobalControlId::FootSwitch.to_string(),
            210,
            &mut controls.foot_switch,
        );
    });

    ui.add_space(12.);
    ui.heading("Transport Buttons");
    Grid::new("transport_grid").striped(true).show(ui, |ui| {
        button_header(ui);
        for (index, button) in controls.transport.iter_mut().enumerate() {
            button_row(ui, format!("Transport {}", index + 1), index + 300, button);
        }
    });
}

fn render_settings(ui: &mut Ui, ui_state: &mut UiState, outbox: &mut Vec<UiEffect>) {
    ui.horizontal(|ui| {
        ui.heading("Global Settings");
        if ui
            .button("Write Settings")
            .on_hover_text("Writes apply to the live device state instantly")
            .clicked()
        {
            outbox.push(UiEffect::WriteGlobalSettings(ui_state.dump.settings));
        }
    });
    ui.separator();

    let settings = &mut ui_state.dump.settings;

    Grid::new("settings_grid").show(ui, |ui| {
        ui.label("Global MIDI Channel");
        enum_combo(ui, "settings_channel", &mut settings.midi_channel);
        ui.end_row();
    });

    ui.add_space(12.);
    ui.heading("Unmapped Settings");
    ui.label(
        "The meanings of these settings are not reverse engineered yet; \
         they round-trip as raw values.",
    );
    ui.add_space(8.);

    Grid::new("unknown_settings_grid")
        .striped(true)
        .show(ui, |ui| {
            for (setting, value) in GlobalSettingId::iter()
                .skip(1)
                .zip(settings.unknown.iter_mut())
            {
                ui.label(format!("Setting {:#04X}", u8::from(setting)));
                value_drag(ui, value);
                ui.end_row();
            }
        });
}
