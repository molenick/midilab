use bytemuck::Zeroable;
use num_enum::IntoPrimitive;
use num_enum::TryFromPrimitive;
use strum_macros::Display;
use strum_macros::EnumIter;

pub use super::raw::RawArpeggio;
pub use super::raw::RawGlobal;
pub use super::raw::RawInsertFx;
pub use super::raw::RawMasterFx;
pub use super::raw::RawMotionSeq;
pub use super::raw::RawProgram;
pub use super::raw::RawTimbre;
pub use super::raw::RawTimbreProgram;
pub use super::raw::RawVocoder;

macro_rules! centered {
    ($(#[$m:meta])* $name:ident, $lo:literal..=$hi:literal, $center:literal) => {
        $(#[$m])*
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        pub struct $name(i16);
        impl $name {
            pub const LO: i16 = $lo;
            pub const HI: i16 = $hi;
            pub const CENTER: i16 = $center;
            pub fn new(v: i16) -> Self { Self(v.clamp($lo, $hi)) }
            pub fn from_wire(b: u8) -> Self { Self::new(b as i16 - $center) }
            pub fn to_wire(self) -> u8 { (self.0 + $center).clamp(0, 127) as u8 }
            pub fn get(self) -> i16 { self.0 }
            pub fn set(&mut self, v: i16) { self.0 = v.clamp($lo, $hi); }
        }
        impl Default for $name { fn default() -> Self { Self(0) } }
        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { write!(f, "{:+}", self.0) }
        }
    };
}

macro_rules! signed {
    ($(#[$m:meta])* $name:ident, $lo:literal..=$hi:literal) => {
        $(#[$m])*
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        pub struct $name(i16);
        impl $name {
            pub const LO: i16 = $lo;
            pub const HI: i16 = $hi;
            pub fn new(v: i16) -> Self { Self(v.clamp($lo, $hi)) }
            pub fn from_wire(b: u8) -> Self { Self::new(b as i8 as i16) }
            pub fn to_wire(self) -> u8 { (self.0 as i8) as u8 }
            pub fn get(self) -> i16 { self.0 }
            pub fn set(&mut self, v: i16) { self.0 = v.clamp($lo, $hi); }
        }
        impl Default for $name { fn default() -> Self { Self(0) } }
        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { write!(f, "{:+}", self.0) }
        }
    };
}

macro_rules! unsigned {
    ($(#[$m:meta])* $name:ident, $lo:literal..=$hi:literal, $def:literal) => {
        $(#[$m])*
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        pub struct $name(u8);
        impl $name {
            pub const LO: u8 = $lo;
            pub const HI: u8 = $hi;
            pub fn new(v: u8) -> Self { Self(v.clamp($lo, $hi)) }
            pub fn from_wire(b: u8) -> Self { Self::new(b) }
            pub fn to_wire(self) -> u8 { self.0 }
            pub fn get(self) -> u8 { self.0 }
            pub fn set(&mut self, v: u8) { self.0 = v.clamp($lo, $hi); }
        }
        impl Default for $name { fn default() -> Self { Self($def) } }
        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { write!(f, "{}", self.0) }
        }
    };
}

centered!(Centered63, -63..=63, 64);
centered!(Transpose48, -48..=48, 64);
centered!(Semitone24, -24..=24, 64);
centered!(Detune50, -50..=50, 64);
centered!(BendRange12, -12..=12, 64);
centered!(Swing50, -50..=50, 64);
centered!(EqGain30, -30..=30, 64);
signed!(GlobalTranspose12, -12..=12);

unsigned!(U7, 0..=127, 0);
unsigned!(UnisonDetune, 0..=99, 0);
unsigned!(Dwgs, 0..=64, 0);
unsigned!(GateTime, 0..=100, 0);
unsigned!(Category, 0..=15, 0);
unsigned!(KnobAssign, 0..=55, 0);
unsigned!(SyncNote, 0..=16, 0);
unsigned!(WaveShape, 0..=12, 0);
unsigned!(MotionSeqType, 0..=4, 0);
unsigned!(ArpResolution, 0..=8, 6);
unsigned!(MotionSeqResolution, 0..=15, 0);
unsigned!(MidiCtrlNo, 0..=115, 0);
unsigned!(FxParam, 0..=127, 0);

pub const SYNC_NOTE_LABELS: [&str; 17] = [
    "8/1", "4/1", "2/1", "1/1", "3/4", "1/2", "3/8", "1/3", "1/4", "3/16", "1/6", "1/8", "1/12",
    "1/16", "1/24", "1/32", "1/64",
];
pub const ARP_RES_LABELS: [&str; 9] = [
    "1/32", "1/24", "1/16", "1/12", "1/8", "1/6", "1/4", "1/2", "1/1",
];

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct Pan(i16);
impl Pan {
    pub fn new(v: i16) -> Self {
        Self(v.clamp(-63, 63))
    }
    pub fn from_wire(b: u8) -> Self {
        Self::new(b.max(1) as i16 - 64)
    }
    pub fn to_wire(self) -> u8 {
        (self.0 + 64).clamp(1, 127) as u8
    }
    pub fn get(self) -> i16 {
        self.0
    }
}
impl std::fmt::Display for Pan {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.0 {
            0 => write!(f, "CNT"),
            n if n < 0 => write!(f, "L{}", -n),
            n => write!(f, "R{}", n),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Tempo(u16);
impl Tempo {
    pub fn new(v: u16) -> Self {
        Self(v.clamp(20, 300))
    }
    pub fn from_wire(v: u16) -> Self {
        Self::new(if v == 0 { 120 } else { v })
    }
    pub fn to_wire(self) -> u16 {
        self.0
    }
    pub fn bpm(self) -> u16 {
        self.0
    }
}
impl Default for Tempo {
    fn default() -> Self {
        Self(120)
    }
}
impl std::fmt::Display for Tempo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct MasterTune(i16);
impl MasterTune {
    pub fn new(v: i16) -> Self {
        Self(v.clamp(-100, 100))
    }
    pub fn from_wire(b: u8) -> Self {
        Self::new(b as i8 as i16)
    }
    pub fn to_wire(self) -> u8 {
        (self.0 as i8) as u8
    }
    pub fn get(self) -> i16 {
        self.0
    }
}
impl std::fmt::Display for MasterTune {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:+}", self.0)
    }
}

macro_rules! spec_enum {
    ($name:ident { $def:ident = 0 $(, $variant:ident = $val:literal)* $(,)? }) => {
        #[repr(u8)]
        #[derive(Debug, IntoPrimitive, TryFromPrimitive, PartialEq, Eq, Clone, Copy, Default, EnumIter, Display)]
        pub enum $name { #[default] $def = 0 $(, $variant = $val)* }
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ProgramSlot(u16);

impl ProgramSlot {
    pub const COUNT: u16 = 128;
    pub const MAX: u16 = Self::COUNT - 1;

    pub fn new(n: u16) -> Self {
        Self(n.min(Self::MAX))
    }
    pub fn as_u16(self) -> u16 {
        self.0
    }
    pub fn bank(self) -> char {
        (b'A' + (self.0 / 8) as u8) as char
    }
    pub fn position(self) -> u8 {
        (self.0 % 8) as u8 + 1
    }
    pub fn is_vocoder(self) -> bool {
        self.0 >= 112
    }
}

impl std::fmt::Display for ProgramSlot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}", self.bank(), self.position())
    }
}

spec_enum!(VoiceMode { Single = 0, Layer = 1, Split = 2, Multi = 3 });
spec_enum!(ArpTimbre { Timbre1 = 0, Timbre2 = 1, Both = 2 });
spec_enum!(VoiceAssign { Mono1 = 0, Mono2 = 1, Poly = 2 });
spec_enum!(UnisonVoice { Off = 0, Two = 1, Three = 2, Four = 3 });
spec_enum!(OscMod { Waveform = 0, Cross = 1, Unison = 2, Vpm = 3 });
spec_enum!(Osc2Mod { Off = 0, Ring = 1, Sync = 2, RingSync = 3 });
spec_enum!(Osc2Wave { Saw = 0, Square = 1, Triangle = 2, Sine = 3 });
spec_enum!(Filter2Type { Lpf = 0, Hpf = 1, Bpf = 2, Comb = 3 });
spec_enum!(FilterRouting { Single = 0, Serial = 1, Parallel = 2, Individual = 3 });
spec_enum!(LfoKeySync { Off = 0, Timbre = 1, Voice = 2 });
spec_enum!(WaveShapePosition { PreFilter1 = 0, PreAmp = 1 });
spec_enum!(ArpType { Up = 0, Down = 1, Alt1 = 2, Alt2 = 3, Random = 4, Trigger = 5 });
spec_enum!(VelCurve { Curve1 = 0, Curve2 = 1, Curve3 = 2, Curve4 = 3, Curve5 = 4, Curve6 = 5, Curve7 = 6, Curve8 = 7, Const127 = 8 });

#[repr(u8)]
#[derive(
    Debug, IntoPrimitive, TryFromPrimitive, PartialEq, Eq, Clone, Copy, Default, EnumIter, Display,
)]
pub enum Osc1Wave {
    #[default]
    Saw = 0,
    Pulse = 1,
    Triangle = 2,
    #[strum(serialize = "Sin(Cross)")]
    SinCross = 3,
    Noise = 4,
    Formant = 5,
    Dwgs = 6,
    Pcm = 7,
    #[strum(serialize = "Audio In")]
    AudioIn = 8,
}

#[repr(u8)]
#[derive(
    Debug, IntoPrimitive, TryFromPrimitive, PartialEq, Eq, Clone, Copy, Default, EnumIter, Display,
)]
pub enum Lfo1Wave {
    #[default]
    Saw = 0,
    Square = 1,
    Triangle = 2,
    #[strum(serialize = "S/H")]
    SampleHold = 3,
    Random = 4,
}

