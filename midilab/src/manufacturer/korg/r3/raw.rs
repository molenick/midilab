use bytemuck::Pod;
use bytemuck::Zeroable;

#[repr(C)]
#[derive(Debug, Copy, Clone, Pod, Zeroable)]
pub struct RawTimbreProgram {
    pub knob_assigns: [u8; 4],
    pub voice_assign: u8,
    pub unison_detune: u8,
    pub pitch_tune: u8,
    pub pitch_bend_range: u8,
    pub pitch_transpose: u8,
    pub pitch_vibrato_int: u8,
    pub osc1_wave: u8,
    pub osc1_ctrl1: u8,
    pub osc1_ctrl2: u8,
    pub osc1_dwgs: u8,
    pub osc2_wave_mod: u8,
    pub osc2_semitone: u8,
    pub osc2_tune: u8,
    pub mixer_osc1_level: u8,
    pub mixer_osc2_level: u8,
    pub mixer_noise_level: u8,
    pub filter1_type: u8,
    pub filter1_cutoff: u8,
    pub filter1_resonance: u8,
    pub filter1_eg1_int: u8,
    pub filter1_key_track: u8,
    pub filter2_type: u8,
    pub filter2_cutoff: u8,
    pub filter2_resonance: u8,
    pub filter2_eg1_int: u8,
    pub filter2_key_track: u8,
    pub filter_routing: u8,
    pub amp_level: u8,
    pub amp_pan: u8,
    pub amp_sw_distortion: u8,
    pub amp_key_track: u8,
    pub eg1_attack: u8,
    pub eg1_decay: u8,
    pub eg1_sustain: u8,
    pub eg1_release: u8,
    pub eg2_attack: u8,
    pub eg2_decay: u8,
    pub eg2_sustain: u8,
    pub eg2_release: u8,
    pub eg3_attack: u8,
    pub eg3_decay: u8,
    pub eg3_sustain: u8,
    pub eg3_release: u8,
    pub lfo1_wave: u8,
    pub lfo1_freq: u8,
    pub lfo1_key_sync_tempo: u8,
    pub lfo1_sync_note: u8,
    pub lfo2_wave: u8,
    pub lfo2_freq: u8,
    pub lfo2_key_sync_tempo: u8,
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
    pub _reserved: [u8; 19],
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Pod, Zeroable)]
pub struct RawInsertFx {
    pub dry_wet: u8,
    pub _reserved: u8,
    pub params_lo: [u8; 32],
    pub params_hi: [u8; 18],
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Pod, Zeroable)]
pub struct RawMotionSeq {
    pub knob_assign: u8,
    pub motion_type: u8,
    pub steps: [u8; 16],
    pub _padding: [u8; 2],
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Pod, Zeroable)]
pub struct RawTimbre {
    pub program: RawTimbreProgram,
    pub insert_fx: RawInsertFx,
    pub motion_seq: RawMotionSeq,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Pod, Zeroable)]
pub struct RawVocoder {
    pub sw_source_hpf: u8,
    pub threshold: u8,
    pub hpf_level: u8,
    pub gate_sense: u8,
    pub band_levels: [u8; 16],
    pub band_pans: [u8; 16],
    pub filter_cutoff: u8,
    pub filter_resonance: u8,
    pub filter_mod_src: u8,
    pub filter_eg1_int: u8,
    pub amp_level: u8,
    pub amp_direct_level: u8,
    pub amp_distortion: u8,
    pub amp_key_track: u8,
    pub formant_hold: u8,
    pub formant_shift: u8,
    pub _reserved: [u8; 32],
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Pod, Zeroable)]
pub struct RawMasterFx {
    pub fx_type: u8,
    pub knob_assign: u8,
    pub params: [u8; 20],
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Pod, Zeroable)]
pub struct RawArpeggio {
    pub resolution_type: u8,
    pub latch_oct_last: u8,
    pub gate_time: u8,
    pub swing: u8,
    pub step_switches: u8,
    pub _dummy: [u8; 3],
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Pod, Zeroable)]
pub struct RawProgram {
    pub name: [u8; 8],
    pub voice_mode_arp_timb: u8,
    pub knob_assigns: [u8; 4],
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
    pub secondary: [u8; 16],
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Pod, Zeroable)]
pub struct RawFormantHeader {
    pub field_0: u8,
    pub field_1: u8,
    pub field_2: u8,
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
    use super::*;

    #[test]
    fn test_raw_timbre_program_size() {
        assert_eq!(std::mem::size_of::<RawTimbreProgram>(), 92);
    }

    #[test]
    fn test_raw_insert_fx_size() {
        assert_eq!(std::mem::size_of::<RawInsertFx>(), 52);
    }

    #[test]
    fn test_raw_motion_seq_size() {
        assert_eq!(std::mem::size_of::<RawMotionSeq>(), 20);
    }

    #[test]
    fn test_raw_timbre_size() {
        assert_eq!(std::mem::size_of::<RawTimbre>(), 164);
    }

    #[test]
    fn test_raw_vocoder_size() {
        assert_eq!(std::mem::size_of::<RawVocoder>(), 78);
    }

    #[test]
    fn test_raw_master_fx_size() {
        assert_eq!(std::mem::size_of::<RawMasterFx>(), 22);
    }

    #[test]
    fn test_raw_arpeggio_size() {
        assert_eq!(std::mem::size_of::<RawArpeggio>(), 8);
    }

    #[test]
    fn test_raw_program_size() {
        assert_eq!(std::mem::size_of::<RawProgram>(), 456);
    }

    #[test]
    fn test_raw_global_size() {
        assert_eq!(std::mem::size_of::<RawGlobal>(), 80);
    }

    #[test]
    fn test_raw_formant_step_size() {
        assert_eq!(std::mem::size_of::<RawFormantStep>(), 32);
    }

    #[test]
    fn test_raw_formant_header_size() {
        assert_eq!(std::mem::size_of::<RawFormantHeader>(), 3);
    }

    #[test]
    fn test_raw_parameter_change_size() {
        assert_eq!(std::mem::size_of::<RawParameterChange>(), 6);
    }
}
