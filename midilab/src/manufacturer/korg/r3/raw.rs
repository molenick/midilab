use bytemuck::Pod;
use bytemuck::Zeroable;

#[repr(C)]
#[derive(Debug, Copy, Clone, Pod, Zeroable)]
pub struct RawTimbreProgram {
    pub knob_assigns: [u8; 4],
    pub voice_unison: u8,
    pub unison_detune: u8,
    pub unison_spread: u8,
    pub voice_assign: u8,
    pub _dummy8: u8,
    pub analog_tuning: u8,
    pub transpose: u8,
    pub detune: u8,
    pub vibrato_int: u8,
    pub bend_range: u8,
    pub portamento: u8,
    pub _dummy15: u8,
    pub osc1_wave_mod: u8,
    pub osc1_ctrl1: u8,
    pub osc1_ctrl2: u8,
    pub osc1_dwgs: u8,
    pub _dummy20: u8,
    pub osc2_wave_mod: u8,
    pub osc2_semitone: u8,
    pub osc2_tune: u8,
    pub osc1_level: u8,
    pub osc2_level: u8,
    pub noise_level: u8,
    pub _dummy27: u8,
    pub filter_routing_type2: u8,
    pub filter1_balance: u8,
    pub filter1_cutoff: u8,
    pub filter1_resonance: u8,
    pub filter1_eg1_int: u8,
    pub filter1_key_track: u8,
    pub filter1_velo_sens: u8,
    pub filter2_cutoff: u8,
    pub filter2_resonance: u8,
    pub filter2_eg1_int: u8,
    pub filter2_key_track: u8,
    pub filter2_velo_sens: u8,
    pub amp_level: u8,
    pub amp_ws_position: u8,
    pub amp_ws_type: u8,
    pub amp_ws_depth: u8,
    pub amp_panpot: u8,
    pub amp_key_track: u8,
    pub punch_level: u8,
    pub _dummy47: u8,
    pub eg1_attack: u8,
    pub eg1_decay: u8,
    pub eg1_sustain: u8,
    pub eg1_release: u8,
    pub eg1_level_velo: u8,
    pub _dummy53: u8,
    pub eg2_attack: u8,
    pub eg2_decay: u8,
    pub eg2_sustain: u8,
    pub eg2_release: u8,
    pub eg2_level_velo: u8,
    pub _dummy59: u8,
    pub eg3_attack: u8,
    pub eg3_decay: u8,
    pub eg3_sustain: u8,
    pub eg3_release: u8,
    pub eg3_level_velo: u8,
    pub _dummy65: u8,
    pub lfo1_wave: u8,
    pub lfo1_freq: u8,
    pub lfo1_sync: u8,
    pub lfo1_sync_note: u8,
    pub lfo2_wave: u8,
    pub lfo2_freq: u8,
    pub lfo2_sync: u8,
    pub lfo2_sync_note: u8,
    pub patch1_src: u8,
    pub patch1_dst: u8,
    pub patch1_int: u8,
    pub patch2_src: u8,
    pub patch2_dst: u8,
    pub patch2_int: u8,
    pub patch3_src: u8,
    pub patch3_dst: u8,
    pub patch3_int: u8,
    pub patch4_src: u8,
    pub patch4_dst: u8,
    pub patch4_int: u8,
    pub patch5_src: u8,
    pub patch5_dst: u8,
    pub patch5_int: u8,
    pub patch6_src: u8,
    pub patch6_dst: u8,
    pub patch6_int: u8,
}