#[repr(u8)]
#[derive(
    Debug, IntoPrimitive, TryFromPrimitive, PartialEq, Eq, Clone, Copy, Default, EnumIter, Display,
)]
pub enum Lfo2Wave {
    #[default]
    Saw = 0,
    Square = 1,
    Sine = 2,
    #[strum(serialize = "S/H")]
    SampleHold = 3,
    Random = 4,
}

#[repr(u8)]
#[derive(
    Debug, IntoPrimitive, TryFromPrimitive, PartialEq, Eq, Clone, Copy, Default, EnumIter, Display,
)]
pub enum PatchSource {
    #[default]
    Eg1 = 0,
    Eg2 = 1,
    Eg3 = 2,
    Lfo1 = 3,
    Lfo2 = 4,
    Velocity = 5,
    #[strum(serialize = "Pitch Bend")]
    PitchBend = 6,
    #[strum(serialize = "Mod Wheel")]
    ModWheel = 7,
    #[strum(serialize = "Key Track")]
    KeyTrack = 8,
    Midi1 = 9,
    Midi2 = 10,
    Midi3 = 11,
}

#[repr(u8)]
#[derive(
    Debug, IntoPrimitive, TryFromPrimitive, PartialEq, Eq, Clone, Copy, Default, EnumIter, Display,
)]
pub enum PatchDest {
    #[default]
    Pitch = 0,
    #[strum(serialize = "OSC2 Pitch")]
    Osc2Pitch = 1,
    #[strum(serialize = "OSC1 Ctrl1")]
    Osc1Ctrl1 = 2,
    #[strum(serialize = "OSC1 Level")]
    Osc1Level = 3,
    #[strum(serialize = "OSC2 Level")]
    Osc2Level = 4,
    #[strum(serialize = "Noise Level")]
    NoiseLevel = 5,
    #[strum(serialize = "Flt1 Type")]
    Flt1Type = 6,
    #[strum(serialize = "Flt1 Cutoff")]
    Flt1Cutoff = 7,
    #[strum(serialize = "Flt1 Resonance")]
    Flt1Resonance = 8,
    #[strum(serialize = "Flt2 Cutoff")]
    Flt2Cutoff = 9,
    #[strum(serialize = "Drive/WS Depth")]
    DriveWsDepth = 10,
    Amp = 11,
    Pan = 12,
    #[strum(serialize = "LFO1 Freq")]
    Lfo1Freq = 13,
    #[strum(serialize = "LFO2 Freq")]
    Lfo2Freq = 14,
}

#[repr(u8)]
#[derive(
    Debug, IntoPrimitive, TryFromPrimitive, PartialEq, Eq, Clone, Copy, Default, EnumIter, Display,
)]
pub enum FxType {
    #[default]
    #[strum(serialize = "No Effect")]
    NoEffect = 0,
    #[strum(serialize = "St.Compressor")]
    StCompressor = 1,
    #[strum(serialize = "St.Limiter")]
    StLimiter = 2,
    #[strum(serialize = "St.Gate")]
    StGate = 3,
    #[strum(serialize = "St.Filter")]
    StFilter = 4,
    #[strum(serialize = "St.Wah")]
    StWah = 5,
    #[strum(serialize = "St.BandEQ")]
    StBandEq = 6,
    Distortion = 7,
    #[strum(serialize = "CabinetSimltr")]
    CabinetSim = 8,
    #[strum(serialize = "TubePreampSim")]
    TubePreamp = 9,
    #[strum(serialize = "St.Decimator")]
    StDecimator = 10,
    Reverb = 11,
    #[strum(serialize = "Early Reflect")]
    EarlyReflect = 12,
    #[strum(serialize = "L/C/R Delay")]
    LcrDelay = 13,
    #[strum(serialize = "St.Delay")]
    StDelay = 14,
    #[strum(serialize = "AutoPanDelay")]
    AutoPanDelay = 15,
    #[strum(serialize = "St.AutoPanDly")]
    StAutoPanDelay = 16,
    #[strum(serialize = "Mod Delay")]
    ModDelay = 17,
    #[strum(serialize = "St.Mod Delay")]
    StModDelay = 18,
    #[strum(serialize = "Tape Echo")]
    TapeEcho = 19,
    #[strum(serialize = "St.Chorus")]
    StChorus = 20,
    Ensemble = 21,
    #[strum(serialize = "St.Flanger")]
    StFlanger = 22,
    #[strum(serialize = "St.Phaser")]
    StPhaser = 23,
    #[strum(serialize = "St.Tremolo")]
    StTremolo = 24,
    #[strum(serialize = "St.Ring Mod")]
    StRingMod = 25,
    #[strum(serialize = "Pitch Shifter")]
    PitchShifter = 26,
    #[strum(serialize = "Grain Shifter")]
    GrainShifter = 27,
    #[strum(serialize = "St.Vibrato")]
    StVibrato = 28,
    #[strum(serialize = "RotarySpeaker")]
    RotarySpeaker = 29,
    #[strum(serialize = "Talking Mod")]
    TalkingMod = 30,
}

