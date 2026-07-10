use std::time::Instant;

use midilab::manufacturer::korg::r3::KorgR3Message;
use midilab::manufacturer::korg::r3::live;
use midilab::manufacturer::korg::r3::wrappers::FormantMotion;
use midilab::manufacturer::korg::r3::wrappers::Global;
use midilab::manufacturer::korg::r3::wrappers::Program;
use midilab::manufacturer::korg::r3::wrappers::ProgramSlot;

use crate::config::AppConfig;
use crate::message::AppEffect;
use crate::message::AppMsg;
use crate::message::DeviceMsg;
use crate::message::IoEffect;
use crate::message::IoMsg;
use crate::message::UiEffect;
use crate::message::UiMsg;
use crate::message::UserError;
use crate::message::UserMsg;
use crate::message::UserMsgKind;

pub struct AppState {
    pub program: Program,
    pub global: Global,
    pub formant_motion: Option<FormantMotion>,
    pub selected_slot: ProgramSlot,
    pub config: AppConfig,
}

impl AppState {
    pub fn new(config: AppConfig) -> Self {
        Self {
            program: Program::default(),
            global: Global::default(),
            formant_motion: None,
            selected_slot: ProgramSlot::default(),
            config,
        }
    }

    #[must_use]
    pub fn update(&mut self, msg: AppMsg) -> Vec<AppEffect> {
        match msg {
            AppMsg::Ui(msg) => match msg {
                UiEffect::WriteProgram { program, slot } => {
                    self.program = (*program).clone();
                    vec![AppEffect::Device(DeviceMsg::WriteProgram { program, slot })]
                }
                UiEffect::LiveEdit(program) => {
                    let params = live::program_diff(&self.program, &program);
                    self.program = *program;
                    if params.is_empty() {
                        vec![]
                    } else {
                        vec![AppEffect::Device(DeviceMsg::LiveParams(params))]
                    }
                }
                UiEffect::DumpCurrentProgram => {
                    vec![AppEffect::Device(DeviceMsg::DumpCurrentProgram)]
                }
                UiEffect::DumpProgram(slot) => {
                    vec![AppEffect::Device(DeviceMsg::DumpProgram(slot))]
                }
                UiEffect::DumpSlot(slot) => {
                    self.selected_slot = slot;
                    self.program = Program::blank();
                    vec![AppEffect::Device(DeviceMsg::DumpSlot(slot))]
                }
                UiEffect::DumpCurrentFormantMotion => {
                    vec![AppEffect::Device(DeviceMsg::DumpCurrentFormantMotion)]
                }
                UiEffect::DumpFormantMotion(motion_no) => {
                    vec![AppEffect::Device(DeviceMsg::DumpFormantMotion(motion_no))]
                }
                UiEffect::WriteSelectedProgram => {
                    vec![AppEffect::Device(DeviceMsg::WriteSelectedProgram {
                        program: Box::new(self.program.clone()),
                        slot: self.selected_slot,
                    })]
                }
                UiEffect::WriteFormantMotion { motion, motion_no } => {
                    self.formant_motion = Some(motion.clone());
                    vec![AppEffect::Device(DeviceMsg::WriteFormantMotion {
                        motion,
                        motion_no,
                    })]
                }
                UiEffect::PersistProgram { program, path } => {
                    self.program = (*program).clone();
                    vec![AppEffect::Io(Box::new(IoMsg::SaveProgram {
                        program,
                        path,
                    }))]
                }
                UiEffect::ShowProgramSaveDialog => {
                    let path = std::env::temp_dir().join("korg_r3.program");
                    vec![AppEffect::Ui(UiMsg::SaveProgramDialog(path))]
                }
                UiEffect::ShowProgramLoadDialog => {
                    vec![AppEffect::Ui(UiMsg::LoadProgramDialog)]
                }
                UiEffect::LoadProgramFromFile { path } => {
                    vec![AppEffect::Io(Box::new(IoMsg::LoadProgram { path }))]
                }
                UiEffect::SendGlobalToDevice(global) => {
                    self.global = global.clone();
                    vec![]
                }
                UiEffect::RequestGlobalFromDevice => {
                    vec![AppEffect::Device(DeviceMsg::DumpGlobal)]
                }
                UiEffect::PersistGlobal { global, path } => {
                    self.global = global.clone();
                    vec![AppEffect::Io(Box::new(IoMsg::SaveGlobal { global, path }))]
                }
                UiEffect::ShowGlobalSaveDialog => {
                    let path = std::env::temp_dir().join("korg_r3.global");
                    vec![AppEffect::Ui(UiMsg::SaveGlobalDialog(path))]
                }
                UiEffect::ShowGlobalLoadDialog => {
                    vec![AppEffect::Ui(UiMsg::LoadGlobalDialog)]
                }
                UiEffect::LoadGlobalFromFile { path } => {
                    vec![AppEffect::Io(Box::new(IoMsg::LoadGlobal { path }))]
                }
                UiEffect::PersistFormantMotion { motion, path } => {
                    self.formant_motion = Some(motion.clone());
                    vec![AppEffect::Io(Box::new(IoMsg::SaveFormantMotion {
                        motion,
                        path,
                    }))]
                }
                UiEffect::ShowFormantMotionSaveDialog => {
                    let path = std::env::temp_dir().join("korg_r3.formant");
                    vec![AppEffect::Ui(UiMsg::SaveFormantMotionDialog(path))]
                }
                UiEffect::ShowFormantMotionLoadDialog => {
                    vec![AppEffect::Ui(UiMsg::LoadFormantMotionDialog)]
                }
                UiEffect::LoadFormantMotionFromFile { path } => {
                    vec![AppEffect::Io(Box::new(IoMsg::LoadFormantMotion { path }))]
                }
                UiEffect::ShowSettingsModal => {
                    vec![AppEffect::Ui(UiMsg::ShowSettingsModal)]
                }
                UiEffect::PersistUserSettings { config, path } => {
                    vec![AppEffect::Io(Box::new(IoMsg::PersistUserSettings {
                        config,
                        path,
                    }))]
                }
                UiEffect::AutoSync => {
                    vec![
                        AppEffect::Device(DeviceMsg::DumpCurrentProgram),
                        AppEffect::Device(DeviceMsg::DumpGlobal),
                    ]
                }
            },
            AppMsg::Device(kmsg) => match kmsg {
                KorgR3Message::CurrentProgramDump(program) => {
                    let wrapper: Program = match (*program).try_into() {
                        Ok(w) => w,
                        Err(e) => {
                            return vec![AppEffect::Ui(UiMsg::UserMsg(UserMsg {
                                msg: format!("Program parse error: {e}"),
                                kind: UserMsgKind::Error,
                                received_at: Instant::now(),
                            }))];
                        }
                    };
                    self.program = wrapper.clone();
                    vec![
                        AppEffect::Ui(UiMsg::UpdateProgram(Box::new(wrapper))),
                        AppEffect::Ui(UiMsg::UserMsg(UserMsg {
                            msg: "Current program loaded from device".to_string(),
                            kind: UserMsgKind::Status,
                            received_at: Instant::now(),
                        })),
                    ]
                }
                KorgR3Message::ProgramDump {
                    program_no,
                    program,
                } => {
                    let slot = ProgramSlot::new(program_no);
                    self.selected_slot = slot;
                    let wrapper: Program = match (*program).try_into() {
                        Ok(w) => w,
                        Err(e) => {
                            return vec![AppEffect::Ui(UiMsg::UserMsg(UserMsg {
                                msg: format!("Program parse error: {e}"),
                                kind: UserMsgKind::Error,
                                received_at: Instant::now(),
                            }))];
                        }
                    };
                    self.program = wrapper.clone();
                    vec![
                        AppEffect::Ui(UiMsg::UpdateProgram(Box::new(wrapper))),
                        AppEffect::Ui(UiMsg::UserMsg(UserMsg {
                            msg: format!("Program slot {program_no} loaded from device"),
                            kind: UserMsgKind::Status,
                            received_at: Instant::now(),
                        })),
                    ]
                }
                KorgR3Message::GlobalDump(global) => {
                    let wrapper: Global = match (*global).try_into() {
                        Ok(w) => w,
                        Err(e) => {
                            return vec![AppEffect::Ui(UiMsg::UserMsg(UserMsg {
                                msg: format!("Global parse error: {e}"),
                                kind: UserMsgKind::Error,
                                received_at: Instant::now(),
                            }))];
                        }
                    };
                    self.global = wrapper.clone();
                    vec![
                        AppEffect::Ui(UiMsg::UpdateGlobal(Box::new(wrapper))),
                        AppEffect::Ui(UiMsg::UserMsg(UserMsg {
                            msg: "Global settings loaded from device".to_string(),
                            kind: UserMsgKind::Status,
                            received_at: Instant::now(),
                        })),
                    ]
                }
                KorgR3Message::WriteCompleted => {
                    vec![AppEffect::Ui(UiMsg::UserMsg(UserMsg {
                        msg: "Write to device completed".to_string(),
                        kind: UserMsgKind::Status,
                        received_at: Instant::now(),
                    }))]
                }
                KorgR3Message::WriteError => {
                    vec![AppEffect::Ui(UiMsg::UserMsg(UserMsg {
                        msg: "Write to device failed".to_string(),
                        kind: UserMsgKind::Error,
                        received_at: Instant::now(),
                    }))]
                }
                KorgR3Message::ParameterChange(raw) => {
                    if live::apply_parameter_change(&mut self.program, &raw) {
                        vec![AppEffect::Ui(UiMsg::UpdateProgram(Box::new(
                            self.program.clone(),
                        )))]
                    } else {
                        vec![]
                    }
                }
                KorgR3Message::CurrentFormantMotionDump { steps, .. } => {
                    let motion = FormantMotion::from_raw(None, &steps);
                    let count = motion.steps.len();
                    self.formant_motion = Some(motion.clone());
                    vec![
                        AppEffect::Ui(UiMsg::UpdateFormantMotion(Box::new(motion))),
                        AppEffect::Ui(UiMsg::UserMsg(UserMsg {
                            msg: format!("Current formant motion loaded ({count} steps)"),
                            kind: UserMsgKind::Status,
                            received_at: Instant::now(),
                        })),
                    ]
                }
                KorgR3Message::FormantMotionDump {
                    motion_no, steps, ..
                } => {
                    let motion = FormantMotion::from_raw(Some(motion_no), &steps);
                    let count = motion.steps.len();
                    self.formant_motion = Some(motion.clone());
                    vec![
                        AppEffect::Ui(UiMsg::UpdateFormantMotion(Box::new(motion))),
                        AppEffect::Ui(UiMsg::UserMsg(UserMsg {
                            msg: format!("Formant motion {} loaded ({count} steps)", motion_no + 1),
                            kind: UserMsgKind::Status,
                            received_at: Instant::now(),
                        })),
                    ]
                }
                _ => vec![],
            },
            AppMsg::Io(io_effect) => match *io_effect {
                IoEffect::ProgramSaveResult(result) => match result {
                    Ok(path) => vec![AppEffect::Ui(UiMsg::UserMsg(UserMsg {
                        msg: format!("Saved program {}", path),
                        kind: UserMsgKind::Status,
                        received_at: Instant::now(),
                    }))],
                    Err(e) => vec![AppEffect::Ui(UiMsg::UserMsg(UserMsg {
                        msg: format!("Program save failed: {e}"),
                        kind: UserMsgKind::Error,
                        received_at: Instant::now(),
                    }))],
                },
                IoEffect::ProgramLoadResult(result) => match result {
                    Ok(program) => {
                        self.program = (*program).clone();
                        vec![
                            AppEffect::Ui(UiMsg::UpdateProgram(program)),
                            AppEffect::Ui(UiMsg::UserMsg(UserMsg {
                                msg: "Program loaded".to_string(),
                                kind: UserMsgKind::Status,
                                received_at: Instant::now(),
                            })),
                        ]
                    }
                    Err(e) => vec![AppEffect::Ui(UiMsg::UserMsg(UserMsg {
                        msg: format!("Program load failed: {e}"),
                        kind: UserMsgKind::Error,
                        received_at: Instant::now(),
                    }))],
                },
                IoEffect::GlobalSaveResult(result) => match result {
                    Ok(path) => vec![AppEffect::Ui(UiMsg::UserMsg(UserMsg {
                        msg: format!("Saved global {}", path),
                        kind: UserMsgKind::Status,
                        received_at: Instant::now(),
                    }))],
                    Err(e) => vec![AppEffect::Ui(UiMsg::UserMsg(UserMsg {
                        msg: format!("Global save failed: {e}"),
                        kind: UserMsgKind::Error,
                        received_at: Instant::now(),
                    }))],
                },
                IoEffect::GlobalLoadResult(result) => match result {
                    Ok(global) => {
                        self.global = global.clone();
                        vec![
                            AppEffect::Ui(UiMsg::UpdateGlobal(Box::new(global))),
                            AppEffect::Ui(UiMsg::UserMsg(UserMsg {
                                msg: "Global loaded".to_string(),
                                kind: UserMsgKind::Status,
                                received_at: Instant::now(),
                            })),
                        ]
                    }
                    Err(e) => vec![AppEffect::Ui(UiMsg::UserMsg(UserMsg {
                        msg: format!("Global load failed: {e}"),
                        kind: UserMsgKind::Error,
                        received_at: Instant::now(),
                    }))],
                },
                IoEffect::FormantMotionSaveResult(result) => match result {
                    Ok(path) => vec![AppEffect::Ui(UiMsg::UserMsg(UserMsg {
                        msg: format!("Saved formant motion {}", path),
                        kind: UserMsgKind::Status,
                        received_at: Instant::now(),
                    }))],
                    Err(e) => vec![AppEffect::Ui(UiMsg::UserMsg(UserMsg {
                        msg: format!("Formant motion save failed: {e}"),
                        kind: UserMsgKind::Error,
                        received_at: Instant::now(),
                    }))],
                },
                IoEffect::FormantMotionLoadResult(result) => match result {
                    Ok(motion) => {
                        self.formant_motion = Some(motion.clone());
                        let count = motion.steps.len();
                        vec![
                            AppEffect::Ui(UiMsg::UpdateFormantMotion(Box::new(motion))),
                            AppEffect::Ui(UiMsg::UserMsg(UserMsg {
                                msg: format!("Formant motion loaded ({count} frames)"),
                                kind: UserMsgKind::Status,
                                received_at: Instant::now(),
                            })),
                        ]
                    }
                    Err(e) => vec![AppEffect::Ui(UiMsg::UserMsg(UserMsg {
                        msg: format!("Formant motion load failed: {e}"),
                        kind: UserMsgKind::Error,
                        received_at: Instant::now(),
                    }))],
                },
                IoEffect::PersistConfigResult(result) => match result {
                    Ok(_) => vec![],
                    Err(e) => vec![AppEffect::Ui(UiMsg::UserMsg(UserMsg {
                        msg: format!("App config save failed: {e}"),
                        kind: UserMsgKind::Error,
                        received_at: Instant::now(),
                    }))],
                },
                IoEffect::PersistUserSettingsResult(result) => match result {
                    Ok(_) => vec![],
                    Err(e) => vec![AppEffect::Ui(UiMsg::UserMsg(UserMsg {
                        msg: format!("User settings save failed: {e}"),
                        kind: UserMsgKind::Error,
                        received_at: Instant::now(),
                    }))],
                },
            },
            AppMsg::UserError(e) => match e {
                UserError::Midi(e) => vec![AppEffect::Ui(UiMsg::UserMsg(UserMsg {
                    msg: e.to_string(),
                    received_at: Instant::now(),
                    kind: UserMsgKind::Error,
                }))],
                UserError::Parse(e) => vec![AppEffect::Ui(UiMsg::UserMsg(UserMsg {
                    msg: e.to_string(),
                    received_at: Instant::now(),
                    kind: UserMsgKind::Error,
                }))],
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use midilab::manufacturer::korg::r3::live::LiveParam;
    use midilab::manufacturer::korg::r3::live::ParamAddr;
    use midilab::manufacturer::korg::r3::wrappers::U7;

    use super::*;

    #[test]
    fn live_edit_emits_changed_params_and_updates_snapshot() {
        let mut app = AppState::new(AppConfig::default());
        app.program = Program::blank();

        let mut edited = app.program.clone();
        edited.timbre1.filter.cutoff1 = U7::new(99);

        let effects = app.update(AppMsg::Ui(UiEffect::LiveEdit(Box::new(edited))));

        assert_eq!(app.program.timbre1.filter.cutoff1.get(), 99);
        assert_eq!(effects.len(), 1);
        match &effects[0] {
            AppEffect::Device(DeviceMsg::LiveParams(params)) => {
                assert_eq!(params.len(), 1);
                assert_eq!(
                    params[0].addr,
                    ParamAddr {
                        id: 0x10,
                        sub: 0x32
                    }
                );
                assert_eq!(params[0].value, 99);
            }
            other => panic!(
                "expected LiveParams, got something else: {:?}",
                matches!(other, AppEffect::Device(_))
            ),
        }
    }

    #[test]
    fn live_edit_with_no_change_emits_nothing() {
        let mut app = AppState::new(AppConfig::default());
        app.program = Program::blank();
        let same = app.program.clone();

        let effects = app.update(AppMsg::Ui(UiEffect::LiveEdit(Box::new(same))));
        assert!(effects.is_empty());
    }

    #[test]
    fn inbound_parameter_change_applies_and_refreshes_ui() {
        let mut app = AppState::new(AppConfig::default());
        app.program = Program::blank();

        let bytes = LiveParam {
            addr: ParamAddr {
                id: 0x10,
                sub: 0x32,
            },
            value: 77,
        }
        .to_sysex(0x00);
        let kmsg = KorgR3Message::try_from(bytes.as_slice()).expect("valid sysex");

        let effects = app.update(AppMsg::Device(kmsg));

        assert_eq!(app.program.timbre1.filter.cutoff1.get(), 77);
        assert_eq!(effects.len(), 1);
        assert!(matches!(effects[0], AppEffect::Ui(UiMsg::UpdateProgram(_))));
    }

    #[test]
    fn auto_sync_dumps_program_and_global() {
        let mut app = AppState::new(AppConfig::default());

        let effects = app.update(AppMsg::Ui(UiEffect::AutoSync));

        assert_eq!(effects.len(), 2);
        assert!(matches!(
            &effects[0],
            AppEffect::Device(DeviceMsg::DumpCurrentProgram)
        ));
        assert!(matches!(
            &effects[1],
            AppEffect::Device(DeviceMsg::DumpGlobal)
        ));
    }
}
