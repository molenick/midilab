use std::sync::Arc;

use arturia_minilab_mk2_editor::MinilabMk2Editor;
use arturia_minilab_mk2_editor::config::AppConfig;
use egui_kittest::HarnessBuilder;
use egui_kittest::kittest::Queryable;
use tokio::sync::mpsc::unbounded_channel;

#[test]
fn every_page_renders() {
    let probes = [
        ("KNOBS", "Shift Layer"),
        ("PADS", "Pad 16"),
        ("CONTROLLER", "Sustain Pedal"),
        ("GLOBAL", "Global Settings"),
        ("MEMORY", "Recall"),
    ];

    for (tab_label, expected_label) in probes {
        let (app_tx, _app_rx) = unbounded_channel();
        let (_ui_tx, ui_rx) = unbounded_channel();
        let config = Arc::new(AppConfig::default());
        let app = MinilabMk2Editor::new(app_tx, ui_rx, config);

        let mut harness = HarnessBuilder::default()
            .with_size(eframe::egui::vec2(1200.0, 820.0))
            .build_ui_state(
                |ui, app: &mut MinilabMk2Editor| {
                    app.render(ui);
                },
                app,
            );
        harness.run();
        harness.get_by_label(tab_label).click();
        harness.run();
        let _ = harness.get_by_label(expected_label);
    }
}
