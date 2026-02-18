use eframe::egui::Key;
use egui_kittest::HarnessBuilder;
use egui_kittest::kittest::Queryable;
use midilab_gui::AkaiMpd226Editor;
use midilab_gui::akai_mpd226_editor::APP_DIMENSIONS;
use tokio::sync::mpsc::unbounded_channel;

#[test]
fn test_modal_for_different_control_types() {
    let (app_tx, _app_rx) = unbounded_channel();
    let (_ui_tx, ui_rx) = unbounded_channel();
    let app = AkaiMpd226Editor::new(app_tx, ui_rx);

    let mut harness = HarnessBuilder::default()
        .with_size(APP_DIMENSIONS)
        .build_state(
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

    harness.key_press(Key::Escape);
    harness.run();

    harness.get_by_label("Switch C3").click();
    harness.run();
    let _switch_modal = harness.get_by_label("Edit Switch C3");
}
