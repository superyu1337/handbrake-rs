use handbrake_rs::HandBrake;

#[tokio::main]
async fn main() {
    println!("HandBrakeCLI version is: {}", HandBrake::new().await.expect("Failed to find HandBrakeCLI").version());
}
