use std::sync::Arc;

use egui_kittest::HarnessBuilder;
use egui_kittest::kittest::Queryable;
use midilab_editor::nektar_impact_lx_plus::ImpactLxPlusEditor;
use midilab_editor::nektar_impact_lx_plus::config::AppConfig;
use tokio::sync::mpsc::unbounded_channel;

#[test]
fn every_page_renders() {
    let probes = [
        ("PRESETS", "Fader Buttons"),
        ("PADS", "Pad 8"),
        ("WHEELS & TRANSPORT", "Transport Buttons"),
        ("SETTINGS", "Unmapped Settings"),
    ];

    for (tab_label, expected_label) in probes {
        let (app_tx, _app_rx) = unbounded_channel();
        let (_ui_tx, ui_rx) = unbounded_channel();
        let config = Arc::new(AppConfig::default());
        let app = ImpactLxPlusEditor::new(app_tx, ui_rx, config);

        let mut harness = HarnessBuilder::default()
            .with_size(eframe::egui::vec2(1200.0, 820.0))
            .build_ui_state(
                |ui, app: &mut ImpactLxPlusEditor| {
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
