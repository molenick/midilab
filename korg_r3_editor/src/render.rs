use eframe::egui;
use eframe::egui::CentralPanel;
use eframe::egui::Context;
use eframe::egui::Frame;
use eframe::egui::Grid;
use eframe::egui::Margin;
use eframe::egui::ScrollArea;
use eframe::egui::Ui;
use eframe::egui::containers::Modal;
use midilab::manufacturer::korg::r3::live;
use midilab::manufacturer::korg::r3::wrappers::*;

use crate::message::UiEffect;
use crate::state::EditorTab;
use crate::state::TimbreSelect;
use crate::state::UiState;

mod palette {
    use eframe::egui::Color32;
    pub const PANEL: Color32 = Color32::from_rgb(26, 26, 30);
    pub const HEADER: Color32 = Color32::from_rgb(16, 16, 20);
    pub const ACCENT: Color32 = Color32::from_rgb(96, 165, 224);
}

fn sp(n: i32) -> f32 {
    8.0 * std::f32::consts::GOLDEN_RATIO.powi(n)
}

pub fn ui(ui: &mut Ui, ui_state: &mut UiState, outbox: &mut Vec<UiEffect>) {
    egui::Panel::top("top_panel").show(ui, |ui| {
        egui::MenuBar::new().ui(ui, |ui| {
            ui.menu_button("File", |ui| {
                if ui.button("Dump Program").clicked() {
                    outbox.push(UiEffect::DumpCurrentProgram);
                }
                if ui.button("Save Program").clicked() {
                    outbox.push(UiEffect::ShowProgramSaveDialog);
                }
                if ui.button("Load Program").clicked() {
                    outbox.push(UiEffect::ShowProgramLoadDialog);
                }
                ui.separator();
                if ui.button("Dump Global").clicked() {
                    outbox.push(UiEffect::RequestGlobalFromDevice);
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
                if ui.button("Blank Program").clicked() {
                    ui_state.program = Program::blank();
                }
                if ui.button("Blank Global").clicked() {
                    ui_state.global = Global::default();
                }
            });

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let dot = if ui_state.live_edit {
                    palette::ACCENT
                } else {
                    egui::Color32::DARK_GRAY
                };
                let (rect, _) = ui.allocate_exact_size(egui::vec2(10., 10.), egui::Sense::hover());
                ui.painter()
                    .circle(rect.center(), 5., dot, egui::Stroke::NONE);
                ui.checkbox(&mut ui_state.live_edit, "Live")
                    .on_hover_text("Stream program edits to the R3 edit buffer in real time");
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
                "AutoSync: Sync Program & Global from device on app start",
            )
            .changed()
        {
            ui_state.user_settings.auto_sync_enabled = checked;
            let config = crate::config::AppConfig {
                persistence_path: ui_state.configured_directory.clone(),
                user: ui_state.user_settings.clone(),
            };
            let config_path =
                crate::config::AppConfig::config_path().expect("Failed to get config path");
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
    let before = ui_state.live_edit.then(|| ui_state.program.clone());

    egui::Panel::left("device_panel")
        .default_size(170.)
        .min_size(140.)
        .max_size(220.)
        .show(ui, |ui| {
            ui.add_space(sp(0));
            ui.heading("Device");
            ui.separator();

            if ui.button("Dump Program").clicked() {
                outbox.push(UiEffect::DumpCurrentProgram);
            }
            if ui.button("Write to Device").clicked() {
                outbox.push(UiEffect::WriteSelectedProgram);
            }
            ui.separator();
            if ui.button("Dump Global").clicked() {
                outbox.push(UiEffect::RequestGlobalFromDevice);
            }
            if ui.button("Write Global").clicked() {
                outbox.push(UiEffect::SendGlobalToDevice(ui_state.global.clone()));
            }
            ui.separator();
            if ui.button("Save Program").clicked() {
                outbox.push(UiEffect::ShowProgramSaveDialog);
            }
            if ui.button("Load Program").clicked() {
                outbox.push(UiEffect::ShowProgramLoadDialog);
            }
            ui.separator();
            if ui.button("Save Global").clicked() {
                outbox.push(UiEffect::ShowGlobalSaveDialog);
            }
            if ui.button("Load Global").clicked() {
                outbox.push(UiEffect::ShowGlobalLoadDialog);
            }
            ui.separator();

            let dot_color = match &ui_state.user_msg {
                Some(s) => match s.kind {
                    crate::message::UserMsgKind::Status => egui::Color32::GREEN,
                    crate::message::UserMsgKind::Error => egui::Color32::RED,
                },
                None => egui::Color32::DARK_GRAY,
            };
            ui.horizontal(|ui| {
                let (rect, _) = ui.allocate_exact_size(egui::vec2(10., 10.), egui::Sense::hover());
                ui.painter()
                    .circle(rect.center(), 5., dot_color, egui::Stroke::NONE);
                if let Some(status) = &ui_state.user_msg {
                    ui.colored_label(dot_color, &status.msg);
                } else {
                    ui.label("Connected");
                }
            });
        });

    CentralPanel::default().show(ui, |ui| {
        ui.horizontal(|ui| {
            for tab in EditorTab::ALL {
                let selected = ui_state.editor_tab == tab;
                let mut text = egui::RichText::new(format!("  {}  ", tab.label()));
                if selected {
                    text = text.color(palette::ACCENT).strong();
                }
                if ui.selectable_label(selected, text).clicked() {
                    ui_state.editor_tab = tab;
                }
            }
        });
        ui.separator();

        if !matches!(ui_state.editor_tab, EditorTab::Global | EditorTab::Formant) {
            program_header(ui, ui_state, outbox);
            if matches!(ui_state.editor_tab, EditorTab::Synth | EditorTab::Fx) {
                timbre_selector(ui, &mut ui_state.selected_timbre);
            }
            ui.add_space(sp(-1));
        }

        ScrollArea::both().show(ui, |ui| match ui_state.editor_tab {
            EditorTab::Program => render_program_page(ui, &mut ui_state.program),
            EditorTab::Synth => render_synth_page(
                ui,
                active_timbre(&mut ui_state.program, ui_state.selected_timbre),
            ),
            EditorTab::Vocoder => render_vocoder_page(ui, &mut ui_state.program.vocoder),
            EditorTab::Fx => render_fx_page(ui, &mut ui_state.program, ui_state.selected_timbre),
            EditorTab::Arp => render_arp_page(ui, &mut ui_state.program.arp),
            EditorTab::Global => render_global_editor(ui, &mut ui_state.global, outbox),
            EditorTab::Formant => render_formant_page(ui, ui_state, outbox),
        });
    });

    if let Some(before) = before
        && !live::program_diff(&before, &ui_state.program).is_empty()
    {
        outbox.push(UiEffect::LiveEdit(Box::new(ui_state.program.clone())));
    }
}

fn active_timbre(program: &mut Program, sel: TimbreSelect) -> &mut Timbre {
    match sel {
        TimbreSelect::One => &mut program.timbre1,
        TimbreSelect::Two => &mut program.timbre2,
    }
}

fn program_header(ui: &mut Ui, ui_state: &mut UiState, outbox: &mut Vec<UiEffect>) {
    let program = &mut ui_state.program;
    let selected_slot = &mut ui_state.selected_slot;

    Frame::group(ui.style())
        .fill(palette::HEADER)
        .inner_margin(Margin::same(sp(0) as i8))
        .show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label("Name");
                let mut ne = program.name.clone();
                if ui
                    .add(egui::TextEdit::singleline(&mut ne).desired_width(sp(7)))
                    .changed()
                {
                    program.name = ne;
                }
                ui.separator();

                ui.label("Category");
                let mut cat = program.category.get() as i32;
                if ui
                    .add(egui::DragValue::new(&mut cat).range(0..=15))
                    .changed()
                {
                    program.category.set(cat as u8);
                }
                ui.separator();

                ui.label("Voice");
                compact_enum(ui, "hdr_vmode", &mut program.voice_mode);
                ui.separator();

                ui.label("Tempo");
                let mut bpm = program.tempo.bpm() as i32;
                if ui
                    .add(
                        egui::DragValue::new(&mut bpm)
                            .range(20..=300)
                            .suffix(" BPM"),
                    )
                    .changed()
                {
                    program.tempo = Tempo::new(bpm as u16);
                }
            });

            ui.horizontal_wrapped(|ui| {
                ui.label("Slot");
                let mut slot_val = selected_slot.as_u16() as i32;
                if ui
                    .add(egui::DragValue::new(&mut slot_val).range(0..=ProgramSlot::MAX as i32))
                    .changed()
                {
                    *selected_slot = ProgramSlot::new(slot_val as u16);
                }
                ui.label(format!(
                    "({} · {})",
                    selected_slot,
                    if selected_slot.is_vocoder() {
                        "vocoder"
                    } else {
                        "synth"
                    },
                ));
                ui.separator();

                if ui.button(format!("Dump {}", *selected_slot)).clicked() {
                    outbox.push(UiEffect::DumpSlot(*selected_slot));
                }
                if ui.button("Dump Edit Buffer").clicked() {
                    outbox.push(UiEffect::DumpCurrentProgram);
                }

                let sel = *selected_slot;
                let label = format!("Write to {}", sel);
                if ui
                    .add(egui::Button::new(
                        egui::RichText::new(label).color(palette::ACCENT),
                    ))
                    .clicked()
                {
                    outbox.push(UiEffect::WriteProgram {
                        program: Box::new(program.clone()),
                        slot: sel.as_u16() as u8,
                    });
                }
            });
        });
}

fn timbre_selector(ui: &mut Ui, sel: &mut TimbreSelect) {
    ui.horizontal(|ui| {
        ui.label("Editing:");
        ui.selectable_value(sel, TimbreSelect::One, "Timbre 1");
        ui.selectable_value(sel, TimbreSelect::Two, "Timbre 2");
    });
}

fn render_program_page(ui: &mut Ui, program: &mut Program) {
    ui.horizontal_top(|ui| {
        section(ui, "Common", |ui| {
            Grid::new("common").striped(true).show(ui, |ui| {
                row_enum(ui, "vm_mode", "Voice Mode", &mut program.voice_mode);
                row_enum(ui, "arp_timb", "Arp Timbre", &mut program.arp_timbre);
                row_enum(
                    ui,
                    "t2_midi",
                    "Timbre 2 MIDI Ch",
                    &mut program.timbre2_midi_ch,
                );
                row_field(ui, "category", "Category", &mut program.category);
                row_u8(
                    ui,
                    "center_key",
                    "Center Key",
                    &mut program.center_key,
                    0..=127,
                );
                row_i8(ui, "octave_sw", "Octave", &mut program.octave_sw, -3..=3);
                row_field(ui, "tempo", "Tempo (BPM)", &mut program.tempo);
            });
        });

        section(ui, "Vocoder Knob Assigns", |ui| {
            Grid::new("vcd_knobs").striped(true).show(ui, |ui| {
                for (i, k) in program.vcd_knob_assigns.iter_mut().enumerate() {
                    row_field(ui, &format!("vcd_knob{i}"), &format!("Knob {}", i + 1), k);
                }
            });
        });
    });
}

fn render_synth_page(ui: &mut Ui, t: &mut Timbre) {
    ui.horizontal_top(|ui| {
        section(ui, "Pitch & Voice", |ui| {
            Grid::new("t_pitch").striped(true).show(ui, |ui| {
                row_enum(ui, "uvoice", "Unison Voice", &mut t.unison_voice);
                row_field(ui, "udet", "Unison Detune", &mut t.unison_detune);
                row_field(ui, "uspread", "Unison Spread", &mut t.unison_spread);
                row_enum(ui, "vassign", "Voice Assign", &mut t.voice_assign);
                row_field(ui, "atune", "Analog Tuning", &mut t.analog_tuning);
                row_field(ui, "transpose", "Transpose", &mut t.transpose);
                row_field(ui, "detune", "Detune", &mut t.detune);
                row_field(ui, "vib", "Vibrato Int", &mut t.vibrato_int);
                row_field(ui, "bend", "Bend Range", &mut t.bend_range);
                row_field(ui, "porta", "Portamento", &mut t.portamento);
            });
        });
        section(ui, "OSC 1", |ui| {
            Grid::new("t_osc1").striped(true).show(ui, |ui| {
                row_enum(ui, "o1wave", "Wave", &mut t.osc1.wave);
                row_enum(ui, "o1mod", "OSC Mod", &mut t.osc1.osc_mod);
                row_field(ui, "o1c1", "Ctrl 1", &mut t.osc1.ctrl1);
                row_field(ui, "o1c2", "Ctrl 2", &mut t.osc1.ctrl2);
                row_field(ui, "o1dwgs", "DWGS", &mut t.osc1.dwgs);
            });
        });
        section(ui, "OSC 2", |ui| {
            Grid::new("t_osc2").striped(true).show(ui, |ui| {
                row_enum(ui, "o2wave", "Wave", &mut t.osc2.wave);
                row_enum(ui, "o2mod", "OSC Mod", &mut t.osc2.osc_mod);
                row_field(ui, "o2semi", "Semitone", &mut t.osc2.semitone);
                row_field(ui, "o2tune", "Tune", &mut t.osc2.tune);
            });
        });
        section(ui, "Mixer", |ui| {
            Grid::new("t_mix").striped(true).show(ui, |ui| {
                row_field(ui, "mo1", "OSC 1 Level", &mut t.mixer.osc1_level);
                row_field(ui, "mo2", "OSC 2 Level", &mut t.mixer.osc2_level);
                row_field(ui, "mn", "Noise Level", &mut t.mixer.noise_level);
            });
        });
    });

    ui.horizontal_top(|ui| {
        section(ui, "Filter", |ui| {
            let f = &mut t.filter;
            Grid::new("t_filt").striped(true).show(ui, |ui| {
                row_enum(ui, "frout", "Routing", &mut f.routing);
                row_enum(ui, "f2type", "Filter 2 Type", &mut f.filter2_type);
                row_field(ui, "fbal", "Filter 1 Balance", &mut f.balance);
                row_field(ui, "f1cut", "Cutoff 1", &mut f.cutoff1);
                row_field(ui, "f1res", "Resonance 1", &mut f.resonance1);
                row_field(ui, "f1eg", "EG1 Int 1", &mut f.eg1_int1);
                row_field(ui, "f1kt", "Key Track 1", &mut f.key_track1);
                row_field(ui, "f1vs", "Velo Sens 1", &mut f.velo_sens1);
                row_field(ui, "f2cut", "Cutoff 2", &mut f.cutoff2);
                row_field(ui, "f2res", "Resonance 2", &mut f.resonance2);
                row_field(ui, "f2eg", "EG1 Int 2", &mut f.eg1_int2);
                row_field(ui, "f2kt", "Key Track 2", &mut f.key_track2);
                row_field(ui, "f2vs", "Velo Sens 2", &mut f.velo_sens2);
            });
        });
        section(ui, "Amp / Drive", |ui| {
            let a = &mut t.amp;
            Grid::new("t_amp").striped(true).show(ui, |ui| {
                row_field(ui, "alvl", "Level", &mut a.level);
                row_enum(ui, "wspos", "WaveShape Position", &mut a.ws_position);
                row_field(ui, "wstype", "WaveShape Type", &mut a.ws_type);
                row_field(ui, "wsdep", "WaveShape Depth", &mut a.ws_depth);
                row_field(ui, "apan", "Pan", &mut a.pan);
                row_field(ui, "akt", "Key Track", &mut a.key_track);
                row_field(ui, "apunch", "Punch Level", &mut a.punch_level);
            });
        });
    });

    ui.horizontal_top(|ui| {
        for (i, eg) in t.eg.iter_mut().enumerate() {
            section(ui, &format!("EG {}", i + 1), |ui| {
                Grid::new(format!("t_eg{i}")).striped(true).show(ui, |ui| {
                    row_field(ui, &format!("eg{i}a"), "Attack", &mut eg.attack);
                    row_field(ui, &format!("eg{i}d"), "Decay", &mut eg.decay);
                    row_field(ui, &format!("eg{i}s"), "Sustain", &mut eg.sustain);
                    row_field(ui, &format!("eg{i}r"), "Release", &mut eg.release);
                    row_field(ui, &format!("eg{i}lv"), "Level Velo", &mut eg.level_velo);
                });
            });
        }
    });

    ui.horizontal_top(|ui| {
        for (i, lfo) in t.lfo.iter_mut().enumerate() {
            section(ui, &format!("LFO {}", i + 1), |ui| {
                Grid::new(format!("t_lfo{i}")).striped(true).show(ui, |ui| {
                    row_u8(ui, &format!("lfo{i}w"), "Wave", &mut lfo.wave, 0..=4);
                    row_field(ui, &format!("lfo{i}f"), "Freq", &mut lfo.freq);
                    row_bool(ui, &format!("lfo{i}bpm"), "BPM Sync", &mut lfo.bpm_sync);
                    row_enum(ui, &format!("lfo{i}ks"), "Key Sync", &mut lfo.key_sync);
                    row_field(ui, &format!("lfo{i}sn"), "Sync Note", &mut lfo.sync_note);
                });
            });
        }
        section(ui, "Virtual Patch", |ui| {
            Grid::new("t_patch").striped(true).show(ui, |ui| {
                for (i, p) in t.patches.iter_mut().enumerate() {
                    ui.label(format!("Patch {}", i + 1));
                    patch_combo(ui, &format!("p{i}src"), &mut p.src);
                    patch_combo(ui, &format!("p{i}dst"), &mut p.dst);
                    let mut v = p.int.get() as i32;
                    ui.push_id(format!("p{i}int"), |ui| {
                        if ui
                            .add(egui::DragValue::new(&mut v).range(-63..=63))
                            .changed()
                        {
                            p.int.set(v as i16);
                        }
                    });
                    ui.end_row();
                }
            });
        });
    });

    ui.horizontal_top(|ui| {
        section(ui, "Motion Sequence", |ui| {
            let ms = &mut t.motion_seq;
            Grid::new("t_ms").striped(true).show(ui, |ui| {
                row_bool(ui, "mson", "On", &mut ms.on);
                row_field(ui, "mstype", "Type", &mut ms.seq_type);
                row_u8(ui, "mslast", "Last Step", &mut ms.last_step, 0..=15);
                row_enum(ui, "msks", "Key Sync", &mut ms.key_sync);
                row_field(ui, "msres", "Resolution", &mut ms.resolution);
            });
        });
    });
}

fn render_vocoder_page(ui: &mut Ui, v: &mut Vocoder) {
    ui.horizontal_top(|ui| {
        section(ui, "Switches", |ui| {
            Grid::new("voc_sw").striped(true).show(ui, |ui| {
                row_bool(ui, "voc_on", "On", &mut v.on);
                row_bool(
                    ui,
                    "voc_src",
                    "Source = Formant Rec",
                    &mut v.source_formant_rec,
                );
                row_bool(ui, "voc_hpfg", "HPF Gate", &mut v.hpf_gate);
                row_bool(
                    ui,
                    "voc_ftr",
                    "Formant Trig Reset",
                    &mut v.formant_trig_reset,
                );
                row_bool(ui, "voc_sel", "Select Timbre 2", &mut v.select_timbre2);
            });
        });
        section(ui, "Levels", |ui| {
            Grid::new("voc_lvl").striped(true).show(ui, |ui| {
                row_field(ui, "voc_gate", "Gate Sens", &mut v.gate_sens);
                row_field(ui, "voc_thr", "Threshold", &mut v.threshold);
                row_field(ui, "voc_hpf", "HPF Level", &mut v.hpf_level);
                row_field(ui, "voc_dir", "Direct Level", &mut v.direct_level);
                row_field(ui, "voc_t1", "Timbre 1 Level", &mut v.timbre1_level);
                row_field(ui, "voc_in1", "Input 1 Level", &mut v.input1_level);
                row_field(ui, "voc_vlvl", "Vocoder Level", &mut v.vocoder_level);
            });
        });
        section(ui, "Filter", |ui| {
            Grid::new("voc_filt").striped(true).show(ui, |ui| {
                row_enum(ui, "voc_fcsrc", "Fc Mod Source", &mut v.fc_mod_src);
                row_field(ui, "voc_coff", "Cutoff Offset", &mut v.cutoff_offset);
                row_field(ui, "voc_res", "Resonance", &mut v.resonance);
                row_field(ui, "voc_fcint", "Fc Mod Int", &mut v.fc_mod_int);
                row_field(ui, "voc_ef", "E.F. Sens", &mut v.ef_sens);
            });
        });
    });

    ui.horizontal_top(|ui| {
        section(ui, "Band Levels & Pans", |ui| {
            Grid::new("voc_bands").striped(true).show(ui, |ui| {
                for i in 0..16 {
                    ui.label(format!("Band {}", i + 1));
                    let mut lvl = v.band_levels[i].get() as i32;
                    ui.push_id(format!("voc_bl{i}"), |ui| {
                        if ui
                            .add(egui::DragValue::new(&mut lvl).range(0..=127).prefix("L "))
                            .changed()
                        {
                            v.band_levels[i].set(lvl as u8);
                        }
                    });
                    let mut pan = v.band_pans[i].get() as i32;
                    ui.push_id(format!("voc_bp{i}"), |ui| {
                        if ui
                            .add(egui::DragValue::new(&mut pan).range(-63..=63).prefix("P "))
                            .changed()
                        {
                            v.band_pans[i] = Pan::new(pan as i16);
                        }
                    });
                    ui.end_row();
                }
            });
        });
    });
}

fn render_fx_page(ui: &mut Ui, program: &mut Program, sel: TimbreSelect) {
    let ifx = &mut active_timbre(program, sel).insert_fx;
    ui.horizontal_top(|ui| {
        section(ui, "Insert FX 1", |ui| {
            Grid::new("ifx1").striped(true).show(ui, |ui| {
                row_enum(ui, "ifx1t", "Type", &mut ifx.fx1_type);
                row_u8(ui, "ifx1k", "Knob Assign", &mut ifx.fx1_knob, 0..=19);
                for (i, p) in ifx.fx1_params.iter_mut().enumerate() {
                    row_field(ui, &format!("ifx1p{i}"), &format!("Param {}", i + 1), p);
                }
            });
        });
        section(ui, "Insert FX 2", |ui| {
            Grid::new("ifx2").striped(true).show(ui, |ui| {
                row_enum(ui, "ifx2t", "Type", &mut ifx.fx2_type);
                row_u8(ui, "ifx2k", "Knob Assign", &mut ifx.fx2_knob, 0..=19);
                for (i, p) in ifx.fx2_params.iter_mut().enumerate() {
                    row_field(ui, &format!("ifx2p{i}"), &format!("Param {}", i + 1), p);
                }
            });
        });
        section(ui, "Timbre EQ", |ui| {
            Grid::new("ifx_eq").striped(true).show(ui, |ui| {
                row_u8(ui, "eqlf", "Low Freq", &mut ifx.eq_low_freq, 0..=33);
                row_field(ui, "eqlg", "Low Gain", &mut ifx.eq_low_gain);
                row_u8(ui, "eqhf", "Hi Freq", &mut ifx.eq_hi_freq, 0..=25);
                row_field(ui, "eqhg", "Hi Gain", &mut ifx.eq_hi_gain);
            });
        });
    });

    ui.horizontal_top(|ui| {
        section(ui, "Master FX", |ui| {
            let mfx = &mut program.master_fx;
            Grid::new("mfx").striped(true).show(ui, |ui| {
                row_enum(ui, "mfx_type", "Type", &mut mfx.fx_type);
                row_u8(ui, "mfx_knob", "Knob Assign", &mut mfx.knob_assign, 0..=19);
                for (i, p) in mfx.params.iter_mut().enumerate() {
                    row_field(ui, &format!("mfx_p{i}"), &format!("Param {}", i + 1), p);
                }
            });
        });
    });
}

fn render_arp_page(ui: &mut Ui, arp: &mut Arpeggio) {
    ui.horizontal_top(|ui| {
        section(ui, "Arpeggiator", |ui| {
            Grid::new("arp").striped(true).show(ui, |ui| {
                row_bool(ui, "arp_on", "On", &mut arp.on);
                row_bool(ui, "arp_latch", "Latch", &mut arp.latch);
                row_bool(ui, "arp_ksync", "Key Sync", &mut arp.key_sync);
                row_field(ui, "arp_res", "Resolution", &mut arp.resolution);
                row_enum(ui, "arp_type", "Type", &mut arp.arp_type);
                row_u8(ui, "arp_oct", "Octave Range", &mut arp.octave_range, 0..=3);
                row_u8(ui, "arp_last", "Last Step", &mut arp.last_step, 0..=7);
                row_field(ui, "arp_gate", "Gate Time", &mut arp.gate_time);
                row_field(ui, "arp_swing", "Swing", &mut arp.swing);
            });
        });
        section(ui, "Step Switches", |ui| {
            Grid::new("arp_steps").striped(true).show(ui, |ui| {
                for i in 0..8u8 {
                    let mask = 1u8 << (7 - i);
                    let mut on = arp.step_switches & mask != 0;
                    ui.label(format!("Step {}", i + 1));
                    ui.push_id(format!("arp_step{i}"), |ui| {
                        if ui.checkbox(&mut on, "").changed() {
                            if on {
                                arp.step_switches |= mask;
                            } else {
                                arp.step_switches &= !mask;
                            }
                        }
                    });
                    ui.end_row();
                }
            });
        });
    });
}

fn render_global_editor(ui: &mut Ui, global: &mut Global, outbox: &mut Vec<UiEffect>) {
    ui.horizontal_top(|ui| {
        section(ui, "Global Settings", |ui| {
            Grid::new("g1").striped(true).show(ui, |ui| {
                row_field(ui, "g_mastune", "Master Tune", &mut global.master_tune);
                row_field(ui, "g_transpose", "Transpose", &mut global.transpose);
                row_enum(
                    ui,
                    "g_velcurve",
                    "Velocity Curve",
                    &mut global.velocity_curve,
                );
                row_enum(
                    ui,
                    "g_midi",
                    "Global MIDI Channel",
                    &mut global.midi_channel,
                );
                row_bool(
                    ui,
                    "g_memprot",
                    "Memory Protect",
                    &mut global.memory_protect,
                );
                row_bool(ui, "g_local", "Local Control", &mut global.local_ctrl);
            });
        });
        section(ui, "MIDI Control Numbers", |ui| {
            Grid::new("g_midictrl").striped(true).show(ui, |ui| {
                for (i, c) in global.midi_ctrl.iter_mut().enumerate() {
                    row_field(ui, &format!("g_mc{i}"), &format!("MIDI {}", i + 1), c);
                }
            });
        });
        section(ui, "CC Mapping", |ui| {
            if global.cc_map.is_empty() {
                ui.label("No CC mappings configured.");
            } else {
                Grid::new("cc_map_grid").striped(true).show(ui, |ui| {
                    for (idx, mapped) in &global.cc_map {
                        ui.label(format!("Knob/SW {}", idx));
                        ui.label(format!("CC {}", mapped));
                        ui.end_row();
                    }
                });
            }
        });
    });

    ui.add_space(sp(0));
    ui.horizontal(|ui| {
        if ui
            .add(egui::Button::new(
                egui::RichText::new("Write Global to Device").color(palette::ACCENT),
            ))
            .clicked()
        {
            outbox.push(UiEffect::SendGlobalToDevice(global.clone()));
        }
        if ui.button("Save Global to File").clicked() {
            outbox.push(UiEffect::ShowGlobalSaveDialog);
        }
        if ui.button("Load Global from File").clicked() {
            outbox.push(UiEffect::ShowGlobalLoadDialog);
        }
    });
}

fn render_formant_page(ui: &mut Ui, state: &mut UiState, outbox: &mut Vec<UiEffect>) {
    let selected = state.selected_formant_no;
    ui.horizontal(|ui| {
        if ui
            .add(egui::Button::new(
                egui::RichText::new("Dump Current").color(palette::ACCENT),
            ))
            .clicked()
        {
            outbox.push(UiEffect::DumpCurrentFormantMotion);
        }
        ui.separator();
        let mut n = selected as i32 + 1;
        if ui
            .add(egui::DragValue::new(&mut n).range(1..=16).prefix("Motion "))
            .changed()
        {
            state.selected_formant_no = (n - 1) as u8;
        }
        if ui.button(format!("Dump Motion {}", selected + 1)).clicked() {
            outbox.push(UiEffect::DumpFormantMotion(selected));
        }
        ui.separator();
        if ui.button("Dump All 16").clicked() {
            for i in 0u8..16 {
                outbox.push(UiEffect::DumpFormantMotion(i));
            }
        }
        ui.separator();
        if ui.button("Save to File").clicked() {
            outbox.push(UiEffect::ShowFormantMotionSaveDialog);
        }
        if ui.button("Load from File").clicked() {
            outbox.push(UiEffect::ShowFormantMotionLoadDialog);
        }
    });
    ui.separator();

    ui.horizontal_top(|ui| {
        section(ui, "Formant Motion", |ui| {
            Grid::new("fm_list").striped(true).show(ui, |ui| {
                for i in 0u8..16 {
                    let cached = state.formant_motions[i as usize].as_ref();
                    let label = format!("Formant Motion {:02}", i + 1);
                    let text = if cached.is_some() {
                        egui::RichText::new(label).color(palette::ACCENT).strong()
                    } else {
                        egui::RichText::new(label)
                    };
                    let frames_text = cached
                        .map(|m| format!("{} frames · {:.2}s", m.steps.len(), m.duration_secs()))
                        .unwrap_or_default();
                    if ui.selectable_label(selected == i, text).clicked() {
                        state.selected_formant_no = i;
                        outbox.push(UiEffect::DumpFormantMotion(i));
                    }
                    ui.label(frames_text);
                    ui.end_row();
                }
            });
        });

        section(ui, "Formant Motion Edit", |ui| {
            formant_edit_section(ui, state, outbox);
        });
    });
}

fn formant_edit_section(ui: &mut Ui, state: &mut UiState, outbox: &mut Vec<UiEffect>) {
    let selected = state.selected_formant_no;

    ui.horizontal(|ui| {
        ui.checkbox(&mut state.formant_edit, "Edit")
            .on_hover_text("Draw band levels and author frames before writing");
        ui.separator();
        let mut count = state.formant_frame_count.clamp(1, FORMANT_MAX_FRAMES) as i32;
        if ui
            .add(egui::DragValue::new(&mut count).range(1..=FORMANT_MAX_FRAMES as i32).suffix(" frames"))
            .on_hover_text(format!("max {} frames = {:.1}s", FORMANT_MAX_FRAMES, FORMANT_MAX_FRAMES as f32 / 100.0))
            .changed()
        {
            state.formant_frame_count = count as usize;
        }
        ui.weak(format!("{:.2}s", count as f32 / 100.0));
        if ui
            .button("Resize")
            .on_hover_text("Change the current motion's length to the frame count, keeping drawn levels (extra frames truncated, new frames blank)")
            .clicked()
            && let Some(m) = state.formant_motion.as_mut() {
                m.resize(state.formant_frame_count);
                state.formant_selected_frame =
                    state.formant_selected_frame.min(m.steps.len().saturating_sub(1));
            }
        ui.separator();
        if ui
            .button("Clear")
            .on_hover_text("Zero all band levels (keeps the current frame count); if no motion is loaded, creates a blank one of the frame count")
            .clicked()
        {
            match state.formant_motion.as_mut() {
                Some(m) => m.clear_levels(),
                None => {
                    state.formant_motion =
                        Some(FormantMotion::blank(Some(selected), state.formant_frame_count));
                    state.formant_selected_frame = 0;
                }
            }
        }
    });

    let has_motion = matches!(&state.formant_motion, Some(m) if !m.is_empty());
    if !has_motion {
        ui.add_space(sp(2));
        ui.weak("No formant motion loaded");
        ui.add_space(sp(2));
        return;
    }

    let (frames, duration) = {
        let m = state.formant_motion.as_ref().unwrap();
        (m.steps.len(), m.duration_secs())
    };

    ui.horizontal(|ui| {
        ui.label(format!("{} frames  ·  {:.2}s", frames, duration));
        ui.separator();
        if ui
            .button(format!("Write to Motion {}", selected + 1))
            .clicked()
            && let Some(m) = state.formant_motion.clone()
        {
            outbox.push(UiEffect::WriteFormantMotion {
                motion: m,
                motion_no: selected,
            });
        }
    });

    if state.formant_edit {
        ui.horizontal(|ui| {
            ui.label("Band:");
            let mut band = (state.formant_selected_band as usize).min(FORMANT_BANDS - 1) as i32 + 1;
            if ui
                .add(
                    egui::Slider::new(&mut band, 1..=FORMANT_BANDS as i32)
                        .suffix(format!(" / {}", FORMANT_BANDS)),
                )
                .on_hover_text("Select the band whose envelope you draw on the editor")
                .changed()
            {
                state.formant_selected_band = (band - 1) as u8;
            }
        });
        ui.horizontal(|ui| {
            ui.label("Frame:");
            let mut frame = state.formant_selected_frame.min(frames - 1) as i32 + 1;
            if ui
                .add(
                    egui::Slider::new(&mut frame, 1..=frames as i32)
                        .suffix(format!(" / {}", frames)),
                )
                .on_hover_text("Select the frame edited by the band sliders below")
                .changed()
            {
                state.formant_selected_frame = (frame - 1) as usize;
            }
        });
        formant_envelope_edit(ui, state);
        formant_frame_sliders(ui, state);
    } else if let Some(m) = state.formant_motion.as_ref() {
        formant_waterfall(ui, m);
    }
}

fn formant_envelope_edit(ui: &mut Ui, state: &mut UiState) {
    let desired = egui::vec2(ui.available_width().max(sp(8)), sp(7).max(360.0));
    let (response, painter) = ui.allocate_painter(desired, egui::Sense::click_and_drag());
    let rect = response.rect;
    painter.rect_filled(rect, 2.0, palette::HEADER);

    let Some(motion) = state.formant_motion.as_mut() else {
        return;
    };
    let frames = motion.steps.len();
    if frames == 0 {
        return;
    }
    let sel_band = (state.formant_selected_band as usize).min(FORMANT_BANDS - 1);
    let geom = FormantGeom::new(rect, FORMANT_BANDS, frames);

    if let Some(pos) = response.interact_pointer_pos()
        && (response.clicked() || response.dragged())
    {
        let (frame, level) = geom.pick(sel_band, pos);
        motion.set_band(frame, sel_band, level);
        state.formant_selected_frame = frame.min(frames - 1);
    }

    for band in (0..FORMANT_BANDS).rev() {
        if band == sel_band {
            continue;
        }
        draw_band(
            &painter,
            &geom,
            motion,
            band,
            egui::Stroke::new(1.0, band_color(band, FORMANT_BANDS).gamma_multiply(0.4)),
        );
    }

    draw_band(
        &painter,
        &geom,
        motion,
        sel_band,
        egui::Stroke::new(2.0, palette::ACCENT),
    );
    for f in 0..frames {
        painter.circle_filled(
            geom.point(sel_band, f, motion.band_at(f, sel_band)),
            2.5,
            palette::ACCENT,
        );
    }
    let sf = state.formant_selected_frame.min(frames - 1);
    painter.circle_stroke(
        geom.point(sel_band, sf, motion.band_at(sf, sel_band)),
        5.0,
        egui::Stroke::new(1.5, egui::Color32::WHITE),
    );
}

fn formant_frame_sliders(ui: &mut Ui, state: &mut UiState) {
    let Some(motion) = state.formant_motion.as_mut() else {
        return;
    };
    let frames = motion.steps.len();
    if frames == 0 {
        return;
    }
    let frame = state.formant_selected_frame.min(frames - 1);
    ui.add_space(sp(-2));
    ui.strong(format!("Frame {} of {}", frame + 1, frames));
    Grid::new("fm_frame_sliders").show(ui, |ui| {
        for band in 0..FORMANT_BANDS {
            ui.label(format!("Band {:02}", band + 1));
            let mut v = motion.band_at(frame, band) as i32;
            if ui.add(egui::Slider::new(&mut v, 0..=127)).changed() {
                motion.set_band(frame, band, v as u8);
            }
            ui.end_row();
        }
    });
}

fn band_color(band: usize, bands: usize) -> egui::Color32 {
    let h = band as f32 / bands.max(1) as f32;
    egui::ecolor::Hsva::new(h, 0.7, 0.95, 1.0).into()
}

struct FormantGeom {
    base_x: f32,
    base_y: f32,
    skew_x: f32,
    skew_y: f32,
    amp: f32,
    plot_w: f32,
    steps: usize,
}

impl FormantGeom {
    const DATA_FRAC: f32 = 0.9;
    const LEAN_FROM_VERTICAL_DEG: f32 = 10.0;

    fn new(rect: egui::Rect, bands: usize, steps: usize) -> Self {
        let skew_y = rect.height() * 0.55 / bands as f32;
        let skew_x = skew_y * Self::LEAN_FROM_VERTICAL_DEG.to_radians().tan();
        Self {
            base_x: rect.left() + sp(-1),
            base_y: rect.bottom() - sp(-1),
            skew_x,
            skew_y,
            amp: skew_y * 4.0,
            plot_w: rect.width() - skew_x * bands as f32 - sp(0),
            steps,
        }
    }

    fn t_for(&self, step: usize) -> f32 {
        if self.steps > 1 {
            step as f32 / (self.steps - 1) as f32 * Self::DATA_FRAC
        } else {
            0.0
        }
    }

    fn x_at(&self, band: usize, t: f32) -> f32 {
        self.base_x + self.skew_x * band as f32 + t * self.plot_w
    }

    fn y_at(&self, band: usize, level: u8) -> f32 {
        self.base_y - self.skew_y * band as f32 - (level as f32 / 127.0) * self.amp
    }

    fn point(&self, band: usize, step: usize, level: u8) -> egui::Pos2 {
        egui::pos2(self.x_at(band, self.t_for(step)), self.y_at(band, level))
    }

    fn pick(&self, band: usize, pos: egui::Pos2) -> (usize, u8) {
        let t = ((pos.x - self.x_at(band, 0.0)) / self.plot_w).clamp(0.0, Self::DATA_FRAC);
        let step = if self.steps > 1 {
            (t / Self::DATA_FRAC * (self.steps - 1) as f32).round() as usize
        } else {
            0
        };
        let baseline = self.y_at(band, 0);
        let level = (((baseline - pos.y) / self.amp).clamp(0.0, 1.0) * 127.0).round() as u8;
        (step.min(self.steps.saturating_sub(1)), level)
    }
}

fn draw_band(
    painter: &egui::Painter,
    geom: &FormantGeom,
    motion: &FormantMotion,
    band: usize,
    stroke: egui::Stroke,
) {
    let mut points: Vec<egui::Pos2> = (0..geom.steps)
        .map(|step| geom.point(band, step, motion.band_at(step, band)))
        .collect();
    points.push(egui::pos2(
        geom.x_at(band, FormantGeom::DATA_FRAC),
        geom.y_at(band, 0),
    ));
    points.push(egui::pos2(geom.x_at(band, 1.0), geom.y_at(band, 0)));
    painter.add(egui::Shape::line(points, stroke));
}

fn formant_waterfall(ui: &mut Ui, motion: &FormantMotion) {
    let desired = egui::vec2((ui.available_width() - sp(0)).max(sp(8)), sp(7).max(360.0));
    let (response, painter) = ui.allocate_painter(desired, egui::Sense::hover());
    let rect = response.rect;

    painter.rect_filled(rect, 2.0, palette::HEADER);

    let steps = motion.steps.len();
    if steps == 0 {
        return;
    }

    let geom = FormantGeom::new(rect, FORMANT_BANDS, steps);
    for band in (0..FORMANT_BANDS).rev() {
        draw_band(
            &painter,
            &geom,
            motion,
            band,
            egui::Stroke::new(1.0, band_color(band, FORMANT_BANDS)),
        );
    }
}

fn section(ui: &mut Ui, title: &str, add: impl FnOnce(&mut Ui)) {
    Frame::group(ui.style())
        .fill(palette::PANEL)
        .inner_margin(Margin::same(sp(-1) as i8))
        .show(ui, |ui| {
            ui.vertical(|ui| {
                ui.strong(title);
                ui.add_space(sp(-2));
                add(ui);
            });
        });
}

trait Field: std::fmt::Display {
    fn fget(&self) -> i32;
    fn fset(&mut self, v: i32);
    fn flo() -> i32
    where
        Self: Sized;
    fn fhi() -> i32
    where
        Self: Sized;
}

macro_rules! field_impl {
    ($t:ty, $p:ty) => {
        impl Field for $t {
            fn fget(&self) -> i32 {
                self.get() as i32
            }
            fn fset(&mut self, v: i32) {
                self.set(v as $p);
            }
            fn flo() -> i32 {
                <$t>::LO as i32
            }
            fn fhi() -> i32 {
                <$t>::HI as i32
            }
        }
    };
}

field_impl!(Centered63, i16);
field_impl!(Transpose48, i16);
field_impl!(Semitone24, i16);
field_impl!(Detune50, i16);
field_impl!(BendRange12, i16);
field_impl!(Swing50, i16);
field_impl!(EqGain30, i16);
field_impl!(GlobalTranspose12, i16);
field_impl!(U7, u8);
field_impl!(UnisonDetune, u8);
field_impl!(Dwgs, u8);
field_impl!(GateTime, u8);
field_impl!(Category, u8);
field_impl!(KnobAssign, u8);
field_impl!(SyncNote, u8);
field_impl!(WaveShape, u8);
field_impl!(MotionSeqType, u8);
field_impl!(MotionSeqResolution, u8);
field_impl!(ArpResolution, u8);
field_impl!(MidiCtrlNo, u8);
field_impl!(FxParam, u8);

impl Field for Pan {
    fn fget(&self) -> i32 {
        self.get() as i32
    }
    fn fset(&mut self, v: i32) {
        *self = Pan::new(v as i16);
    }
    fn flo() -> i32 {
        -63
    }
    fn fhi() -> i32 {
        63
    }
}
impl Field for Tempo {
    fn fget(&self) -> i32 {
        self.bpm() as i32
    }
    fn fset(&mut self, v: i32) {
        *self = Tempo::new(v as u16);
    }
    fn flo() -> i32 {
        20
    }
    fn fhi() -> i32 {
        300
    }
}
impl Field for MasterTune {
    fn fget(&self) -> i32 {
        self.get() as i32
    }
    fn fset(&mut self, v: i32) {
        *self = MasterTune::new(v as i16);
    }
    fn flo() -> i32 {
        -100
    }
    fn fhi() -> i32 {
        100
    }
}

fn row_field<T: Field>(ui: &mut Ui, id_salt: &str, label: &str, value: &mut T) {
    ui.label(label);
    let mut v = value.fget();
    ui.push_id(id_salt, |ui| {
        if ui
            .add(egui::DragValue::new(&mut v).range(T::flo()..=T::fhi()))
            .changed()
        {
            value.fset(v);
        }
    });
    ui.end_row();
}

fn row_enum<T>(ui: &mut Ui, id_salt: &str, label: &str, value: &mut T)
where
    T: midilab::IntoEnumIterator + std::fmt::Display + Clone + Copy + PartialEq,
{
    ui.label(label);
    egui::ComboBox::from_id_salt(id_salt)
        .selected_text(format!("{}", value))
        .show_ui(ui, |ui| {
            for v in T::iter() {
                ui.selectable_value(value, v, format!("{}", v));
            }
        });
    ui.end_row();
}

fn compact_enum<T>(ui: &mut Ui, id_salt: &str, value: &mut T)
where
    T: midilab::IntoEnumIterator + std::fmt::Display + Clone + Copy + PartialEq,
{
    egui::ComboBox::from_id_salt(id_salt)
        .selected_text(format!("{}", value))
        .show_ui(ui, |ui| {
            for v in T::iter() {
                ui.selectable_value(value, v, format!("{}", v));
            }
        });
}

fn patch_combo<T>(ui: &mut Ui, id_salt: &str, value: &mut T)
where
    T: midilab::IntoEnumIterator + std::fmt::Display + Clone + Copy + PartialEq,
{
    egui::ComboBox::from_id_salt(id_salt)
        .selected_text(format!("{}", value))
        .show_ui(ui, |ui| {
            for v in T::iter() {
                ui.selectable_value(value, v, format!("{}", v));
            }
        });
}

fn row_u8(
    ui: &mut Ui,
    id_salt: &str,
    label: &str,
    value: &mut u8,
    range: std::ops::RangeInclusive<u8>,
) {
    ui.label(label);
    let mut v = *value;
    ui.push_id(id_salt, |ui| {
        if ui
            .add(egui::DragValue::new(&mut v).range(range.clone()))
            .changed()
        {
            *value = v;
        }
    });
    ui.end_row();
}

fn row_i8(
    ui: &mut Ui,
    id_salt: &str,
    label: &str,
    value: &mut i8,
    range: std::ops::RangeInclusive<i8>,
) {
    ui.label(label);
    let mut v = *value;
    ui.push_id(id_salt, |ui| {
        if ui
            .add(egui::DragValue::new(&mut v).range(range.clone()))
            .changed()
        {
            *value = v;
        }
    });
    ui.end_row();
}

fn row_bool(ui: &mut Ui, id_salt: &str, label: &str, value: &mut bool) {
    ui.label(label);
    ui.push_id(id_salt, |ui| {
        ui.checkbox(value, "");
    });
    ui.end_row();
}
