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
}

#[test]
fn test_modal_for_different_control_types() {
    let (app_tx, _app_rx) = unbounded_channel();
    let (_ui_tx, ui_rx) = unbounded_channel();
    let app = AkaiMpd226Editor::new(app_tx, ui_rx);

    let mut harness = Harness::new_state(
        |ctx, app: &mut AkaiMpd226Editor| {
            app.render_ui(ctx);
        },
        app,
    );

    harness.get_by_label("Pad 0").click();
    harness.run();
    let _pad_modal = harness.get_by_label("Edit Pad 0");
    let _kind_label = harness.get_by_label("kind");
    let _channel_label = harness.get_by_label("channel");
    let _note_label = harness.get_by_label("note");

    harness.get_by_label("Close").click();
    harness.run();

    harness.get_by_label("Dial A1").click();
    harness.run();
    let _dial_modal = harness.get_by_label("Edit Dial A1");
    harness.key_press(Key::Escape);
    harness.run();

    harness.get_by_label("Fader B2").click();
    harness.run();
    let _fader_modal = harness.get_by_label("Edit Fader B2");
    // todo: we don't have "click outside of modal to dismiss" tested - we should replace
    // this dupe of esc key
    harness.key_press(Key::Escape);
    harness.run();

    harness.get_by_label("Switch C3").click();
    harness.run();
    let _switch_modal = harness.get_by_label("Edit Switch C3");
}

#[test]
fn test_modal_blocks_background_focus() {
    let (app_tx, mut app_rx) = unbounded_channel();
    let (_ui_tx, ui_rx) = unbounded_channel();
    let app = AkaiMpd226Editor::new(app_tx, ui_rx);

    let mut harness = Harness::new_state(
        |ctx, app: &mut AkaiMpd226Editor| {
            app.render_ui(ctx);
        },
        app,
    );

    harness.get_by_label("Pad 0").click();
    harness.run();

    let _modal_title = harness.get_by_label("Edit Pad 0");

    harness.get_by_label("Dump preset from device").click();
    harness.run();

    // verify no messages sent
    assert!(app_rx.try_recv().is_err(),);

    harness.key_press(Key::Escape);
    harness.run();

    harness.get_by_label("Dump preset from device").click();
    harness.run();

    // verify controls active and messages sent
    assert!(app_rx.try_recv().is_ok(),);
}