#[repr(u8)]
#[derive(
    Debug, IntoPrimitive, TryFromPrimitive, PartialEq, Eq, Clone, Copy, Default, EnumIter, Display,
)]
pub enum Timbre2MidiCh {
    #[default]
    Ch1 = 0,
    Ch2 = 1,
    Ch3 = 2,
    Ch4 = 3,
    Ch5 = 4,
    Ch6 = 5,
    Ch7 = 6,
    Ch8 = 7,
    Ch9 = 8,
    Ch10 = 9,
    Ch11 = 10,
    Ch12 = 11,
    Ch13 = 12,
    Ch14 = 13,
    Ch15 = 14,
    Ch16 = 15,
    Global = 16,
}

#[repr(u8)]
#[derive(
    Debug, IntoPrimitive, TryFromPrimitive, PartialEq, Eq, Clone, Copy, Default, EnumIter, Display,
)]
pub enum MidiChannelR3 {
    #[default]
    Ch1 = 0,
    Ch2 = 1,
    Ch3 = 2,
    Ch4 = 3,
    Ch5 = 4,
    Ch6 = 5,
    Ch7 = 6,
    Ch8 = 7,
    Ch9 = 8,
    Ch10 = 9,
    Ch11 = 10,
    Ch12 = 11,
    Ch13 = 12,
    Ch14 = 13,
    Ch15 = 14,
    Ch16 = 15,
}

