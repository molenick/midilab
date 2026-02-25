#[cfg(unix)]
use midilab_sim::manufacturer::akai::akai_mpd226::SimRunner;

#[cfg(unix)]
const PORT_NAME: &str = "MPD226 Remote";

#[cfg(unix)]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (runner, _handle) = SimRunner::start(PORT_NAME)?;

    println!("MPD226 simulator running on virtual port: {PORT_NAME}");
    println!("Press Ctrl+C to exit");

    runner.run().await;

    Ok(())
}

#[cfg(not(unix))]
fn main() {}
