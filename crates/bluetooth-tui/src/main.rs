mod tui;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tui::run().await
}
