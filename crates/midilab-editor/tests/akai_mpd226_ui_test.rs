use std::sync::Arc;

use eframe::egui::Key;
use egui_kittest::HarnessBuilder;
use egui_kittest::kittest::Queryable;
use midilab_editor::akai_mpd226::APP_DIMENSIONS;
use midilab_editor::akai_mpd226::AkaiMpd226Editor;
use midilab_editor::akai_mpd226::config::AppConfig;
use tokio::sync::mpsc::unbounded_channel;

#[test]
fn test_modal_for_different_control_types() {
    let (app_tx, _app_rx) = unbounded_channel();
    let (_ui_tx, ui_rx) = unbounded_channel();
    let config = Arc::new(AppConfig::default());
    let app = AkaiMpd226Editor::new(app_tx, ui_rx, config);

    let mut harness = HarnessBuilder::default()
        .with_size(APP_DIMENSIONS)
        .build_ui_state(
            |ui, app: &mut AkaiMpd226Editor| {
                app.render(ui);
            },
            app,
        );

    harness.get_by_label("Pad 1").click();
    harness.run();
    let _pad_modal = harness.get_by_label("Edit Pad 1");
    let _kind_label = harness.get_by_label("kind");
    let _channel_label = harness.get_by_label("channel");
    let _note_label = harness.get_by_label("note");

    harness.key_press(Key::Escape);
    harness.run();

    harness.get_by_label("Dial 1").click();
    harness.run();
    let _dial_modal = harness.get_by_label("Edit Dial 1");
    harness.key_press(Key::Escape);
    harness.run();

    harness.get_by_label("Fader 1").click();
    harness.run();
    let _fader_modal = harness.get_by_label("Edit Fader 1");

    harness.key_press(Key::Escape);
    harness.run();

    harness.get_by_label("Switch 1").click();
    harness.run();
    let _switch_modal = harness.get_by_label("Edit Switch 1");
}