impl RawTimbreProgram {
    pub fn unison_voice(&self) -> u8 {
        self.voice_unison & 0x0F
    }
    pub fn set_unison_voice(&mut self, v: u8) {
        self.voice_unison = (self.voice_unison & !0x0F) | (v & 0x0F);
    }
    pub fn voice_assign_val(&self) -> u8 {
        (self.voice_assign >> 6) & 0x03
    }
    pub fn set_voice_assign_val(&mut self, v: u8) {
        self.voice_assign = (self.voice_assign & !0xC0) | ((v & 0x03) << 6);
    }
    pub fn osc1_wave(&self) -> u8 {
        self.osc1_wave_mod & 0x0F
    }
    pub fn set_osc1_wave(&mut self, v: u8) {
        self.osc1_wave_mod = (self.osc1_wave_mod & !0x0F) | (v & 0x0F);
    }
    pub fn osc1_mod(&self) -> u8 {
        (self.osc1_wave_mod >> 4) & 0x03
    }
    pub fn set_osc1_mod(&mut self, v: u8) {
        self.osc1_wave_mod = (self.osc1_wave_mod & !0x30) | ((v & 0x03) << 4);
    }
    pub fn osc2_wave(&self) -> u8 {
        self.osc2_wave_mod & 0x03
    }
    pub fn set_osc2_wave(&mut self, v: u8) {
        self.osc2_wave_mod = (self.osc2_wave_mod & !0x03) | (v & 0x03);
    }
    pub fn osc2_mod(&self) -> u8 {
        (self.osc2_wave_mod >> 4) & 0x03
    }
    pub fn set_osc2_mod(&mut self, v: u8) {
        self.osc2_wave_mod = (self.osc2_wave_mod & !0x30) | ((v & 0x03) << 4);
    }
    pub fn filter_routing(&self) -> u8 {
        self.filter_routing_type2 & 0x03
    }
    pub fn set_filter_routing(&mut self, v: u8) {
        self.filter_routing_type2 = (self.filter_routing_type2 & !0x03) | (v & 0x03);
    }
    pub fn filter2_type(&self) -> u8 {
        (self.filter_routing_type2 >> 4) & 0x03
    }
    pub fn set_filter2_type(&mut self, v: u8) {
        self.filter_routing_type2 = (self.filter_routing_type2 & !0x30) | ((v & 0x03) << 4);
    }
    pub fn ws_position(&self) -> u8 {
        (self.amp_ws_position >> 4) & 0x03
    }
    pub fn set_ws_position(&mut self, v: u8) {
        self.amp_ws_position = (self.amp_ws_position & !0x30) | ((v & 0x03) << 4);
    }
    pub fn ws_type(&self) -> u8 {
        self.amp_ws_type & 0x0F
    }
    pub fn set_ws_type(&mut self, v: u8) {
        self.amp_ws_type = (self.amp_ws_type & !0x0F) | (v & 0x0F);
    }
    pub fn lfo1_wave_val(&self) -> u8 {
        self.lfo1_wave & 0x0F
    }
    pub fn set_lfo1_wave_val(&mut self, v: u8) {
        self.lfo1_wave = (self.lfo1_wave & !0x0F) | (v & 0x0F);
    }
    pub fn lfo2_wave_val(&self) -> u8 {
        self.lfo2_wave & 0x0F
    }
    pub fn set_lfo2_wave_val(&mut self, v: u8) {
        self.lfo2_wave = (self.lfo2_wave & !0x0F) | (v & 0x0F);
    }
    pub fn lfo1_bpm_sync(&self) -> bool {
        self.lfo1_sync & 0x80 != 0
    }
    pub fn set_lfo1_bpm_sync(&mut self, on: bool) {
        self.lfo1_sync = (self.lfo1_sync & !0x80) | if on { 0x80 } else { 0 };
    }
    pub fn lfo1_key_sync(&self) -> u8 {
        (self.lfo1_sync >> 5) & 0x03
    }
    pub fn set_lfo1_key_sync(&mut self, v: u8) {
        self.lfo1_sync = (self.lfo1_sync & !0x60) | ((v & 0x03) << 5);
    }
    pub fn lfo2_bpm_sync(&self) -> bool {
        self.lfo2_sync & 0x80 != 0
    }
    pub fn set_lfo2_bpm_sync(&mut self, on: bool) {
        self.lfo2_sync = (self.lfo2_sync & !0x80) | if on { 0x80 } else { 0 };
    }
    pub fn lfo2_key_sync(&self) -> u8 {
        (self.lfo2_sync >> 5) & 0x03
    }
    pub fn set_lfo2_key_sync(&mut self, v: u8) {
        self.lfo2_sync = (self.lfo2_sync & !0x60) | ((v & 0x03) << 5);
    }
    pub fn lfo1_sync_note_val(&self) -> u8 {
        self.lfo1_sync_note & 0x1F
    }
    pub fn set_lfo1_sync_note_val(&mut self, v: u8) {
        self.lfo1_sync_note = (self.lfo1_sync_note & !0x1F) | (v & 0x1F);
    }
    pub fn lfo2_sync_note_val(&self) -> u8 {
        self.lfo2_sync_note & 0x1F
    }
    pub fn set_lfo2_sync_note_val(&mut self, v: u8) {
        self.lfo2_sync_note = (self.lfo2_sync_note & !0x1F) | (v & 0x1F);
    }
}

