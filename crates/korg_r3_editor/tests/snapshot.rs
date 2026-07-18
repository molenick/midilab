use std::sync::Arc;

use egui_kittest::HarnessBuilder;
use egui_kittest::kittest::Queryable;
use korg_r3_editor::KorgR3Editor;
use korg_r3_editor::config::AppConfig;
use tokio::sync::mpsc::unbounded_channel;

#[test]
fn every_page_renders() {
    let probes = [
        ("PROGRAM", "Common"),
        ("SYNTH", "Pitch & Voice"),
        ("VOCODER", "Switches"),
        ("FX", "Insert FX 1"),
        ("ARP", "Arpeggiator"),
        ("GLOBAL", "Global Settings"),
        ("FORMANT", "Formant Motion"),
    ];

    for (tab_label, expected_label) in probes {
        let (app_tx, _app_rx) = unbounded_channel();
        let (_ui_tx, ui_rx) = unbounded_channel();
        let config = Arc::new(AppConfig::default());
        let app = KorgR3Editor::new(app_tx, ui_rx, config);

        let mut harness = HarnessBuilder::default()
            .with_size(eframe::egui::vec2(1200.0, 820.0))
            .build_ui_state(
                |ui, app: &mut KorgR3Editor| {
                    app.render(ui);
                },
                app,
            );
        harness.run();
        harness.get_by_label(&format!("  {tab_label}  ")).click();
        harness.run();
        let _ = harness.get_by_label(expected_label);
    }
}
