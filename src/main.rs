pub mod app;
pub mod args;
pub mod aws_api;
pub mod message;
pub mod openai_api;

use app::App;
use args::Args;
use tracing_subscriber::filter::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // A .env is supported, but not required
    let _ = dotenv::dotenv();
    let log_file = std::fs::File::create("debug.log")?;
    let (non_blocking, _guard) = tracing_appender::non_blocking(log_file);
    tracing_subscriber::fmt()
        .compact()
        .with_env_filter(EnvFilter::from_default_env())
        .with_writer(non_blocking)
        .init();

    let args = Args::parse();

    App::run_until_exit(args).await
}