impl Default for RawTimbreProgram {
    fn default() -> Self {
        let mut p = Self::zeroed();
        p.unison_detune = 0;
        p.transpose = 64;
        p.detune = 64;
        p.vibrato_int = 64;
        p.bend_range = 64 + 2;
        p.osc2_semitone = 64;
        p.osc2_tune = 64;
        p.osc1_level = 127;
        p.filter1_balance = 0;
        p.filter1_cutoff = 127;
        p.filter1_eg1_int = 64;
        p.filter1_key_track = 64;
        p.filter1_velo_sens = 64;
        p.filter2_cutoff = 127;
        p.filter2_eg1_int = 64;
        p.filter2_key_track = 64;
        p.filter2_velo_sens = 64;
        p.amp_level = 127;
        p.amp_panpot = 64;
        p.amp_key_track = 64;
        p.eg1_decay = 64;
        p.eg1_sustain = 127;
        p.eg1_level_velo = 64;
        p.eg2_decay = 64;
        p.eg2_sustain = 127;
        p.eg2_level_velo = 64;
        p.eg3_decay = 64;
        p.eg3_sustain = 127;
        p.eg3_level_velo = 64;
        p.lfo1_freq = 64;
        p.lfo2_freq = 64;
        for patch in [
            &mut p.patch1_int,
            &mut p.patch2_int,
            &mut p.patch3_int,
            &mut p.patch4_int,
            &mut p.patch5_int,
            &mut p.patch6_int,
        ] {
            *patch = 64;
        }
        p
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Pod, Zeroable)]
pub struct RawInsertFx {
    pub fx1_type: u8,
    pub _dummy1: u8,
    pub fx1_knob_assign: u8,
    pub _dummy3: u8,
    pub fx1_params: [u8; 20],
    pub fx2_type: u8,
    pub _dummy25: u8,
    pub fx2_knob_assign: u8,
    pub _dummy27: u8,
    pub fx2_params: [u8; 20],
    pub eq_low_freq: u8,
    pub eq_low_gain: u8,
    pub eq_hi_freq: u8,
    pub eq_hi_gain: u8,
}

impl RawInsertFx {
    pub fn fx1_type_val(&self) -> u8 {
        self.fx1_type & 0x7F
    }
    pub fn set_fx1_type_val(&mut self, v: u8) {
        self.fx1_type = (self.fx1_type & 0x80) | (v & 0x7F);
    }
    pub fn fx2_type_val(&self) -> u8 {
        self.fx2_type & 0x7F
    }
    pub fn set_fx2_type_val(&mut self, v: u8) {
        self.fx2_type = (self.fx2_type & 0x80) | (v & 0x7F);
    }
}

