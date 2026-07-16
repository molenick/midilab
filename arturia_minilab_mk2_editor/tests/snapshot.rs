use std::sync::Arc;

use arturia_minilab_mk2_editor::MinilabMk2Editor;
use arturia_minilab_mk2_editor::config::AppConfig;
use arturia_minilab_mk2_editor::state::EditorTab;
use egui_kittest::HarnessBuilder;
use egui_kittest::kittest::Queryable;
use tokio::sync::mpsc::unbounded_channel;

#[test]
fn every_page_renders() {
    let probes = [
        (EditorTab::Knobs, "Shift Layer"),
        (EditorTab::Pads, "Pad 16"),
        (EditorTab::Controller, "Sustain Pedal"),
        (EditorTab::Global, "Global Settings"),
        (EditorTab::Memory, "Recall"),
    ];

    for (tab, expected_label) in probes {
        let (app_tx, _app_rx) = unbounded_channel();
        let (_ui_tx, ui_rx) = unbounded_channel();
        let config = Arc::new(AppConfig::default());
        let app = MinilabMk2Editor::new(app_tx, ui_rx, config);

        let mut harness = HarnessBuilder::default()
            .with_size(eframe::egui::vec2(1200.0, 820.0))
            .build_ui_state(
                move |ui, app: &mut MinilabMk2Editor| {
                    app.set_tab_for_test(tab);
                    app.render(ui);
                },
                app,
            );
        harness.run();
        let _ = harness.get_by_label(expected_label);
    }
}
