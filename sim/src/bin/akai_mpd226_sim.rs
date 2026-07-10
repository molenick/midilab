use midilab::manufacturer::akai::mpd226::PORT_NAME;
use midilab_sim::manufacturer::akai::akai_mpd226::SimRunner;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (runner, _handle) = SimRunner::start(PORT_NAME)?;

    println!("MPD226 simulator running on virtual port: {PORT_NAME}");
    println!("Press Ctrl+C to exit");

    runner.run().await;

    Ok(())
}