impl Default for RawInsertFx {
    fn default() -> Self {
        Self::zeroed()
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Pod, Zeroable)]
pub struct RawMotionSeq {
    pub flags: u8,
    pub sync_res: u8,
    pub seq_params: [u8; 18],
}

impl RawMotionSeq {
    pub fn seq_on(&self) -> bool {
        self.flags & 0x80 != 0
    }
    pub fn set_seq_on(&mut self, on: bool) {
        self.flags = (self.flags & !0x80) | if on { 0x80 } else { 0 };
    }
    pub fn seq_type(&self) -> u8 {
        (self.flags >> 4) & 0x03
    }
    pub fn set_seq_type(&mut self, v: u8) {
        self.flags = (self.flags & !0x30) | ((v & 0x03) << 4);
    }
    pub fn last_step(&self) -> u8 {
        self.flags & 0x0F
    }
    pub fn set_last_step(&mut self, v: u8) {
        self.flags = (self.flags & !0x0F) | (v & 0x0F);
    }
    pub fn key_sync(&self) -> u8 {
        (self.sync_res >> 4) & 0x03
    }
    pub fn set_key_sync(&mut self, v: u8) {
        self.sync_res = (self.sync_res & !0x30) | ((v & 0x03) << 4);
    }
    pub fn resolution(&self) -> u8 {
        self.sync_res & 0x0F
    }
    pub fn set_resolution(&mut self, v: u8) {
        self.sync_res = (self.sync_res & !0x0F) | (v & 0x0F);
    }
}

impl Default for RawMotionSeq {
    fn default() -> Self {
        Self::zeroed()
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Pod, Zeroable, Default)]
pub struct RawTimbre {
    pub program: RawTimbreProgram,
    pub insert_fx: RawInsertFx,
    pub motion_seq: RawMotionSeq,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Pod, Zeroable)]
pub struct RawVocoder {
    pub sw_flags: u8,
    pub gate_sens: u8,
    pub threshold: u8,
    pub hpf_level: u8,
    pub direct_level: u8,
    pub timbre1_level: u8,
    pub input1_level: u8,
    pub vocoder_level: u8,
    pub bands: [u8; 32],
    pub shift_fcmodsrc: u8,
    pub fm_select: u8,
    pub cutoff_offset: u8,
    pub resonance: u8,
    pub fc_mod_int: u8,
    pub ef_sens: u8,
    pub formant_hold: [u8; 32],
}

impl RawVocoder {
    pub fn sw_on(&self) -> bool {
        self.sw_flags & 0x80 != 0
    }
    pub fn set_sw_on(&mut self, on: bool) {
        self.sw_flags = (self.sw_flags & !0x80) | if on { 0x80 } else { 0 };
    }
    pub fn source(&self) -> u8 {
        (self.sw_flags >> 6) & 0x01
    }
    pub fn set_source(&mut self, v: u8) {
        self.sw_flags = (self.sw_flags & !0x40) | ((v & 0x01) << 6);
    }
    pub fn hpf_gate(&self) -> bool {
        self.sw_flags & 0x20 != 0
    }
    pub fn set_hpf_gate(&mut self, on: bool) {
        self.sw_flags = (self.sw_flags & !0x20) | if on { 0x20 } else { 0 };
    }
    pub fn formant_data_play(&self) -> u8 {
        (self.sw_flags >> 4) & 0x01
    }
    pub fn set_formant_data_play(&mut self, v: u8) {
        self.sw_flags = (self.sw_flags & !0x10) | ((v & 0x01) << 4);
    }
    pub fn select(&self) -> u8 {
        self.sw_flags & 0x03
    }
    pub fn set_select(&mut self, v: u8) {
        self.sw_flags = (self.sw_flags & !0x03) | (v & 0x03);
    }
    pub fn band_pan(&self, i: usize) -> u8 {
        self.bands[i * 2]
    }
    pub fn band_level(&self, i: usize) -> u8 {
        self.bands[i * 2 + 1]
    }
    pub fn set_band_pan(&mut self, i: usize, v: u8) {
        self.bands[i * 2] = v;
    }
    pub fn set_band_level(&mut self, i: usize, v: u8) {
        self.bands[i * 2 + 1] = v;
    }
    pub fn shift(&self) -> u8 {
        (self.shift_fcmodsrc >> 4) & 0x07
    }
    pub fn fc_mod_src(&self) -> u8 {
        self.shift_fcmodsrc & 0x0F
    }
}

