use midilab::IntoEnumIterator;
use midilab::manufacturer::akai::mpd226::control::value_kind::PadKind;

#[test]
fn into_enum_iterator_is_usable_from_midi_device() {
    let variants: Vec<PadKind> = PadKind::iter().collect();
    assert_eq!(variants, vec![PadKind::Note, PadKind::Prog, PadKind::Bank]);
}
