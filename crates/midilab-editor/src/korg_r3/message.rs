use std::path::PathBuf;
use std::time::Instant;

use midilab::error::MidiError;
use midilab::manufacturer::korg::r3::KorgR3Message;
use midilab::manufacturer::korg::r3::ParseError;
use midilab::manufacturer::korg::r3::live::LiveParam;
use midilab::manufacturer::korg::r3::wrappers::FormantMotion;
use midilab::manufacturer::korg::r3::wrappers::Global;
use midilab::manufacturer::korg::r3::wrappers::Program;
use midilab::manufacturer::korg::r3::wrappers::ProgramSlot;

use crate::korg_r3::config::AppConfig;
use crate::korg_r3::config::UserSettings;

pub enum AppMsg {
    Device(KorgR3Message),
    Ui(UiEffect),
    UserError(UserError),
    Io(Box<IoEffect>),
}

pub enum UserError {
    Midi(MidiError),
    Parse(ParseError),
}

pub enum IoMsg {
    PersistConfig {
        config: AppConfig,
        path: PathBuf,
    },
    PersistUserSettings {
        config: AppConfig,
        path: PathBuf,
    },
    SaveProgram {
        program: Box<Program>,
        path: PathBuf,
    },
    LoadProgram {
        path: PathBuf,
    },
    SaveGlobal {
        global: Global,
        path: PathBuf,
    },
    LoadGlobal {
        path: PathBuf,
    },
    SaveFormantMotion {
        motion: FormantMotion,
        path: PathBuf,
    },
    LoadFormantMotion {
        path: PathBuf,
    },
}

pub enum IoEffect {
    PersistConfigResult(Result<(), String>),
    PersistUserSettingsResult(Result<(), String>),
    ProgramSaveResult(Result<String, String>),
    ProgramLoadResult(Result<Box<Program>, String>),
    GlobalSaveResult(Result<String, String>),
    GlobalLoadResult(Result<Global, String>),
    FormantMotionSaveResult(Result<String, String>),
    FormantMotionLoadResult(Result<FormantMotion, String>),
}

pub enum AppEffect {
    Ui(UiMsg),
    Device(DeviceMsg),
    Io(Box<IoMsg>),
}

pub enum UiMsg {
    UpdateProgram(Box<Program>),
    UpdateGlobal(Box<Global>),
    UpdateFormantMotion(Box<FormantMotion>),
    UserMsg(UserMsg),
    DirectoryConfigured(PathBuf),
    LoadProgramDialog,
    SaveProgramDialog(PathBuf),
    LoadGlobalDialog,
    SaveGlobalDialog(PathBuf),
    LoadFormantMotionDialog,
    SaveFormantMotionDialog(PathBuf),
    ShowSettingsModal,
    UpdateUserSettings(UserSettings),
    AutoSync,
}

pub enum UiEffect {
    WriteProgram {
        program: Box<Program>,
        slot: u8,
    },
    LiveEdit(Box<Program>),
    DumpCurrentProgram,
    DumpProgram(u8),
    DumpSlot(ProgramSlot),
    DumpCurrentFormantMotion,
    DumpFormantMotion(u8),
    WriteFormantMotion {
        motion: FormantMotion,
        motion_no: u8,
    },
    WriteSelectedProgram,
    PersistProgram {
        program: Box<Program>,
        path: PathBuf,
    },
    ShowProgramSaveDialog,
    ShowProgramLoadDialog,
    LoadProgramFromFile {
        path: PathBuf,
    },
    SendGlobalToDevice(Global),
    RequestGlobalFromDevice,
    PersistGlobal {
        global: Global,
        path: PathBuf,
    },
    ShowGlobalSaveDialog,
    ShowGlobalLoadDialog,
    LoadGlobalFromFile {
        path: PathBuf,
    },
    PersistFormantMotion {
        motion: FormantMotion,
        path: PathBuf,
    },
    ShowFormantMotionSaveDialog,
    ShowFormantMotionLoadDialog,
    LoadFormantMotionFromFile {
        path: PathBuf,
    },
    ShowSettingsModal,
    PersistUserSettings {
        config: AppConfig,
        path: PathBuf,
    },
    AutoSync,
}

pub enum DeviceMsg {
    DumpCurrentProgram,
    DumpProgram(u8),
    DumpSlot(ProgramSlot),
    DumpCurrentFormantMotion,
    DumpFormantMotion(u8),
    DumpGlobal,
    WriteProgram {
        program: Box<Program>,
        slot: u8,
    },
    WriteSelectedProgram {
        program: Box<Program>,
        slot: ProgramSlot,
    },
    WriteFormantMotion {
        motion: FormantMotion,
        motion_no: u8,
    },
    LiveParams(Vec<LiveParam>),
}

pub struct UserMsg {
    pub msg: String,
    pub kind: UserMsgKind,
    pub received_at: Instant,
}

pub enum UserMsgKind {
    Status,
    Error,
}