impl Default for RawVocoder {
    fn default() -> Self {
        let mut v = Self::zeroed();
        v.gate_sens = 64;
        v.threshold = 64;
        v.direct_level = 127;
        v.vocoder_level = 127;
        v.shift_fcmodsrc = 4 << 4;
        v.cutoff_offset = 64;
        v.fc_mod_int = 64;
        for i in 0..16 {
            v.set_band_pan(i, 64);
            v.set_band_level(i, 127);
        }
        v
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Pod, Zeroable)]
pub struct RawMasterFx {
    pub fx_type: u8,
    pub knob_assign: u8,
    pub params: [u8; 20],
}

impl RawMasterFx {
    pub fn fx_type_val(&self) -> u8 {
        self.fx_type & 0x7F
    }
    pub fn set_fx_type_val(&mut self, v: u8) {
        self.fx_type = (self.fx_type & 0x80) | (v & 0x7F);
    }
}

impl Default for RawMasterFx {
    fn default() -> Self {
        Self::zeroed()
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Pod, Zeroable)]
pub struct RawArpeggio {
    pub res_type: u8,
    pub latch_oct_last: u8,
    pub gate_time: u8,
    pub swing: u8,
    pub step_switches: u8,
    pub _dummy: [u8; 3],
}

impl RawArpeggio {
    pub fn resolution(&self) -> u8 {
        (self.res_type >> 4) & 0x0F
    }
    pub fn set_resolution(&mut self, v: u8) {
        self.res_type = (self.res_type & !0xF0) | ((v & 0x0F) << 4);
    }
    pub fn arp_type(&self) -> u8 {
        self.res_type & 0x07
    }
    pub fn set_arp_type(&mut self, v: u8) {
        self.res_type = (self.res_type & !0x07) | (v & 0x07);
    }
    pub fn latch(&self) -> bool {
        self.latch_oct_last & 0x80 != 0
    }
    pub fn octave_range(&self) -> u8 {
        (self.latch_oct_last >> 5) & 0x03
    }
    pub fn last_step(&self) -> u8 {
        self.latch_oct_last & 0x1F
    }
}

impl Default for RawArpeggio {
    fn default() -> Self {
        Self::zeroed()
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Pod, Zeroable)]
pub struct RawProgram {
    pub name: [u8; 8],
    pub voice_arp: u8,
    pub vcd_knob_assigns: [u8; 4],
    pub timbre2_midi_ch: u8,
    pub center_key: u8,
    pub octave_category: u8,
    pub timbre1: RawTimbre,
    pub timbre2: RawTimbre,
    pub vocoder: RawVocoder,
    pub master_fx: RawMasterFx,
    pub tempo: [u8; 2],
    pub arp_flags: u8,
    pub _dummy_447: u8,
    pub arpeggio: RawArpeggio,
}

