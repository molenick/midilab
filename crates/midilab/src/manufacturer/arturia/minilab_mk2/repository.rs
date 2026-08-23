use crate::manufacturer::arturia::minilab_mk2::TOTAL_KNOBS;
use crate::manufacturer::arturia::minilab_mk2::TOTAL_PADS;
use crate::manufacturer::arturia::minilab_mk2::TOTAL_SHIFT_KNOBS;
use crate::manufacturer::arturia::minilab_mk2::control::Button;
use crate::manufacturer::arturia::minilab_mk2::control::ControlId;
use crate::manufacturer::arturia::minilab_mk2::control::Knob;
use crate::manufacturer::arturia::minilab_mk2::control::Pad;
use crate::manufacturer::arturia::minilab_mk2::control::value_kind::PadColor;
use crate::manufacturer::arturia::minilab_mk2::error::PresetParseError;
use crate::manufacturer::arturia::minilab_mk2::raw::RawControl;
use crate::manufacturer::arturia::minilab_mk2::raw::RawPad;
use crate::manufacturer::arturia::minilab_mk2::raw::TOTAL_BUTTONS;
use crate::midi::Note;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct KnobRepository {
    pub knobs: [Knob; TOTAL_KNOBS],
    pub shift_knobs: [Knob; TOTAL_SHIFT_KNOBS],
}

impl KnobRepository {
    pub fn with_cc_values(cc: [u8; TOTAL_KNOBS]) -> Self {
        let mut repository = Self::default();
        for (knob, cc) in repository.knobs.iter_mut().zip(cc) {
            knob.cc = cc.into();
        }
        repository
    }
}

impl Default for KnobRepository {
    fn default() -> Self {
        let knobs = ControlId::KNOBS.map(Knob::new);
        let shift_knobs = ControlId::SHIFT_KNOBS.map(Knob::new);

        Self { knobs, shift_knobs }
    }
}

impl TryFrom<(&[RawControl; TOTAL_KNOBS], &[RawControl; TOTAL_SHIFT_KNOBS])> for KnobRepository {
    type Error = PresetParseError;

    fn try_from(
        (raw_knobs, raw_shift_knobs): (
            &[RawControl; TOTAL_KNOBS],
            &[RawControl; TOTAL_SHIFT_KNOBS],
        ),
    ) -> Result<Self, Self::Error> {
        let mut repository = Self::default();

        for (index, (id, raw)) in ControlId::KNOBS.iter().zip(raw_knobs).enumerate() {
            repository.knobs[index] = Knob::try_from((*id, *raw))
                .map_err(|source| PresetParseError::Knob { index, source })?;
        }

        for (index, (id, raw)) in ControlId::SHIFT_KNOBS
            .iter()
            .zip(raw_shift_knobs)
            .enumerate()
        {
            repository.shift_knobs[index] = Knob::try_from((*id, *raw))
                .map_err(|source| PresetParseError::ShiftKnob { index, source })?;
        }

        Ok(repository)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ButtonRepository {
    pub buttons: [Button; TOTAL_BUTTONS],
}

impl Default for ButtonRepository {
    fn default() -> Self {
        Self {
            buttons: ControlId::BUTTONS.map(Button::new),
        }
    }
}

impl TryFrom<&[RawControl; TOTAL_BUTTONS]> for ButtonRepository {
    type Error = PresetParseError;

    fn try_from(raw_buttons: &[RawControl; TOTAL_BUTTONS]) -> Result<Self, Self::Error> {
        let mut repository = Self::default();

        for (index, (id, raw)) in ControlId::BUTTONS.iter().zip(raw_buttons).enumerate() {
            repository.buttons[index] = Button::try_from((*id, *raw))
                .map_err(|source| PresetParseError::Button { index, source })?;
        }

        Ok(repository)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PadRepository {
    pub pads: [Pad; TOTAL_PADS],
}

impl PadRepository {
    pub fn with_notes_from(&mut self, root: Note) {
        for (index, pad) in self.pads.iter_mut().enumerate() {
            let note: u8 = root.into();
            pad.note = Note::from(note.saturating_add(index as u8));
        }
    }

    pub fn set_color_all(&mut self, color: PadColor) {
        for pad in self.pads.iter_mut() {
            pad.color = color;
        }
    }
}

impl Default for PadRepository {
    fn default() -> Self {
        let mut pads = ControlId::PADS.map(Pad::new);
        for (index, pad) in pads.iter_mut().enumerate() {
            pad.note = Note::from(36 + index as u8);
        }

        Self { pads }
    }
}

impl TryFrom<&[RawPad; TOTAL_PADS]> for PadRepository {
    type Error = PresetParseError;

    fn try_from(raw_pads: &[RawPad; TOTAL_PADS]) -> Result<Self, Self::Error> {
        let mut repository = Self::default();

        for (index, (id, raw)) in ControlId::PADS.iter().zip(raw_pads).enumerate() {
            repository.pads[index] = Pad::try_from((*id, *raw))
                .map_err(|source| PresetParseError::Pad { index, source })?;
        }

        Ok(repository)
    }
}
