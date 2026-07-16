use midilab::manufacturer::arturia::minilab_mk2::PORT_NAME;
use midilab_sim::manufacturer::arturia::arturia_minilab_mk2::SimRunner;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (runner, _handle) = SimRunner::start(PORT_NAME).await?;

    println!("MiniLab mkII simulator running on virtual port: {PORT_NAME}");
    println!("Press Ctrl+C to exit");

    runner.run().await;

    Ok(())
}
