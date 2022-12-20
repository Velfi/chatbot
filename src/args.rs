mod subcommand;

use clap::{command, Parser};
use std::path::{Path, PathBuf};
pub use subcommand::Action;

/// A clap args struct containing the command line arguments for this program
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    #[clap(subcommand)]
    action: Action,

    /// When passed, resume the previous conversation instead of starting a new one.
    #[clap(long, default_value_t = false)]
    resume: bool,

    /// The OpenAI model to use.
    /// If not provided, the OPENAI_MODEL_NAME environment variable will be used.
    /// Defaults to "text-davinci-003".
    #[clap(long)]
    model: Option<String>,

    /// The name of the bot.
    /// If not provided, the THEIR_NAME environment variable will be used.
    /// Defaults to "Bot".
    #[clap(long)]
    their_name: Option<String>,

    /// The name of the user.
    /// If not provided, the YOUR_NAME environment variable will be used.
    /// Defaults to "User".
    #[clap(long)]
    your_name: Option<String>,

    /// The prompt to use when fetching a response from OpenAI.
    /// If not provided, the STARTING_PROMPT environment variable will be used.
    /// Defaults to "The following is a conversation that 'User' is having with an AI assistant named 'Bot'. The assistant is helpful, creative, clever, and very friendly."
    #[clap(long)]
    prompt: Option<String>,

    /// The number of tokens to generate.
    /// If not provided, the RESPONSE_TOKEN_LIMIT environment variable will be used.
    /// Defaults to 100.
    #[clap(long)]
    token_limit: Option<u32>,

    /// The number of messages to use as context for the prompt.
    /// If not provided, the PROMPT_CONTEXT_LENGTH environment variable will be used.
    /// Defaults to 5.
    #[clap(long)]
    prompt_context_length: Option<usize>,

    /// The path to the database file.
    /// If not provided, the DATABASE_FILE_PATH environment variable will be used.
    /// Defaults to "chatbot.db".
    #[clap(long)]
    db_path: Option<PathBuf>,
}

impl Args {
    pub fn parse() -> Self {
        Parser::parse()
    }

    pub fn action(&self) -> Action {
        self.action
    }

    pub fn resume(&self) -> bool {
        self.resume
    }

    pub fn model(&self) -> Option<&str> {
        self.model.as_deref()
    }

    pub fn their_name(&self) -> Option<&str> {
        self.their_name.as_deref()
    }

    pub fn your_name(&self) -> Option<&str> {
        self.your_name.as_deref()
    }

    pub fn prompt(&self) -> Option<&str> {
        self.prompt.as_deref()
    }

    pub fn token_limit(&self) -> Option<u32> {
        self.token_limit
    }

    pub fn prompt_context_length(&self) -> Option<usize> {
        self.prompt_context_length
    }

    pub fn db_path(&self) -> Option<&Path> {
        self.db_path.as_deref()
    }
}
