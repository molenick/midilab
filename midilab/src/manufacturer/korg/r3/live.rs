use super::parameter_change_message;
use super::raw::RawParameterChange;
use super::wrappers::*;
use crate::sysex::unpack_u14;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ParamAddr {
    pub id: u16,
    pub sub: u16,
}

impl ParamAddr {
    const fn new(id: u16, sub: u16) -> Self {
        Self { id, sub }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LiveParam {
    pub addr: ParamAddr,
    pub value: u16,
}

impl LiveParam {
    pub fn to_sysex(&self, channel: u8) -> Vec<u8> {
        parameter_change_message(channel, self.addr.id, self.addr.sub, self.value)
    }
}

type ReadFn = Box<dyn Fn(&Program) -> u16>;
type WriteFn = Box<dyn Fn(&mut Program, u16)>;

struct Desc {
    addr: ParamAddr,
    read: ReadFn,
    write: WriteFn,
}

#[derive(Clone, Copy)]
enum Tsel {
    One,
    Two,
}
fn tref(p: &Program, s: Tsel) -> &Timbre {
    match s {
        Tsel::One => &p.timbre1,
        Tsel::Two => &p.timbre2,
    }
}
fn tmut(p: &mut Program, s: Tsel) -> &mut Timbre {
    match s {
        Tsel::One => &mut p.timbre1,
        Tsel::Two => &mut p.timbre2,
    }
}

fn d(
    id: u16,
    sub: u16,
    read: impl Fn(&Program) -> u16 + 'static,
    write: impl Fn(&mut Program, u16) + 'static,
) -> Desc {
    Desc {
        addr: ParamAddr::new(id, sub),
        read: Box::new(read),
        write: Box::new(write),
    }
}

fn push_timbre(
    out: &mut Vec<Desc>,
    sel: Tsel,
    id_prog: u16,
    id_patch: u16,
    id_ifx: u16,
    id_mhdr: u16,
) {
    for k in 0..4u16 {
        out.push(d(
            id_prog,
            k,
            move |p| tref(p, sel).knob_assigns[k as usize].to_wire() as u16,
            move |p, v| tmut(p, sel).knob_assigns[k as usize] = KnobAssign::from_wire(v as u8),
        ));
    }
    out.push(d(
        id_prog,
        0x08,
        move |p| u8::from(tref(p, sel).unison_voice) as u16,
        move |p, v| tmut(p, sel).unison_voice = UnisonVoice::try_from(v as u8).unwrap_or_default(),
    ));
    out.push(d(
        id_prog,
        0x09,
        move |p| tref(p, sel).unison_detune.to_wire() as u16,
        move |p, v| tmut(p, sel).unison_detune = UnisonDetune::from_wire(v as u8),
    ));
    out.push(d(
        id_prog,
        0x0A,
        move |p| tref(p, sel).unison_spread.to_wire() as u16,
        move |p, v| tmut(p, sel).unison_spread = U7::from_wire(v as u8),
    ));
    out.push(d(
        id_prog,
        0x0B,
        move |p| u8::from(tref(p, sel).voice_assign) as u16,
        move |p, v| tmut(p, sel).voice_assign = VoiceAssign::try_from(v as u8).unwrap_or_default(),
    ));
    out.push(d(
        id_prog,
        0x13,
        move |p| tref(p, sel).analog_tuning.to_wire() as u16,
        move |p, v| tmut(p, sel).analog_tuning = U7::from_wire(v as u8),
    ));
    out.push(d(
        id_prog,
        0x14,
        move |p| tref(p, sel).transpose.to_wire() as u16,
        move |p, v| tmut(p, sel).transpose = Transpose48::from_wire(v as u8),
    ));
    out.push(d(
        id_prog,
        0x15,
        move |p| tref(p, sel).detune.to_wire() as u16,
        move |p, v| tmut(p, sel).detune = Detune50::from_wire(v as u8),
    ));
    out.push(d(
        id_prog,
        0x16,
        move |p| tref(p, sel).vibrato_int.to_wire() as u16,
        move |p, v| tmut(p, sel).vibrato_int = Centered63::from_wire(v as u8),
    ));
    out.push(d(
        id_prog,
        0x10,
        move |p| tref(p, sel).bend_range.to_wire() as u16,
        move |p, v| tmut(p, sel).bend_range = BendRange12::from_wire(v as u8),
    ));
    out.push(d(
        id_prog,
        0x11,
        move |p| tref(p, sel).portamento.to_wire() as u16,
        move |p, v| tmut(p, sel).portamento = U7::from_wire(v as u8),
    ));
    out.push(d(
        id_prog,
        0x17,
        move |p| u8::from(tref(p, sel).osc1.wave) as u16,
        move |p, v| tmut(p, sel).osc1.wave = Osc1Wave::try_from(v as u8).unwrap_or_default(),
    ));
    out.push(d(
        id_prog,
        0x18,
        move |p| u8::from(tref(p, sel).osc1.osc_mod) as u16,
        move |p, v| tmut(p, sel).osc1.osc_mod = OscMod::try_from(v as u8).unwrap_or_default(),
    ));
    out.push(d(
        id_prog,
        0x19,
        move |p| tref(p, sel).osc1.ctrl1.to_wire() as u16,
        move |p, v| tmut(p, sel).osc1.ctrl1 = U7::from_wire(v as u8),
    ));
    out.push(d(
        id_prog,
        0x1A,
        move |p| tref(p, sel).osc1.ctrl2.to_wire() as u16,
        move |p, v| tmut(p, sel).osc1.ctrl2 = U7::from_wire(v as u8),
    ));
    out.push(d(
        id_prog,
        0x1B,
        move |p| tref(p, sel).osc1.dwgs.to_wire() as u16,
        move |p, v| tmut(p, sel).osc1.dwgs = Dwgs::from_wire(v as u8),
    ));
    out.push(d(
        id_prog,
        0x20,
        move |p| u8::from(tref(p, sel).osc2.wave) as u16,
        move |p, v| tmut(p, sel).osc2.wave = Osc2Wave::try_from(v as u8).unwrap_or_default(),
    ));
    out.push(d(
        id_prog,
        0x21,
        move |p| u8::from(tref(p, sel).osc2.osc_mod) as u16,
        move |p, v| tmut(p, sel).osc2.osc_mod = Osc2Mod::try_from(v as u8).unwrap_or_default(),
    ));
    out.push(d(
        id_prog,
        0x22,
        move |p| tref(p, sel).osc2.semitone.to_wire() as u16,
        move |p, v| tmut(p, sel).osc2.semitone = Semitone24::from_wire(v as u8),
    ));
    out.push(d(
        id_prog,
        0x23,
        move |p| tref(p, sel).osc2.tune.to_wire() as u16,
        move |p, v| tmut(p, sel).osc2.tune = Detune50::from_wire(v as u8),
    ));
    out.push(d(
        id_prog,
        0x28,
        move |p| tref(p, sel).mixer.osc1_level.to_wire() as u16,
        move |p, v| tmut(p, sel).mixer.osc1_level = U7::from_wire(v as u8),
    ));
    out.push(d(
        id_prog,
        0x29,
        move |p| tref(p, sel).mixer.osc2_level.to_wire() as u16,
        move |p, v| tmut(p, sel).mixer.osc2_level = U7::from_wire(v as u8),
    ));
    out.push(d(
        id_prog,
        0x2A,
        move |p| tref(p, sel).mixer.noise_level.to_wire() as u16,
        move |p, v| tmut(p, sel).mixer.noise_level = U7::from_wire(v as u8),
    ));
    out.push(d(
        id_prog,
        0x30,
        move |p| u8::from(tref(p, sel).filter.routing) as u16,
        move |p, v| {
            tmut(p, sel).filter.routing = FilterRouting::try_from(v as u8).unwrap_or_default()
        },
    ));
    out.push(d(
        id_prog,
        0x40,
        move |p| u8::from(tref(p, sel).filter.filter2_type) as u16,
        move |p, v| {
            tmut(p, sel).filter.filter2_type = Filter2Type::try_from(v as u8).unwrap_or_default()
        },
    ));
    out.push(d(
        id_prog,
        0x31,
        move |p| tref(p, sel).filter.balance.to_wire() as u16,
        move |p, v| tmut(p, sel).filter.balance = U7::from_wire(v as u8),
    ));
    out.push(d(
        id_prog,
        0x32,
        move |p| tref(p, sel).filter.cutoff1.to_wire() as u16,
        move |p, v| tmut(p, sel).filter.cutoff1 = U7::from_wire(v as u8),
    ));
    out.push(d(
        id_prog,
        0x33,
        move |p| tref(p, sel).filter.resonance1.to_wire() as u16,
        move |p, v| tmut(p, sel).filter.resonance1 = U7::from_wire(v as u8),
    ));
    out.push(d(
        id_prog,
        0x34,
        move |p| tref(p, sel).filter.eg1_int1.to_wire() as u16,
        move |p, v| tmut(p, sel).filter.eg1_int1 = Centered63::from_wire(v as u8),
    ));
    out.push(d(
        id_prog,
        0x35,
        move |p| tref(p, sel).filter.key_track1.to_wire() as u16,
        move |p, v| tmut(p, sel).filter.key_track1 = Centered63::from_wire(v as u8),
    ));
    out.push(d(
        id_prog,
        0x36,
        move |p| tref(p, sel).filter.velo_sens1.to_wire() as u16,
        move |p, v| tmut(p, sel).filter.velo_sens1 = Centered63::from_wire(v as u8),
    ));
    out.push(d(
        id_prog,
        0x42,
        move |p| tref(p, sel).filter.cutoff2.to_wire() as u16,
        move |p, v| tmut(p, sel).filter.cutoff2 = U7::from_wire(v as u8),
    ));
    out.push(d(
        id_prog,
        0x43,
        move |p| tref(p, sel).filter.resonance2.to_wire() as u16,
        move |p, v| tmut(p, sel).filter.resonance2 = U7::from_wire(v as u8),
    ));
    out.push(d(
        id_prog,
        0x44,
        move |p| tref(p, sel).filter.eg1_int2.to_wire() as u16,
        move |p, v| tmut(p, sel).filter.eg1_int2 = Centered63::from_wire(v as u8),
    ));
    out.push(d(
        id_prog,
        0x45,
        move |p| tref(p, sel).filter.key_track2.to_wire() as u16,
        move |p, v| tmut(p, sel).filter.key_track2 = Centered63::from_wire(v as u8),
    ));
    out.push(d(
        id_prog,
        0x46,
        move |p| tref(p, sel).filter.velo_sens2.to_wire() as u16,
        move |p, v| tmut(p, sel).filter.velo_sens2 = Centered63::from_wire(v as u8),
    ));
    out.push(d(
        id_prog,
        0x50,
        move |p| tref(p, sel).amp.level.to_wire() as u16,
        move |p, v| tmut(p, sel).amp.level = U7::from_wire(v as u8),
    ));
    out.push(d(
        id_prog,
        0x52,
        move |p| u8::from(tref(p, sel).amp.ws_position) as u16,
        move |p, v| {
            tmut(p, sel).amp.ws_position = WaveShapePosition::try_from(v as u8).unwrap_or_default()
        },
    ));
    out.push(d(
        id_prog,
        0x51,
        move |p| tref(p, sel).amp.ws_type.to_wire() as u16,
        move |p, v| tmut(p, sel).amp.ws_type = WaveShape::from_wire(v as u8),
    ));
    out.push(d(
        id_prog,
        0x54,
        move |p| tref(p, sel).amp.ws_depth.to_wire() as u16,
        move |p, v| tmut(p, sel).amp.ws_depth = U7::from_wire(v as u8),
    ));
    out.push(d(
        id_prog,
        0x55,
        move |p| tref(p, sel).amp.pan.to_wire() as u16,
        move |p, v| tmut(p, sel).amp.pan = Pan::from_wire(v as u8),
    ));
    out.push(d(
        id_prog,
        0x56,
        move |p| tref(p, sel).amp.key_track.to_wire() as u16,
        move |p, v| tmut(p, sel).amp.key_track = Centered63::from_wire(v as u8),
    ));
    out.push(d(
        id_prog,
        0x57,
        move |p| tref(p, sel).amp.punch_level.to_wire() as u16,
        move |p, v| tmut(p, sel).amp.punch_level = U7::from_wire(v as u8),
    ));
    for (egi, base) in [(0usize, 0x60u16), (1, 0x70), (2, 0x80)] {
        out.push(d(
            id_prog,
            base,
            move |p| tref(p, sel).eg[egi].attack.to_wire() as u16,
            move |p, v| tmut(p, sel).eg[egi].attack = U7::from_wire(v as u8),
        ));
        out.push(d(
            id_prog,
            base + 1,
            move |p| tref(p, sel).eg[egi].decay.to_wire() as u16,
            move |p, v| tmut(p, sel).eg[egi].decay = U7::from_wire(v as u8),
        ));
        out.push(d(
            id_prog,
            base + 2,
            move |p| tref(p, sel).eg[egi].sustain.to_wire() as u16,
            move |p, v| tmut(p, sel).eg[egi].sustain = U7::from_wire(v as u8),
        ));
        out.push(d(
            id_prog,
            base + 3,
            move |p| tref(p, sel).eg[egi].release.to_wire() as u16,
            move |p, v| tmut(p, sel).eg[egi].release = U7::from_wire(v as u8),
        ));
        out.push(d(
            id_prog,
            base + 4,
            move |p| tref(p, sel).eg[egi].level_velo.to_wire() as u16,
            move |p, v| tmut(p, sel).eg[egi].level_velo = Centered63::from_wire(v as u8),
        ));
    }
    for (lfi, base) in [(0usize, 0x90u16), (1, 0xA0)] {
        out.push(d(
            id_prog,
            base,
            move |p| tref(p, sel).lfo[lfi].wave as u16,
            move |p, v| tmut(p, sel).lfo[lfi].wave = v as u8,
        ));
        out.push(d(
            id_prog,
            base + 2,
            move |p| tref(p, sel).lfo[lfi].freq.to_wire() as u16,
            move |p, v| tmut(p, sel).lfo[lfi].freq = U7::from_wire(v as u8),
        ));
        out.push(d(
            id_prog,
            base + 3,
            move |p| tref(p, sel).lfo[lfi].bpm_sync as u16,
            move |p, v| tmut(p, sel).lfo[lfi].bpm_sync = v != 0,
        ));
        out.push(d(
            id_prog,
            base + 4,
            move |p| u8::from(tref(p, sel).lfo[lfi].key_sync) as u16,
            move |p, v| {
                tmut(p, sel).lfo[lfi].key_sync = LfoKeySync::try_from(v as u8).unwrap_or_default()
            },
        ));
        out.push(d(
            id_prog,
            base + 6,
            move |p| tref(p, sel).lfo[lfi].sync_note.to_wire() as u16,
            move |p, v| tmut(p, sel).lfo[lfi].sync_note = SyncNote::from_wire(v as u8),
        ));
    }
    for i in 0..6usize {
        let s = (4 * i) as u16;
        out.push(d(
            id_patch,
            s,
            move |p| u8::from(tref(p, sel).patches[i].src) as u16,
            move |p, v| {
                tmut(p, sel).patches[i].src = PatchSource::try_from(v as u8).unwrap_or_default()
            },
        ));
        out.push(d(
            id_patch,
            s + 1,
            move |p| u8::from(tref(p, sel).patches[i].dst) as u16,
            move |p, v| {
                tmut(p, sel).patches[i].dst = PatchDest::try_from(v as u8).unwrap_or_default()
            },
        ));
        out.push(d(
            id_patch,
            s + 2,
            move |p| tref(p, sel).patches[i].int.to_wire() as u16,
            move |p, v| tmut(p, sel).patches[i].int = Centered63::from_wire(v as u8),
        ));
    }
    out.push(d(
        id_ifx,
        0x01,
        move |p| u8::from(tref(p, sel).insert_fx.fx1_type) as u16,
        move |p, v| tmut(p, sel).insert_fx.fx1_type = FxType::try_from(v as u8).unwrap_or_default(),
    ));
    out.push(d(
        id_ifx,
        0x02,
        move |p| tref(p, sel).insert_fx.fx1_knob as u16,
        move |p, v| tmut(p, sel).insert_fx.fx1_knob = (v as u8) & 0x1F,
    ));
    out.push(d(
        id_ifx,
        0x31,
        move |p| u8::from(tref(p, sel).insert_fx.fx2_type) as u16,
        move |p, v| tmut(p, sel).insert_fx.fx2_type = FxType::try_from(v as u8).unwrap_or_default(),
    ));
    out.push(d(
        id_ifx,
        0x32,
        move |p| tref(p, sel).insert_fx.fx2_knob as u16,
        move |p, v| tmut(p, sel).insert_fx.fx2_knob = (v as u8) & 0x1F,
    ));
    for i in 0..20usize {
        out.push(d(
            id_ifx,
            0x10 + i as u16,
            move |p| tref(p, sel).insert_fx.fx1_params[i].to_wire() as u16,
            move |p, v| tmut(p, sel).insert_fx.fx1_params[i] = FxParam::from_wire(v as u8),
        ));
        out.push(d(
            id_ifx,
            0x40 + i as u16,
            move |p| tref(p, sel).insert_fx.fx2_params[i].to_wire() as u16,
            move |p, v| tmut(p, sel).insert_fx.fx2_params[i] = FxParam::from_wire(v as u8),
        ));
    }
    out.push(d(
        id_ifx,
        0x60,
        move |p| tref(p, sel).insert_fx.eq_low_freq as u16,
        move |p, v| tmut(p, sel).insert_fx.eq_low_freq = v as u8,
    ));
    out.push(d(
        id_ifx,
        0x61,
        move |p| tref(p, sel).insert_fx.eq_low_gain.to_wire() as u16,
        move |p, v| tmut(p, sel).insert_fx.eq_low_gain = EqGain30::from_wire(v as u8),
    ));
    out.push(d(
        id_ifx,
        0x62,
        move |p| tref(p, sel).insert_fx.eq_hi_freq as u16,
        move |p, v| tmut(p, sel).insert_fx.eq_hi_freq = v as u8,
    ));
    out.push(d(
        id_ifx,
        0x63,
        move |p| tref(p, sel).insert_fx.eq_hi_gain.to_wire() as u16,
        move |p, v| tmut(p, sel).insert_fx.eq_hi_gain = EqGain30::from_wire(v as u8),
    ));
    out.push(d(
        id_mhdr,
        0x00,
        move |p| tref(p, sel).motion_seq.on as u16,
        move |p, v| tmut(p, sel).motion_seq.on = v != 0,
    ));
    out.push(d(
        id_mhdr,
        0x01,
        move |p| tref(p, sel).motion_seq.last_step as u16,
        move |p, v| tmut(p, sel).motion_seq.last_step = v as u8,
    ));
    out.push(d(
        id_mhdr,
        0x02,
        move |p| tref(p, sel).motion_seq.seq_type.to_wire() as u16,
        move |p, v| tmut(p, sel).motion_seq.seq_type = MotionSeqType::from_wire(v as u8),
    ));
    out.push(d(
        id_mhdr,
        0x04,
        move |p| u8::from(tref(p, sel).motion_seq.key_sync) as u16,
        move |p, v| {
            tmut(p, sel).motion_seq.key_sync = LfoKeySync::try_from(v as u8).unwrap_or_default()
        },
    ));
    out.push(d(
        id_mhdr,
        0x05,
        move |p| tref(p, sel).motion_seq.resolution.to_wire() as u16,
        move |p, v| tmut(p, sel).motion_seq.resolution = MotionSeqResolution::from_wire(v as u8),
    ));
}

fn push_program_global(out: &mut Vec<Desc>) {
    out.push(d(
        0x00,
        0x19,
        |p| u8::from(p.voice_mode) as u16,
        |p, v| p.voice_mode = VoiceMode::try_from(v as u8).unwrap_or_default(),
    ));
    out.push(d(
        0x00,
        0x18,
        |p| u8::from(p.arp_timbre) as u16,
        |p, v| p.arp_timbre = ArpTimbre::try_from(v as u8).unwrap_or_default(),
    ));
    for k in 0..4u16 {
        out.push(d(
            0x00,
            0x14 + k,
            move |p| p.vcd_knob_assigns[k as usize].to_wire() as u16,
            move |p, v| p.vcd_knob_assigns[k as usize] = KnobAssign::from_wire(v as u8),
        ));
    }
    out.push(d(
        0x00,
        0x1A,
        |p| u8::from(p.timbre2_midi_ch) as u16,
        |p, v| p.timbre2_midi_ch = Timbre2MidiCh::try_from((v as u8).min(16)).unwrap_or_default(),
    ));
    out.push(d(
        0x00,
        0x1B,
        |p| p.center_key as u16,
        |p, v| p.center_key = v as u8,
    ));
    out.push(d(
        0x00,
        0x1C,
        |p| p.category.to_wire() as u16,
        |p, v| p.category = Category::from_wire(v as u8),
    ));

    out.push(d(
        0x60,
        0x00,
        |p| p.tempo.to_wire(),
        |p, v| p.tempo = Tempo::from_wire(v),
    ));
    out.push(d(0x60, 0x01, |p| p.arp.on as u16, |p, v| p.arp.on = v != 0));
    out.push(d(
        0x60,
        0x02,
        |p| p.arp.key_sync as u16,
        |p, v| p.arp.key_sync = v != 0,
    ));

    out.push(d(
        0x61,
        0x00,
        |p| u8::from(p.arp.arp_type) as u16,
        |p, v| p.arp.arp_type = ArpType::try_from(v as u8).unwrap_or_default(),
    ));
    out.push(d(
        0x61,
        0x01,
        |p| p.arp.resolution.to_wire() as u16,
        |p, v| p.arp.resolution = ArpResolution::from_wire(v as u8),
    ));
    out.push(d(
        0x61,
        0x02,
        |p| p.arp.octave_range as u16,
        |p, v| p.arp.octave_range = (v as u8) & 0x03,
    ));
    out.push(d(
        0x61,
        0x03,
        |p| p.arp.last_step as u16,
        |p, v| p.arp.last_step = (v as u8) & 0x07,
    ));
    out.push(d(
        0x61,
        0x04,
        |p| p.arp.gate_time.to_wire() as u16,
        |p, v| p.arp.gate_time = GateTime::from_wire(v as u8),
    ));
    out.push(d(
        0x61,
        0x05,
        |p| p.arp.swing.to_wire() as u16,
        |p, v| p.arp.swing = Swing50::from_wire(v as u8),
    ));
    out.push(d(
        0x61,
        0x06,
        |p| p.arp.latch as u16,
        |p, v| p.arp.latch = v != 0,
    ));
    for i in 0..8u16 {
        let bit = 7 - i as u8;
        out.push(d(
            0x61,
            0x10 + i,
            move |p| ((p.arp.step_switches >> bit) & 1) as u16,
            move |p, v| {
                let mask = 1u8 << bit;
                if v != 0 {
                    p.arp.step_switches |= mask
                } else {
                    p.arp.step_switches &= !mask
                }
            },
        ));
    }

    out.push(d(
        0x50,
        0x01,
        |p| u8::from(p.master_fx.fx_type) as u16,
        |p, v| p.master_fx.fx_type = FxType::try_from(v as u8).unwrap_or_default(),
    ));
    out.push(d(
        0x50,
        0x02,
        |p| p.master_fx.knob_assign as u16,
        |p, v| p.master_fx.knob_assign = (v as u8) & 0x1F,
    ));
    for i in 0..20usize {
        out.push(d(
            0x51,
            i as u16,
            move |p| p.master_fx.params[i].to_wire() as u16,
            move |p, v| p.master_fx.params[i] = FxParam::from_wire(v as u8),
        ));
    }

    out.push(d(
        0x40,
        0x00,
        |p| p.vocoder.on as u16,
        |p, v| p.vocoder.on = v != 0,
    ));
    out.push(d(
        0x40,
        0x05,
        |p| p.vocoder.source_formant_rec as u16,
        |p, v| p.vocoder.source_formant_rec = v != 0,
    ));
    out.push(d(
        0x40,
        0x04,
        |p| p.vocoder.hpf_gate as u16,
        |p, v| p.vocoder.hpf_gate = v != 0,
    ));
    out.push(d(
        0x40,
        0x03,
        |p| p.vocoder.formant_trig_reset as u16,
        |p, v| p.vocoder.formant_trig_reset = v != 0,
    ));
    out.push(d(
        0x40,
        0x01,
        |p| p.vocoder.select_timbre2 as u16,
        |p, v| p.vocoder.select_timbre2 = v != 0,
    ));
    out.push(d(
        0x40,
        0x06,
        |p| p.vocoder.gate_sens.to_wire() as u16,
        |p, v| p.vocoder.gate_sens = U7::from_wire(v as u8),
    ));
    out.push(d(
        0x40,
        0x07,
        |p| p.vocoder.threshold.to_wire() as u16,
        |p, v| p.vocoder.threshold = U7::from_wire(v as u8),
    ));
    out.push(d(
        0x40,
        0x08,
        |p| p.vocoder.hpf_level.to_wire() as u16,
        |p, v| p.vocoder.hpf_level = U7::from_wire(v as u8),
    ));
    out.push(d(
        0x40,
        0x09,
        |p| p.vocoder.direct_level.to_wire() as u16,
        |p, v| p.vocoder.direct_level = U7::from_wire(v as u8),
    ));
    out.push(d(
        0x40,
        0x0A,
        |p| p.vocoder.timbre1_level.to_wire() as u16,
        |p, v| p.vocoder.timbre1_level = U7::from_wire(v as u8),
    ));
    out.push(d(
        0x40,
        0x0B,
        |p| p.vocoder.input1_level.to_wire() as u16,
        |p, v| p.vocoder.input1_level = U7::from_wire(v as u8),
    ));
    out.push(d(
        0x40,
        0x0C,
        |p| p.vocoder.vocoder_level.to_wire() as u16,
        |p, v| p.vocoder.vocoder_level = U7::from_wire(v as u8),
    ));
    for i in 0..16usize {
        let s = (2 * i) as u16;
        out.push(d(
            0x41,
            s,
            move |p| p.vocoder.band_pans[i].to_wire() as u16,
            move |p, v| p.vocoder.band_pans[i] = Pan::from_wire(v as u8),
        ));
        out.push(d(
            0x41,
            s + 1,
            move |p| p.vocoder.band_levels[i].to_wire() as u16,
            move |p, v| p.vocoder.band_levels[i] = U7::from_wire(v as u8),
        ));
    }
    out.push(d(
        0x42,
        0x00,
        |p| u8::from(p.vocoder.fc_mod_src) as u16,
        |p, v| p.vocoder.fc_mod_src = PatchSource::try_from(v as u8).unwrap_or_default(),
    ));
    out.push(d(
        0x42,
        0x10,
        |p| p.vocoder.cutoff_offset.to_wire() as u16,
        |p, v| p.vocoder.cutoff_offset = Centered63::from_wire(v as u8),
    ));
    out.push(d(
        0x42,
        0x11,
        |p| p.vocoder.resonance.to_wire() as u16,
        |p, v| p.vocoder.resonance = U7::from_wire(v as u8),
    ));
    out.push(d(
        0x42,
        0x12,
        |p| p.vocoder.fc_mod_int.to_wire() as u16,
        |p, v| p.vocoder.fc_mod_int = Centered63::from_wire(v as u8),
    ));
    out.push(d(
        0x42,
        0x13,
        |p| p.vocoder.ef_sens.to_wire() as u16,
        |p, v| p.vocoder.ef_sens = U7::from_wire(v as u8),
    ));
}

fn descriptors() -> Vec<Desc> {
    let mut out = Vec::new();
    push_program_global(&mut out);
    push_timbre(&mut out, Tsel::One, 0x10, 0x11, 0x13, 0x14);
    push_timbre(&mut out, Tsel::Two, 0x20, 0x21, 0x23, 0x24);
    out
}

pub fn program_diff(old: &Program, new: &Program) -> Vec<LiveParam> {
    descriptors()
        .iter()
        .filter_map(|desc| {
            let nv = (desc.read)(new);
            if (desc.read)(old) != nv {
                Some(LiveParam {
                    addr: desc.addr,
                    value: nv,
                })
            } else {
                None
            }
        })
        .collect()
}

pub fn program_params(prog: &Program) -> Vec<LiveParam> {
    descriptors()
        .iter()
        .map(|desc| LiveParam {
            addr: desc.addr,
            value: (desc.read)(prog),
        })
        .collect()
}

pub fn apply_parameter_change(prog: &mut Program, raw: &RawParameterChange) -> bool {
    let id = unpack_u14([raw.param_id[1], raw.param_id[0]]);
    let sub = unpack_u14([raw.sub_id[1], raw.sub_id[0]]);
    let val = unpack_u14([raw.value[1], raw.value[0]]);
    if let Some(desc) = descriptors()
        .iter()
        .find(|d| d.addr.id == id && d.addr.sub == sub)
    {
        (desc.write)(prog, val);
        true
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn prog() -> Program {
        Program::blank()
    }

    #[test]
    fn no_diff_for_identical_programs() {
        assert!(program_diff(&prog(), &prog()).is_empty());
    }

    #[test]
    fn diff_emits_exactly_one_param_for_one_field() {
        let a = prog();
        let mut b = a.clone();
        b.timbre1.filter.cutoff1 = U7::new(99);
        let changes = program_diff(&a, &b);
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].addr, ParamAddr::new(0x10, 0x32));
        assert_eq!(changes[0].value, 99);
    }

    #[test]
    fn timbre2_uses_0x2x_id_base() {
        let a = prog();
        let mut b = a.clone();
        b.timbre2.filter.cutoff1 = U7::new(42);
        let changes = program_diff(&a, &b);
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].addr, ParamAddr::new(0x20, 0x32));
    }

    #[test]
    fn centered_param_travels_64_centered() {
        let a = prog();
        let mut b = a.clone();
        b.timbre1.filter.eg1_int1 = Centered63::new(0);
        b.timbre1.filter.eg1_int1 = Centered63::new(10);
        let changes = program_diff(&a, &b);
        let c = changes
            .iter()
            .find(|c| c.addr == ParamAddr::new(0x10, 0x34))
            .unwrap();
        assert_eq!(c.value, 74);
    }

    #[test]
    fn sysex_bytes_are_well_formed() {
        let lp = LiveParam {
            addr: ParamAddr::new(0x10, 0x32),
            value: 99,
        };
        let msg = lp.to_sysex(0x00);
        assert_eq!(&msg[0..5], &[0xF0, 0x42, 0x30, 0x7D, 0x41]);
        assert_eq!(&msg[5..11], &[0x10, 0x00, 0x32, 0x00, 0x63, 0x00]);
        assert_eq!(*msg.last().unwrap(), 0xF7);
    }

    #[test]
    fn tempo_is_14_bit() {
        let a = prog();
        let mut b = a.clone();
        b.tempo = Tempo::new(200);
        let changes = program_diff(&a, &b);
        let c = changes
            .iter()
            .find(|c| c.addr == ParamAddr::new(0x60, 0x00))
            .unwrap();
        assert_eq!(c.value, 200);
        let msg = c.to_sysex(0);
        assert_eq!(&msg[9..11], &[0x48, 0x01]);
    }

    #[test]
    fn apply_inverts_emit_for_all_params() {
        use crate::manufacturer::korg::r3::KorgR3Message;

        let mut src = prog();
        src.timbre1.filter.cutoff1 = U7::new(120);
        src.timbre1.osc1.wave = Osc1Wave::Dwgs;
        src.timbre1.amp.pan = Pan::new(-40);
        src.timbre2.eg[1].attack = U7::new(77);
        src.vocoder.band_levels[5] = U7::new(33);
        src.master_fx.params[3] = FxParam::new(64);
        src.arp.swing = Swing50::new(-20);
        src.tempo = Tempo::new(176);

        for lp in program_params(&src) {
            let bytes = lp.to_sysex(0x00);
            let parsed = KorgR3Message::try_from(bytes.as_slice()).expect("valid sysex");
            let raw = match parsed {
                KorgR3Message::ParameterChange(r) => r,
                other => panic!("expected ParameterChange, got {other:?}"),
            };
            let mut dst = prog();
            assert!(
                apply_parameter_change(&mut dst, &raw),
                "addr {:?} unmapped",
                lp.addr
            );
            let desc = descriptors();
            let d = desc.iter().find(|d| d.addr == lp.addr).unwrap();
            assert_eq!(
                (d.read)(&dst),
                lp.value,
                "round-trip mismatch at {:?}",
                lp.addr
            );
        }
    }

    #[test]
    fn unmapped_address_is_ignored() {
        let mut p = prog();
        let raw = match crate::manufacturer::korg::r3::KorgR3Message::try_from(
            LiveParam {
                addr: ParamAddr::new(0x7F, 0x7F),
                value: 1,
            }
            .to_sysex(0)
            .as_slice(),
        )
        .unwrap()
        {
            crate::manufacturer::korg::r3::KorgR3Message::ParameterChange(r) => r,
            _ => unreachable!(),
        };
        assert!(!apply_parameter_change(&mut p, &raw));
    }
}
