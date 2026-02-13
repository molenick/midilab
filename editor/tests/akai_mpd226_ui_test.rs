use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use midilab_gui::AkaiMpd226Editor;
use tokio::sync::mpsc::unbounded_channel;

#[test]
fn test_app_title_is_displayed() {
    let (app_tx, _app_rx) = unbounded_channel();
    let (_ui_tx, ui_rx) = unbounded_channel();

    let app = AkaiMpd226Editor::new(app_tx, ui_rx);

    let harness = Harness::new_state(
        |ctx, app: &mut AkaiMpd226Editor| {
            app.render_ui(ctx);
        },
        app,
    );

    let _title_label = harness.get_by_label("Akai MPD226 Editor");
}