impl RawProgram {
    pub fn voice_mode(&self) -> u8 {
        (self.voice_arp >> 6) & 0x03
    }
    pub fn set_voice_mode(&mut self, v: u8) {
        self.voice_arp = (self.voice_arp & !0xC0) | ((v & 0x03) << 6);
    }
    pub fn arp_timb_select(&self) -> u8 {
        (self.voice_arp >> 4) & 0x03
    }
    pub fn set_arp_timb_select(&mut self, v: u8) {
        self.voice_arp = (self.voice_arp & !0x30) | ((v & 0x03) << 4);
    }
    pub fn octave_sw(&self) -> i8 {
        ((self.octave_category >> 4) & 0x0F) as i8 - 8
    }
    pub fn set_octave_sw(&mut self, v: i8) {
        let raw = (v + 8).clamp(0, 15) as u8;
        self.octave_category = (self.octave_category & !0xF0) | (raw << 4);
    }
    pub fn category(&self) -> u8 {
        self.octave_category & 0x0F
    }
    pub fn set_category(&mut self, v: u8) {
        self.octave_category = (self.octave_category & !0x0F) | (v & 0x0F);
    }
    pub fn arp_on(&self) -> bool {
        self.arp_flags & 0x80 != 0
    }
    pub fn set_arp_on(&mut self, on: bool) {
        self.arp_flags = (self.arp_flags & !0x80) | if on { 0x80 } else { 0 };
    }
    pub fn arp_key_sync(&self) -> bool {
        self.arp_flags & 0x40 != 0
    }
    pub fn tempo_raw(&self) -> u16 {
        (self.tempo[0] as u16) | ((self.tempo[1] as u16) << 7)
    }
    pub fn set_tempo_raw(&mut self, v: u16) {
        self.tempo[0] = (v & 0x7F) as u8;
        self.tempo[1] = ((v >> 7) & 0x7F) as u8;
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Pod, Zeroable)]
pub struct RawGlobal {
    pub master_tune: u8,
    pub transpose: u8,
    pub flags_2: u8,
    pub vel_curve: u8,
    pub midi_channel: u8,
    pub flags_5: u8,
    pub midi_ctrl: [u8; 3],
    pub filters: u8,
    pub ass_pedal: u8,
    pub ass_switch: u8,
    pub cc_map_lo: [u8; 32],
    pub cc_map_mid: [u8; 32],
    pub cc_map_hi: [u8; 3],
    pub _dummy_79: u8,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Pod, Zeroable)]
pub struct RawFormantStep {
    pub bands: [u8; 16],
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Pod, Zeroable)]
pub struct RawParameterChange {
    pub param_id: [u8; 2],
    pub sub_id: [u8; 2],
    pub value: [u8; 2],
}

#[cfg(test)]
mod tests {
    use std::mem::offset_of;

    use super::*;

    #[test]
    fn test_raw_timbre_program_size() {
        assert_eq!(size_of::<RawTimbreProgram>(), 92);
    }
    #[test]
    fn test_raw_insert_fx_size() {
        assert_eq!(size_of::<RawInsertFx>(), 52);
    }
    #[test]
    fn test_raw_motion_seq_size() {
        assert_eq!(size_of::<RawMotionSeq>(), 20);
    }
    #[test]
    fn test_raw_timbre_size() {
        assert_eq!(size_of::<RawTimbre>(), 164);
    }
    #[test]
    fn test_raw_vocoder_size() {
        assert_eq!(size_of::<RawVocoder>(), 78);
    }
    #[test]
    fn test_raw_master_fx_size() {
        assert_eq!(size_of::<RawMasterFx>(), 22);
    }
    #[test]
    fn test_raw_arpeggio_size() {
        assert_eq!(size_of::<RawArpeggio>(), 8);
    }
    #[test]
    fn test_raw_program_size() {
        assert_eq!(size_of::<RawProgram>(), 456);
    }
    #[test]
    fn test_raw_global_size() {
        assert_eq!(size_of::<RawGlobal>(), 80);
    }
    #[test]
    fn test_raw_formant_step_size() {
        assert_eq!(size_of::<RawFormantStep>(), 16);
    }
    #[test]
    fn test_raw_parameter_change_size() {
        assert_eq!(size_of::<RawParameterChange>(), 6);
    }

