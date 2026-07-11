use std::sync::Arc;

use egui_kittest::HarnessBuilder;
use egui_kittest::kittest::Queryable;
use korg_r3_editor::KorgR3Editor;
use korg_r3_editor::config::AppConfig;
use korg_r3_editor::state::EditorTab;
use tokio::sync::mpsc::unbounded_channel;

#[test]
fn every_page_renders() {
    let probes = [
        (EditorTab::Program, "Common"),
        (EditorTab::Synth, "Pitch & Voice"),
        (EditorTab::Vocoder, "Switches"),
        (EditorTab::Fx, "Insert FX 1"),
        (EditorTab::Arp, "Arpeggiator"),
        (EditorTab::Global, "Global Settings"),
        (EditorTab::Formant, "Formant Motion"),
    ];

    for (tab, expected_label) in probes {
        let (app_tx, _app_rx) = unbounded_channel();
        let (_ui_tx, ui_rx) = unbounded_channel();
        let config = Arc::new(AppConfig::default());
        let app = KorgR3Editor::new(app_tx, ui_rx, config);

        let mut harness = HarnessBuilder::default()
            .with_size(eframe::egui::vec2(1200.0, 820.0))
            .build_ui_state(
                move |ui, app: &mut KorgR3Editor| {
                    app.set_tab_for_test(tab);
                    app.render(ui);
                },
                app,
            );
        harness.run();
        let _ = harness.get_by_label(expected_label);
    }
}
