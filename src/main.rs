#[tokio::main]
async fn main() {
    if let Err(e) = task_tools::run().await {
        log::error!("Server failed: {e}");
        std::process::exit(1);
    }
}