    #[test]
    fn test_timbre_program_offsets() {
        assert_eq!(offset_of!(RawTimbreProgram, voice_unison), 4);
        assert_eq!(offset_of!(RawTimbreProgram, unison_spread), 6);
        assert_eq!(offset_of!(RawTimbreProgram, voice_assign), 7);
        assert_eq!(offset_of!(RawTimbreProgram, analog_tuning), 9);
        assert_eq!(offset_of!(RawTimbreProgram, portamento), 14);
        assert_eq!(offset_of!(RawTimbreProgram, osc1_wave_mod), 16);
        assert_eq!(offset_of!(RawTimbreProgram, osc2_wave_mod), 21);
        assert_eq!(offset_of!(RawTimbreProgram, osc1_level), 24);
        assert_eq!(offset_of!(RawTimbreProgram, filter_routing_type2), 28);
        assert_eq!(offset_of!(RawTimbreProgram, filter1_balance), 29);
        assert_eq!(offset_of!(RawTimbreProgram, filter1_cutoff), 30);
        assert_eq!(offset_of!(RawTimbreProgram, filter1_velo_sens), 34);
        assert_eq!(offset_of!(RawTimbreProgram, filter2_velo_sens), 39);
        assert_eq!(offset_of!(RawTimbreProgram, amp_level), 40);
        assert_eq!(offset_of!(RawTimbreProgram, punch_level), 46);
        assert_eq!(offset_of!(RawTimbreProgram, eg1_attack), 48);
        assert_eq!(offset_of!(RawTimbreProgram, eg3_level_velo), 64);
        assert_eq!(offset_of!(RawTimbreProgram, lfo1_wave), 66);
        assert_eq!(offset_of!(RawTimbreProgram, lfo2_sync_note), 73);
        assert_eq!(offset_of!(RawTimbreProgram, patch1_src), 74);
        assert_eq!(offset_of!(RawTimbreProgram, patch6_int), 91);
    }

    #[test]
    fn test_program_region_offsets() {
        assert_eq!(offset_of!(RawProgram, timbre1), 16);
        assert_eq!(offset_of!(RawProgram, timbre2), 180);
        assert_eq!(offset_of!(RawProgram, vocoder), 344);
        assert_eq!(offset_of!(RawProgram, master_fx), 422);
        assert_eq!(offset_of!(RawProgram, tempo), 444);
        assert_eq!(offset_of!(RawProgram, arpeggio), 448);
        assert_eq!(offset_of!(RawTimbre, insert_fx), 92);
        assert_eq!(offset_of!(RawTimbre, motion_seq), 144);
        assert_eq!(offset_of!(RawInsertFx, eq_low_freq), 48);
        assert_eq!(offset_of!(RawVocoder, shift_fcmodsrc), 40);
        assert_eq!(offset_of!(RawVocoder, formant_hold), 46);
    }

    #[test]
    fn test_packed_accessors() {
        let mut p = RawTimbreProgram::zeroed();
        p.set_osc1_wave(6);
        p.set_osc1_mod(3);
        assert_eq!(p.osc1_wave(), 6);
        assert_eq!(p.osc1_mod(), 3);
        assert_eq!(p.osc1_wave_mod, 0x36);
        p.set_filter_routing(1);
        p.set_filter2_type(2);
        assert_eq!(p.filter_routing(), 1);
        assert_eq!(p.filter2_type(), 2);
        p.set_lfo1_bpm_sync(true);
        p.set_lfo1_key_sync(2);
        assert!(p.lfo1_bpm_sync());
        assert_eq!(p.lfo1_key_sync(), 2);

        let mut prog = RawProgram::zeroed();
        prog.set_voice_mode(2);
        prog.set_arp_timb_select(1);
        assert_eq!(prog.voice_mode(), 2);
        assert_eq!(prog.arp_timb_select(), 1);
        prog.set_octave_sw(-3);
        assert_eq!(prog.octave_sw(), -3);
        prog.set_octave_sw(3);
        assert_eq!(prog.octave_sw(), 3);
        prog.set_tempo_raw(1450);
        assert_eq!(prog.tempo_raw(), 1450);
    }
}