fn dec<T: TryFromPrimitive<Primitive = u8> + Default>(b: u8) -> T {
    T::try_from_primitive(b).unwrap_or_default()
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Osc1 {
    pub wave: Osc1Wave,
    pub osc_mod: OscMod,
    pub ctrl1: U7,
    pub ctrl2: U7,
    pub dwgs: Dwgs,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Osc2 {
    pub wave: Osc2Wave,
    pub osc_mod: Osc2Mod,
    pub semitone: Semitone24,
    pub tune: Detune50,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Mixer {
    pub osc1_level: U7,
    pub osc2_level: U7,
    pub noise_level: U7,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Filter {
    pub routing: FilterRouting,
    pub filter2_type: Filter2Type,
    pub balance: U7,
    pub cutoff1: U7,
    pub resonance1: U7,
    pub eg1_int1: Centered63,
    pub key_track1: Centered63,
    pub velo_sens1: Centered63,
    pub cutoff2: U7,
    pub resonance2: U7,
    pub eg1_int2: Centered63,
    pub key_track2: Centered63,
    pub velo_sens2: Centered63,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Amp {
    pub level: U7,
    pub ws_position: WaveShapePosition,
    pub ws_type: WaveShape,
    pub ws_depth: U7,
    pub pan: Pan,
    pub key_track: Centered63,
    pub punch_level: U7,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Eg {
    pub attack: U7,
    pub decay: U7,
    pub sustain: U7,
    pub release: U7,
    pub level_velo: Centered63,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Lfo {
    pub wave: u8,
    pub freq: U7,
    pub bpm_sync: bool,
    pub key_sync: LfoKeySync,
    pub sync_note: SyncNote,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Patch {
    pub src: PatchSource,
    pub dst: PatchDest,
    pub int: Centered63,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct InsertFx {
    pub fx1_type: FxType,
    pub fx1_knob: u8,
    pub fx1_params: [FxParam; 20],
    pub fx2_type: FxType,
    pub fx2_knob: u8,
    pub fx2_params: [FxParam; 20],
    pub eq_low_freq: u8,
    pub eq_low_gain: EqGain30,
    pub eq_hi_freq: u8,
    pub eq_hi_gain: EqGain30,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct MotionSeq {
    pub on: bool,
    pub seq_type: MotionSeqType,
    pub last_step: u8,
    pub key_sync: LfoKeySync,
    pub resolution: MotionSeqResolution,
    pub seq_params: [u8; 18],
}

#[derive(Clone, Copy, Debug)]
pub struct Timbre {
    pub knob_assigns: [KnobAssign; 4],
    pub unison_voice: UnisonVoice,
    pub unison_detune: UnisonDetune,
    pub unison_spread: U7,
    pub voice_assign: VoiceAssign,
    pub analog_tuning: U7,
    pub transpose: Transpose48,
    pub detune: Detune50,
    pub vibrato_int: Centered63,
    pub bend_range: BendRange12,
    pub portamento: U7,
    pub osc1: Osc1,
    pub osc2: Osc2,
    pub mixer: Mixer,
    pub filter: Filter,
    pub amp: Amp,
    pub eg: [Eg; 3],
    pub lfo: [Lfo; 2],
    pub patches: [Patch; 6],
    pub insert_fx: InsertFx,
    pub motion_seq: MotionSeq,
}
impl Default for Timbre {
    fn default() -> Self {
        Timbre::from(&RawTimbre::default())
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Vocoder {
    pub on: bool,
    pub source_formant_rec: bool,
    pub hpf_gate: bool,
    pub formant_trig_reset: bool,
    pub select_timbre2: bool,
    pub gate_sens: U7,
    pub threshold: U7,
    pub hpf_level: U7,
    pub direct_level: U7,
    pub timbre1_level: U7,
    pub input1_level: U7,
    pub vocoder_level: U7,
    pub band_pans: [Pan; 16],
    pub band_levels: [U7; 16],
    pub fc_mod_src: PatchSource,
    pub cutoff_offset: Centered63,
    pub resonance: U7,
    pub fc_mod_int: Centered63,
    pub ef_sens: U7,
}
impl Default for Vocoder {
    fn default() -> Self {
        Vocoder::from(&RawVocoder::default())
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct MasterFx {
    pub fx_type: FxType,
    pub knob_assign: u8,
    pub params: [FxParam; 20],
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Arpeggio {
    pub on: bool,
    pub key_sync: bool,
    pub resolution: ArpResolution,
    pub arp_type: ArpType,
    pub latch: bool,
    pub octave_range: u8,
    pub last_step: u8,
    pub gate_time: GateTime,
    pub swing: Swing50,
    pub step_switches: u8,
}

#[derive(Clone, Debug)]
pub struct Program {
    pub name: String,
    pub slot: ProgramSlot,
    pub voice_mode: VoiceMode,
    pub arp_timbre: ArpTimbre,
    pub vcd_knob_assigns: [KnobAssign; 4],
    pub timbre2_midi_ch: Timbre2MidiCh,
    pub center_key: u8,
    pub octave_sw: i8,
    pub category: Category,
    pub timbre1: Timbre,
    pub timbre2: Timbre,
    pub vocoder: Vocoder,
    pub master_fx: MasterFx,
    pub tempo: Tempo,
    pub arp: Arpeggio,
    backing: Box<RawProgram>,
}

impl From<&RawTimbre> for Timbre {
    fn from(raw: &RawTimbre) -> Self {
        let p = &raw.program;
        let eg_at = |a: u8, d: u8, s: u8, r: u8, lv: u8| Eg {
            attack: U7::from_wire(a),
            decay: U7::from_wire(d),
            sustain: U7::from_wire(s),
            release: U7::from_wire(r),
            level_velo: Centered63::from_wire(lv),
        };
        let lfo1 = Lfo {
            wave: p.lfo1_wave_val(),
            freq: U7::from_wire(p.lfo1_freq),
            bpm_sync: p.lfo1_bpm_sync(),
            key_sync: dec(p.lfo1_key_sync()),
            sync_note: SyncNote::from_wire(p.lfo1_sync_note_val()),
        };
        let lfo2 = Lfo {
            wave: p.lfo2_wave_val(),
            freq: U7::from_wire(p.lfo2_freq),
            bpm_sync: p.lfo2_bpm_sync(),
            key_sync: dec(p.lfo2_key_sync()),
            sync_note: SyncNote::from_wire(p.lfo2_sync_note_val()),
        };
        let patch = |s: u8, d: u8, i: u8| Patch {
            src: dec(s),
            dst: dec(d),
            int: Centered63::from_wire(i),
        };
        let ifx = &raw.insert_fx;
        let mut fx1_params = [FxParam::default(); 20];
        let mut fx2_params = [FxParam::default(); 20];
        for i in 0..20 {
            fx1_params[i] = FxParam::from_wire(ifx.fx1_params[i]);
            fx2_params[i] = FxParam::from_wire(ifx.fx2_params[i]);
        }
        let ms = &raw.motion_seq;

        Timbre {
            knob_assigns: [
                KnobAssign::from_wire(p.knob_assigns[0]),
                KnobAssign::from_wire(p.knob_assigns[1]),
                KnobAssign::from_wire(p.knob_assigns[2]),
                KnobAssign::from_wire(p.knob_assigns[3]),
            ],
            unison_voice: dec(p.unison_voice()),
            unison_detune: UnisonDetune::from_wire(p.unison_detune),
            unison_spread: U7::from_wire(p.unison_spread),
            voice_assign: dec(p.voice_assign_val()),
            analog_tuning: U7::from_wire(p.analog_tuning),
            transpose: Transpose48::from_wire(p.transpose),
            detune: Detune50::from_wire(p.detune),
            vibrato_int: Centered63::from_wire(p.vibrato_int),
            bend_range: BendRange12::from_wire(p.bend_range),
            portamento: U7::from_wire(p.portamento),
            osc1: Osc1 {
                wave: dec(p.osc1_wave()),
                osc_mod: dec(p.osc1_mod()),
                ctrl1: U7::from_wire(p.osc1_ctrl1),
                ctrl2: U7::from_wire(p.osc1_ctrl2),
                dwgs: Dwgs::from_wire(p.osc1_dwgs),
            },
            osc2: Osc2 {
                wave: dec(p.osc2_wave()),
                osc_mod: dec(p.osc2_mod()),
                semitone: Semitone24::from_wire(p.osc2_semitone),
                tune: Detune50::from_wire(p.osc2_tune),
            },
            mixer: Mixer {
                osc1_level: U7::from_wire(p.osc1_level),
                osc2_level: U7::from_wire(p.osc2_level),
                noise_level: U7::from_wire(p.noise_level),
            },
            filter: Filter {
                routing: dec(p.filter_routing()),
                filter2_type: dec(p.filter2_type()),
                balance: U7::from_wire(p.filter1_balance),
                cutoff1: U7::from_wire(p.filter1_cutoff),
                resonance1: U7::from_wire(p.filter1_resonance),
                eg1_int1: Centered63::from_wire(p.filter1_eg1_int),
                key_track1: Centered63::from_wire(p.filter1_key_track),
                velo_sens1: Centered63::from_wire(p.filter1_velo_sens),
                cutoff2: U7::from_wire(p.filter2_cutoff),
                resonance2: U7::from_wire(p.filter2_resonance),
                eg1_int2: Centered63::from_wire(p.filter2_eg1_int),
                key_track2: Centered63::from_wire(p.filter2_key_track),
                velo_sens2: Centered63::from_wire(p.filter2_velo_sens),
            },
            amp: Amp {
                level: U7::from_wire(p.amp_level),
                ws_position: dec(p.ws_position()),
                ws_type: WaveShape::from_wire(p.ws_type()),
                ws_depth: U7::from_wire(p.amp_ws_depth),
                pan: Pan::from_wire(p.amp_panpot),
                key_track: Centered63::from_wire(p.amp_key_track),
                punch_level: U7::from_wire(p.punch_level),
            },
            eg: [
                eg_at(
                    p.eg1_attack,
                    p.eg1_decay,
                    p.eg1_sustain,
                    p.eg1_release,
                    p.eg1_level_velo,
                ),
                eg_at(
                    p.eg2_attack,
                    p.eg2_decay,
                    p.eg2_sustain,
                    p.eg2_release,
                    p.eg2_level_velo,
                ),
                eg_at(
                    p.eg3_attack,
                    p.eg3_decay,
                    p.eg3_sustain,
                    p.eg3_release,
                    p.eg3_level_velo,
                ),
            ],
            lfo: [lfo1, lfo2],
            patches: [
                patch(p.patch1_src, p.patch1_dst, p.patch1_int),
                patch(p.patch2_src, p.patch2_dst, p.patch2_int),
                patch(p.patch3_src, p.patch3_dst, p.patch3_int),
                patch(p.patch4_src, p.patch4_dst, p.patch4_int),
                patch(p.patch5_src, p.patch5_dst, p.patch5_int),
                patch(p.patch6_src, p.patch6_dst, p.patch6_int),
            ],
            insert_fx: InsertFx {
                fx1_type: dec(ifx.fx1_type_val()),
                fx1_knob: ifx.fx1_knob_assign & 0x1F,
                fx1_params,
                fx2_type: dec(ifx.fx2_type_val()),
                fx2_knob: ifx.fx2_knob_assign & 0x1F,
                fx2_params,
                eq_low_freq: ifx.eq_low_freq,
                eq_low_gain: EqGain30::from_wire(ifx.eq_low_gain),
                eq_hi_freq: ifx.eq_hi_freq,
                eq_hi_gain: EqGain30::from_wire(ifx.eq_hi_gain),
            },
            motion_seq: MotionSeq {
                on: ms.seq_on(),
                seq_type: MotionSeqType::from_wire(ms.seq_type()),
                last_step: ms.last_step(),
                key_sync: dec(ms.key_sync()),
                resolution: MotionSeqResolution::from_wire(ms.resolution()),
                seq_params: ms.seq_params,
            },
        }
    }
}

impl Timbre {
    fn apply_to(&self, raw: &mut RawTimbre) {
        let t = self;
        let p = &mut raw.program;
        for i in 0..4 {
            p.knob_assigns[i] = t.knob_assigns[i].to_wire();
        }
        p.set_unison_voice(t.unison_voice.into());
        p.unison_detune = t.unison_detune.to_wire();
        p.unison_spread = t.unison_spread.to_wire();
        p.set_voice_assign_val(t.voice_assign.into());
        p.analog_tuning = t.analog_tuning.to_wire();
        p.transpose = t.transpose.to_wire();
        p.detune = t.detune.to_wire();
        p.vibrato_int = t.vibrato_int.to_wire();
        p.bend_range = t.bend_range.to_wire();
        p.portamento = t.portamento.to_wire();
        p.set_osc1_wave(t.osc1.wave.into());
        p.set_osc1_mod(t.osc1.osc_mod.into());
        p.osc1_ctrl1 = t.osc1.ctrl1.to_wire();
        p.osc1_ctrl2 = t.osc1.ctrl2.to_wire();
        p.osc1_dwgs = t.osc1.dwgs.to_wire();
        p.set_osc2_wave(t.osc2.wave.into());
        p.set_osc2_mod(t.osc2.osc_mod.into());
        p.osc2_semitone = t.osc2.semitone.to_wire();
        p.osc2_tune = t.osc2.tune.to_wire();
        p.osc1_level = t.mixer.osc1_level.to_wire();
        p.osc2_level = t.mixer.osc2_level.to_wire();
        p.noise_level = t.mixer.noise_level.to_wire();
        p.set_filter_routing(t.filter.routing.into());
        p.set_filter2_type(t.filter.filter2_type.into());
        p.filter1_balance = t.filter.balance.to_wire();
        p.filter1_cutoff = t.filter.cutoff1.to_wire();
        p.filter1_resonance = t.filter.resonance1.to_wire();
        p.filter1_eg1_int = t.filter.eg1_int1.to_wire();
        p.filter1_key_track = t.filter.key_track1.to_wire();
        p.filter1_velo_sens = t.filter.velo_sens1.to_wire();
        p.filter2_cutoff = t.filter.cutoff2.to_wire();
        p.filter2_resonance = t.filter.resonance2.to_wire();
        p.filter2_eg1_int = t.filter.eg1_int2.to_wire();
        p.filter2_key_track = t.filter.key_track2.to_wire();
        p.filter2_velo_sens = t.filter.velo_sens2.to_wire();
        p.amp_level = t.amp.level.to_wire();
        p.set_ws_position(t.amp.ws_position.into());
        p.set_ws_type(t.amp.ws_type.to_wire());
        p.amp_ws_depth = t.amp.ws_depth.to_wire();
        p.amp_panpot = t.amp.pan.to_wire();
        p.amp_key_track = t.amp.key_track.to_wire();
        p.punch_level = t.amp.punch_level.to_wire();
        let egs = &t.eg;
        p.eg1_attack = egs[0].attack.to_wire();
        p.eg1_decay = egs[0].decay.to_wire();
        p.eg1_sustain = egs[0].sustain.to_wire();
        p.eg1_release = egs[0].release.to_wire();
        p.eg1_level_velo = egs[0].level_velo.to_wire();
        p.eg2_attack = egs[1].attack.to_wire();
        p.eg2_decay = egs[1].decay.to_wire();
        p.eg2_sustain = egs[1].sustain.to_wire();
        p.eg2_release = egs[1].release.to_wire();
        p.eg2_level_velo = egs[1].level_velo.to_wire();
        p.eg3_attack = egs[2].attack.to_wire();
        p.eg3_decay = egs[2].decay.to_wire();
        p.eg3_sustain = egs[2].sustain.to_wire();
        p.eg3_release = egs[2].release.to_wire();
        p.eg3_level_velo = egs[2].level_velo.to_wire();
        p.set_lfo1_wave_val(t.lfo[0].wave);
        p.lfo1_freq = t.lfo[0].freq.to_wire();
        p.set_lfo1_bpm_sync(t.lfo[0].bpm_sync);
        p.set_lfo1_key_sync(t.lfo[0].key_sync.into());
        p.set_lfo1_sync_note_val(t.lfo[0].sync_note.to_wire());
        p.set_lfo2_wave_val(t.lfo[1].wave);
        p.lfo2_freq = t.lfo[1].freq.to_wire();
        p.set_lfo2_bpm_sync(t.lfo[1].bpm_sync);
        p.set_lfo2_key_sync(t.lfo[1].key_sync.into());
        p.set_lfo2_sync_note_val(t.lfo[1].sync_note.to_wire());
        let pat = &t.patches;
        p.patch1_src = pat[0].src.into();
        p.patch1_dst = pat[0].dst.into();
        p.patch1_int = pat[0].int.to_wire();
        p.patch2_src = pat[1].src.into();
        p.patch2_dst = pat[1].dst.into();
        p.patch2_int = pat[1].int.to_wire();
        p.patch3_src = pat[2].src.into();
        p.patch3_dst = pat[2].dst.into();
        p.patch3_int = pat[2].int.to_wire();
        p.patch4_src = pat[3].src.into();
        p.patch4_dst = pat[3].dst.into();
        p.patch4_int = pat[3].int.to_wire();
        p.patch5_src = pat[4].src.into();
        p.patch5_dst = pat[4].dst.into();
        p.patch5_int = pat[4].int.to_wire();
        p.patch6_src = pat[5].src.into();
        p.patch6_dst = pat[5].dst.into();
        p.patch6_int = pat[5].int.to_wire();

        let ifx = &mut raw.insert_fx;
        ifx.set_fx1_type_val(t.insert_fx.fx1_type.into());
        ifx.fx1_knob_assign = t.insert_fx.fx1_knob & 0x1F;
        ifx.set_fx2_type_val(t.insert_fx.fx2_type.into());
        ifx.fx2_knob_assign = t.insert_fx.fx2_knob & 0x1F;
        for i in 0..20 {
            ifx.fx1_params[i] = t.insert_fx.fx1_params[i].to_wire();
            ifx.fx2_params[i] = t.insert_fx.fx2_params[i].to_wire();
        }
        ifx.eq_low_freq = t.insert_fx.eq_low_freq;
        ifx.eq_low_gain = t.insert_fx.eq_low_gain.to_wire();
        ifx.eq_hi_freq = t.insert_fx.eq_hi_freq;
        ifx.eq_hi_gain = t.insert_fx.eq_hi_gain.to_wire();

        let ms = &mut raw.motion_seq;
        ms.set_seq_on(t.motion_seq.on);
        ms.set_seq_type(t.motion_seq.seq_type.to_wire());
        ms.set_last_step(t.motion_seq.last_step);
        ms.set_key_sync(t.motion_seq.key_sync.into());
        ms.set_resolution(t.motion_seq.resolution.to_wire());
        ms.seq_params = t.motion_seq.seq_params;
    }
}

impl From<&RawVocoder> for Vocoder {
    fn from(v: &RawVocoder) -> Self {
        let mut band_pans = [Pan::default(); 16];
        let mut band_levels = [U7::default(); 16];
        for i in 0..16 {
            band_pans[i] = Pan::from_wire(v.band_pan(i));
            band_levels[i] = U7::from_wire(v.band_level(i));
        }
        Vocoder {
            on: v.sw_on(),
            source_formant_rec: v.source() != 0,
            hpf_gate: v.hpf_gate(),
            formant_trig_reset: v.formant_data_play() != 0,
            select_timbre2: v.select() != 0,
            gate_sens: U7::from_wire(v.gate_sens),
            threshold: U7::from_wire(v.threshold),
            hpf_level: U7::from_wire(v.hpf_level),
            direct_level: U7::from_wire(v.direct_level),
            timbre1_level: U7::from_wire(v.timbre1_level),
            input1_level: U7::from_wire(v.input1_level),
            vocoder_level: U7::from_wire(v.vocoder_level),
            band_pans,
            band_levels,
            fc_mod_src: dec(v.fc_mod_src()),
            cutoff_offset: Centered63::from_wire(v.cutoff_offset),
            resonance: U7::from_wire(v.resonance),
            fc_mod_int: Centered63::from_wire(v.fc_mod_int),
            ef_sens: U7::from_wire(v.ef_sens),
        }
    }
}

impl Vocoder {
    fn apply_to(&self, raw: &mut RawVocoder) {
        let v = self;
        raw.set_sw_on(v.on);
        raw.set_source(v.source_formant_rec as u8);
        raw.set_hpf_gate(v.hpf_gate);
        raw.set_formant_data_play(v.formant_trig_reset as u8);
        raw.set_select(v.select_timbre2 as u8);
        raw.gate_sens = v.gate_sens.to_wire();
        raw.threshold = v.threshold.to_wire();
        raw.hpf_level = v.hpf_level.to_wire();
        raw.direct_level = v.direct_level.to_wire();
        raw.timbre1_level = v.timbre1_level.to_wire();
        raw.input1_level = v.input1_level.to_wire();
        raw.vocoder_level = v.vocoder_level.to_wire();
        for i in 0..16 {
            raw.set_band_pan(i, v.band_pans[i].to_wire());
            raw.set_band_level(i, v.band_levels[i].to_wire());
        }
        raw.shift_fcmodsrc = (raw.shift_fcmodsrc & 0xF0) | (u8::from(v.fc_mod_src) & 0x0F);
        raw.cutoff_offset = v.cutoff_offset.to_wire();
        raw.resonance = v.resonance.to_wire();
        raw.fc_mod_int = v.fc_mod_int.to_wire();
        raw.ef_sens = v.ef_sens.to_wire();
    }
}

impl TryFrom<RawProgram> for Program {
    type Error = String;
    fn try_from(raw: RawProgram) -> Result<Self, Self::Error> {
        let name = raw
            .name
            .iter()
            .take_while(|b| **b != 0)
            .map(|&b| b as char)
            .collect();
        let mut fx_params = [FxParam::default(); 20];
        for (param, wire) in fx_params.iter_mut().zip(raw.master_fx.params) {
            *param = FxParam::from_wire(wire);
        }
        let arp = &raw.arpeggio;
        Ok(Self {
            name,
            slot: ProgramSlot::default(),
            voice_mode: dec(raw.voice_mode()),
            arp_timbre: dec(raw.arp_timb_select()),
            vcd_knob_assigns: [
                KnobAssign::from_wire(raw.vcd_knob_assigns[0]),
                KnobAssign::from_wire(raw.vcd_knob_assigns[1]),
                KnobAssign::from_wire(raw.vcd_knob_assigns[2]),
                KnobAssign::from_wire(raw.vcd_knob_assigns[3]),
            ],
            timbre2_midi_ch: Timbre2MidiCh::try_from(raw.timbre2_midi_ch.min(16))
                .unwrap_or_default(),
            center_key: raw.center_key,
            octave_sw: raw.octave_sw(),
            category: Category::from_wire(raw.category()),
            timbre1: Timbre::from(&raw.timbre1),
            timbre2: Timbre::from(&raw.timbre2),
            vocoder: Vocoder::from(&raw.vocoder),
            master_fx: MasterFx {
                fx_type: dec(raw.master_fx.fx_type_val()),
                knob_assign: raw.master_fx.knob_assign & 0x1F,
                params: fx_params,
            },
            tempo: Tempo::from_wire(raw.tempo_raw()),
            arp: Arpeggio {
                on: raw.arp_on(),
                key_sync: raw.arp_key_sync(),
                resolution: ArpResolution::from_wire(arp.resolution()),
                arp_type: dec(arp.arp_type()),
                latch: arp.latch(),
                octave_range: arp.octave_range(),
                last_step: arp.last_step(),
                gate_time: GateTime::from_wire(arp.gate_time),
                swing: Swing50::from_wire(arp.swing),
                step_switches: arp.step_switches,
            },
            backing: Box::new(raw),
        })
    }
}

impl From<&Program> for RawProgram {
    fn from(prog: &Program) -> Self {
        let mut raw = *prog.backing;
        raw.name = str_to_8(&prog.name);
        raw.set_voice_mode(prog.voice_mode.into());
        raw.set_arp_timb_select(prog.arp_timbre.into());
        for i in 0..4 {
            raw.vcd_knob_assigns[i] = prog.vcd_knob_assigns[i].to_wire();
        }
        raw.timbre2_midi_ch = prog.timbre2_midi_ch.into();
        raw.center_key = prog.center_key;
        raw.set_octave_sw(prog.octave_sw);
        raw.set_category(prog.category.to_wire());
        prog.timbre1.apply_to(&mut raw.timbre1);
        prog.timbre2.apply_to(&mut raw.timbre2);
        prog.vocoder.apply_to(&mut raw.vocoder);
        raw.master_fx.set_fx_type_val(prog.master_fx.fx_type.into());
        raw.master_fx.knob_assign = prog.master_fx.knob_assign & 0x1F;
        for i in 0..20 {
            raw.master_fx.params[i] = prog.master_fx.params[i].to_wire();
        }
        raw.set_tempo_raw(prog.tempo.to_wire());
        raw.set_arp_on(prog.arp.on);
        raw.arpeggio.set_resolution(prog.arp.resolution.to_wire());
        raw.arpeggio.set_arp_type(prog.arp.arp_type.into());
        raw.arpeggio.gate_time = prog.arp.gate_time.to_wire();
        raw.arpeggio.swing = prog.arp.swing.to_wire();
        raw.arpeggio.step_switches = prog.arp.step_switches;
        let mut lol = prog.arp.last_step & 0x1F;
        if prog.arp.latch {
            lol |= 0x80;
        }
        lol |= (prog.arp.octave_range & 0x03) << 5;
        raw.arpeggio.latch_oct_last = lol;
        if prog.arp.key_sync {
            raw.arp_flags |= 0x40;
        } else {
            raw.arp_flags &= !0x40;
        }
        raw
    }
}

impl Program {
    pub fn blank() -> Self {
        RawProgram::default_init().try_into().unwrap()
    }
    pub fn as_bytes(&self) -> Vec<u8> {
        bytemuck::bytes_of(&RawProgram::from(self)).to_vec()
    }
    pub fn default_filename(&self) -> String {
        let safe = self.name.replace(
            |c: char| !(c.is_alphanumeric() || c == '_' || c == '-'),
            "_",
        );
        format!(
            "korg_r3-{}-{}{}.program",
            safe,
            self.slot.bank(),
            self.slot.position()
        )
    }
}
impl Default for Program {
    fn default() -> Self {
        Self::blank()
    }
}

impl RawProgram {
    fn default_init() -> Self {
        let mut raw = RawProgram::zeroed();
        raw.name = str_to_8("New Program");
        raw.center_key = 60;
        raw.timbre1 = RawTimbre::default();
        raw.timbre2 = RawTimbre::default();
        raw.vocoder = RawVocoder::default();
        raw.set_tempo_raw(120);
        raw
    }
}

fn str_to_8(s: &str) -> [u8; 8] {
    let mut buf = [0u8; 8];
    let len = s.len().min(8);
    buf[..len].copy_from_slice(&s.as_bytes()[..len]);
    buf
}

#[derive(Clone, Debug)]
pub struct Global {
    pub master_tune: MasterTune,
    pub transpose: GlobalTranspose12,
    pub velocity_curve: VelCurve,
    pub midi_channel: MidiChannelR3,
    pub midi_ctrl: [MidiCtrlNo; 3],
    pub memory_protect: bool,
    pub local_ctrl: bool,
    pub cc_map: Vec<(u8, u8)>,
    backing: Box<RawGlobal>,
}

impl Default for Global {
    fn default() -> Self {
        Self {
            master_tune: MasterTune::default(),
            transpose: GlobalTranspose12::default(),
            velocity_curve: VelCurve::default(),
            midi_channel: MidiChannelR3::default(),
            midi_ctrl: [MidiCtrlNo::default(); 3],
            memory_protect: false,
            local_ctrl: true,
            cc_map: Vec::new(),
            backing: Box::new(RawGlobal::zeroed()),
        }
    }
}

impl Global {
    pub fn as_bytes(&self) -> Vec<u8> {
        bytemuck::bytes_of(&RawGlobal::from(self)).to_vec()
    }
    pub fn default_filename(&self) -> String {
        "korg_r3.global".to_string()
    }
}

impl TryFrom<RawGlobal> for Global {
    type Error = String;
    fn try_from(raw: RawGlobal) -> Result<Self, Self::Error> {
        let mut cc_map = Vec::new();
        for (i, &b) in raw.cc_map_lo.iter().enumerate() {
            if b != 0 {
                cc_map.push((i as u8, b));
            }
        }
        for (i, &b) in raw.cc_map_mid.iter().enumerate() {
            if b != 0 {
                cc_map.push((i as u8 + 32, b));
            }
        }
        for (i, &b) in raw.cc_map_hi.iter().enumerate() {
            if b != 0 {
                cc_map.push((i as u8 + 64, b));
            }
        }
        Ok(Self {
            master_tune: MasterTune::from_wire(raw.master_tune),
            transpose: GlobalTranspose12::from_wire(raw.transpose),
            velocity_curve: VelCurve::try_from(raw.vel_curve & 0x0F).unwrap_or_default(),
            midi_channel: MidiChannelR3::try_from(raw.midi_channel & 0x0F).unwrap_or_default(),
            midi_ctrl: [
                MidiCtrlNo::from_wire(raw.midi_ctrl[0]),
                MidiCtrlNo::from_wire(raw.midi_ctrl[1]),
                MidiCtrlNo::from_wire(raw.midi_ctrl[2]),
            ],
            memory_protect: raw.flags_2 & 0x80 != 0,
            local_ctrl: raw.flags_5 & 0x80 != 0,
            cc_map,
            backing: Box::new(raw),
        })
    }
}

impl From<&Global> for RawGlobal {
    fn from(g: &Global) -> Self {
        let mut raw = *g.backing;
        raw.master_tune = g.master_tune.to_wire();
        raw.transpose = g.transpose.to_wire();
        raw.vel_curve = (raw.vel_curve & 0xF0) | (u8::from(g.velocity_curve) & 0x0F);
        raw.midi_channel = (raw.midi_channel & 0xF0) | (u8::from(g.midi_channel) & 0x0F);
        for i in 0..3 {
            raw.midi_ctrl[i] = g.midi_ctrl[i].to_wire();
        }
        if g.memory_protect {
            raw.flags_2 |= 0x80;
        } else {
            raw.flags_2 &= !0x80;
        }
        if g.local_ctrl {
            raw.flags_5 |= 0x80;
        } else {
            raw.flags_5 &= !0x80;
        }
        for &(cc, dest) in &g.cc_map {
            match cc {
                0..=31 => raw.cc_map_lo[cc as usize] = dest,
                32..=63 => raw.cc_map_mid[(cc - 32) as usize] = dest,
                64..=66 => raw.cc_map_hi[(cc - 64) as usize] = dest,
                _ => {}
            }
        }
        raw
    }
}

pub const FORMANT_BANDS: usize = 16;

pub const FORMANT_FRAME_RATE_HZ: f32 = 100.0;

pub const FORMANT_MAX_FRAMES: usize = 750;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FormantStep {
    pub bands: [u8; FORMANT_BANDS],
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FormantMotion {
    pub motion_no: Option<u8>,
    pub steps: Vec<FormantStep>,
}

impl FormantMotion {
    pub fn from_raw(motion_no: Option<u8>, steps: &[super::raw::RawFormantStep]) -> Self {
        let steps = steps
            .iter()
            .map(|s| FormantStep { bands: s.bands })
            .collect();
        Self { motion_no, steps }
    }

    pub fn is_empty(&self) -> bool {
        self.steps.is_empty()
    }

    pub fn duration_secs(&self) -> f32 {
        self.steps.len() as f32 / FORMANT_FRAME_RATE_HZ
    }

    pub fn band_envelope(&self, band: usize) -> impl Iterator<Item = u8> + '_ {
        self.steps.iter().map(move |s| s.bands[band])
    }

    pub fn to_raw(&self) -> Vec<super::raw::RawFormantStep> {
        self.steps
            .iter()
            .map(|s| super::raw::RawFormantStep { bands: s.bands })
            .collect()
    }

    pub fn size(&self) -> u16 {
        self.steps.len() as u16
    }

    pub fn as_bytes(&self) -> Vec<u8> {
        bytemuck::cast_slice(&self.to_raw()).to_vec()
    }

    pub fn from_bytes(motion_no: Option<u8>, bytes: &[u8]) -> Result<Self, String> {
        let steps: &[super::raw::RawFormantStep] =
            bytemuck::try_cast_slice(bytes).map_err(|e| e.to_string())?;
        Ok(Self::from_raw(motion_no, steps))
    }

    pub fn blank(motion_no: Option<u8>, frames: usize) -> Self {
        let frames = frames.min(FORMANT_MAX_FRAMES);
        Self {
            motion_no,
            steps: vec![
                FormantStep {
                    bands: [0; FORMANT_BANDS]
                };
                frames
            ],
        }
    }

    pub fn resize(&mut self, frames: usize) {
        let frames = frames.min(FORMANT_MAX_FRAMES);
        self.steps.resize(
            frames,
            FormantStep {
                bands: [0; FORMANT_BANDS],
            },
        );
    }

    pub fn clear_levels(&mut self) {
        for step in &mut self.steps {
            step.bands = [0; FORMANT_BANDS];
        }
    }

    pub fn band_at(&self, frame: usize, band: usize) -> u8 {
        self.steps
            .get(frame)
            .and_then(|s| s.bands.get(band).copied())
            .unwrap_or(0)
    }

    pub fn set_band(&mut self, frame: usize, band: usize, level: u8) {
        if let Some(step) = self.steps.get_mut(frame)
            && let Some(slot) = step.bands.get_mut(band)
        {
            *slot = level.min(127);
        }
    }
}

#[cfg(test)]
mod tests {
    use bytemuck::Zeroable;
    use strum::IntoEnumIterator;

    use super::*;

    #[test]
    fn formant_motion_file_round_trip() {
        let steps = vec![
            FormantStep {
                bands: [0xFF; FORMANT_BANDS],
            },
            FormantStep {
                bands: [0x80, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 0x7F],
            },
            FormantStep {
                bands: [0; FORMANT_BANDS],
            },
        ];
        let motion = FormantMotion {
            motion_no: Some(3),
            steps,
        };
        let bytes = motion.as_bytes();
        assert_eq!(bytes.len(), 3 * FORMANT_BANDS);

        let loaded = FormantMotion::from_bytes(None, &bytes).unwrap();
        assert_eq!(loaded.motion_no, None);
        assert_eq!(loaded.steps, motion.steps);
    }

    #[test]
    fn formant_motion_from_bytes_rejects_misaligned() {
        assert!(FormantMotion::from_bytes(None, &[0u8; 17]).is_err());
    }

    #[test]
    fn formant_motion_authoring() {
        let mut m = FormantMotion::blank(Some(2), 4);
        assert_eq!(m.steps.len(), 4);
        assert!(m.steps.iter().all(|s| s.bands == [0; FORMANT_BANDS]));

        m.set_band(1, 3, 200);
        assert_eq!(m.band_at(1, 3), 127);
        m.set_band(2, 0, 64);
        assert_eq!(m.band_at(2, 0), 64);

        m.resize(2);
        assert_eq!(m.steps.len(), 2);
        assert_eq!(m.band_at(1, 3), 127);
        m.resize(5);
        assert_eq!(m.steps.len(), 5);
        assert_eq!(m.band_at(4, 0), 0);

        m.clear_levels();
        assert!(m.steps.iter().all(|s| s.bands == [0; FORMANT_BANDS]));
        assert_eq!(m.steps.len(), 5);

        let mut big = FormantMotion::blank(None, FORMANT_MAX_FRAMES + 100);
        assert_eq!(big.steps.len(), FORMANT_MAX_FRAMES);
        big.resize(FORMANT_MAX_FRAMES + 50);
        assert_eq!(big.steps.len(), FORMANT_MAX_FRAMES);

        m.set_band(99, 0, 50);
        assert_eq!(m.band_at(99, 0), 0);
    }

    #[test]
    fn newtypes_clamp() {
        assert_eq!(Centered63::new(999).get(), 63);
        assert_eq!(Centered63::new(-999).get(), -63);
        assert_eq!(Centered63::from_wire(64).get(), 0);
        assert_eq!(Centered63::new(10).to_wire(), 74);
        assert_eq!(U7::new(200).get(), 127);
        assert_eq!(UnisonDetune::new(200).get(), 99);
        assert_eq!(Tempo::new(5000).to_wire(), 300);
        assert_eq!(Tempo::new(5).to_wire(), 20);
        assert_eq!(Pan::from_wire(0).get(), -63);
        assert_eq!(Pan::from_wire(64).get(), 0);
    }

    #[test]
    fn enums_only_valid_discriminants() {
        for b in 0u8..=255 {
            let _: Osc1Wave = dec(b);
            let _: PatchSource = dec(b);
            let _: PatchDest = dec(b);
            let _: FxType = dec(b);
        }
        assert_eq!(Osc1Wave::iter().count(), 9);
        assert_eq!(PatchSource::iter().count(), 12);
        assert_eq!(PatchDest::iter().count(), 15);
        assert_eq!(FxType::iter().count(), 31);
        assert_eq!(VoiceMode::iter().count(), 4);
        assert_eq!(VelCurve::iter().count(), 9);
        assert_eq!(Timbre2MidiCh::iter().count(), 17);
    }

    #[test]
    fn program_round_trip_zeroed() {
        let raw = RawProgram::zeroed();
        let prog: Program = raw.try_into().unwrap();
        let back: RawProgram = (&prog).into();
        let prog2: Program = back.try_into().unwrap();
        assert_eq!(prog.name, prog2.name);
        assert_eq!(prog.timbre1.filter.cutoff1, prog2.timbre1.filter.cutoff1);
    }

    #[test]
    fn program_byte_round_trip_preserves_meaningful_bytes() {
        let prog = Program::blank();
        let raw: RawProgram = (&prog).into();
        let prog2: Program = raw.try_into().unwrap();
        let raw2: RawProgram = (&prog2).into();
        assert_eq!(bytemuck::bytes_of(&raw), bytemuck::bytes_of(&raw2));
    }

    #[test]
    fn osc1_wave_decodes_per_spec() {
        let mut raw = RawProgram::zeroed();
        raw.timbre1.program.set_osc1_wave(6);
        let prog: Program = raw.try_into().unwrap();
        assert_eq!(prog.timbre1.osc1.wave, Osc1Wave::Dwgs);
    }

    #[test]
    fn voice_mode_split_decodes() {
        let mut raw = RawProgram::zeroed();
        raw.set_voice_mode(2);
        assert_eq!(Program::try_from(raw).unwrap().voice_mode, VoiceMode::Split);
    }

    #[test]
    fn global_round_trip() {
        let mut raw = RawGlobal::zeroed();
        raw.master_tune = 64;
        raw.transpose = 64;
        raw.midi_channel = 5;
        let g: Global = raw.try_into().unwrap();
        let back: RawGlobal = (&g).into();
        assert_eq!(back.midi_channel, 5);
        assert_eq!(back.master_tune, 64);
    }

    #[test]
    fn global_cc_map_round_trip() {
        let mut raw = RawGlobal::zeroed();
        raw.cc_map_lo[0] = 74;
        raw.cc_map_hi[2] = 100;
        let g: Global = raw.try_into().unwrap();
        assert!(g.cc_map.contains(&(0, 74)));
        assert!(g.cc_map.contains(&(66, 100)));
        let back: RawGlobal = (&g).into();
        assert_eq!(back.cc_map_lo[0], 74);
        assert_eq!(back.cc_map_hi[2], 100);
    }

    #[test]
    fn program_as_bytes_size() {
        assert_eq!(Program::blank().as_bytes().len(), size_of::<RawProgram>());
    }
    #[test]
    fn global_as_bytes_size() {
        assert_eq!(Global::default().as_bytes().len(), size_of::<RawGlobal>());
    }
    #[test]
    fn default_filename() {
        let mut p = Program::blank();
        p.name = "MySynth".into();
        p.slot = ProgramSlot::new(3);
        assert_eq!(p.default_filename(), "korg_r3-MySynth-A4.program");
    }
}
