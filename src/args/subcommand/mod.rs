pub mod call;
pub mod text;

use clap::{command, Subcommand};

#[derive(Subcommand, Debug, Clone, Copy)]
pub enum Action {
    /// Speak to GPT3 with your own voice, and hear audible responses. Requires valid AWS credentials.
    #[command()]
    Call,
    /// Speak to GPT3 by typing and sending messages to it, and read responses in the terminal.
    #[command()]
    Text,
}
