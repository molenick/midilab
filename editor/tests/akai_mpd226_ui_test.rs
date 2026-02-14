use eframe::egui::Key;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use midilab_gui::AkaiMpd226Editor;
use tokio::sync::mpsc::unbounded_channel;

#[test]
fn test_top_level_app_tabbing() {
    let (app_tx, _app_rx) = unbounded_channel();
    let (_ui_tx, ui_rx) = unbounded_channel();
    let app = AkaiMpd226Editor::new(app_tx, ui_rx);

    let mut harness = Harness::new_state(
        |ctx, app: &mut AkaiMpd226Editor| {
            app.render_ui(ctx);
        },
        app,
    );

    let mut tabbable_elements = Vec::from(
        [
            "Dump preset from device",
            "Write preset to device",
            "Dump global from device",
            "Write global to device",
            "Preset Settings",
            "Global Settings",
            "Pattern Mapping",
            "Reset Preset",
        ]
        .map(|s| s.to_owned()),
    );

    let control_banks = ["A", "B", "C"];

    // Find app nodes w/ unique labels
    // panics when quantity != 1
    let _title_label = harness.get_by_label("Akai MPD226 Editor");

    let pad_labels: Vec<String> = (0..64).map(|v| format!("Pad {v}")).collect();
    tabbable_elements.extend_from_slice(&pad_labels);
    let dial_labels: Vec<String> = control_banks
        .iter()
        .flat_map(|bank| (1..5).map(move |i| format!("Dial {bank}{i}")))
        .collect();
    tabbable_elements.extend_from_slice(&dial_labels);
    let fader_labels: Vec<String> = control_banks
        .iter()
        .flat_map(|bank| (1..5).map(move |i| format!("Fader {bank}{i}")))
        .collect();
    tabbable_elements.extend_from_slice(&fader_labels);
    let switch_labels: Vec<String> = control_banks
        .iter()
        .flat_map(|bank| (1..5).map(move |i| format!("Switch {bank}{i}")))
        .collect();
    tabbable_elements.extend_from_slice(&switch_labels);

    // todo: we nav through accesskit or keyboard - whats best?
    // is there value in each?
    for label in tabbable_elements {
        harness.key_press(Key::Tab);
        harness.run_steps(1);
        let node = harness.get_by_label(&label);

        assert!(
            node.is_focused(),
            "expected node to have focus but it did not: {:?}",
            label
        );
    }

    // todo: how to get tooltip text? We prob want to verify each type
}
