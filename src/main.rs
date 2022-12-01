pub mod app;
pub mod message;
pub mod openai_api;

use crate::app::App;

const USER_NAME: &str = "User";
const BOT_NAME: &str = "Bot";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv().expect("failed to read .env file, please create one");
    // TODO log to file instead of Stdout
    // tracing_subscriber::fmt::init();

    App::run_until_exit().await
}
