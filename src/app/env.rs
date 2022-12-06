use anyhow::Context;
use std::{
    env,
    path::{Path, PathBuf},
    time::Duration,
};

use crate::Args;

const DEFAULT_PROMPT: &str = "The following is a conversation that 'User' is having with an AI assistant named 'Bot'. The assistant is helpful, creative, clever, and very friendly.";
const DEFAULT_YOUR_NAME: &str = "User";
const DEFAULT_THEIR_NAME: &str = "Bot";
const DEFAULT_MODEL_NAME: &str = "text-davinci-003";
const DEFAULT_TOKEN_LIMIT: u32 = 100;
const DEFAULT_EXPECTED_RESPONSE_TIME: Duration = Duration::from_secs(5);
const DEFAULT_PROMPT_CONTEXT_LENGTH: usize = 5;
const DEFAULT_DB_PATH: &str = "chatbot.db";

pub struct Env {
    your_name: String,
    their_name: String,
    starting_prompt: String,
    openai_model_name: String,
    expected_response_time: Duration,
    prompt_context_length: usize,
    database_file_path: PathBuf,
    user_input_poll_duration: Duration,
    token_limit: u32,
}

impl Env {
    pub fn new(args: &Args) -> Result<Self, anyhow::Error> {
        let your_name = args
            .your_name()
            .map(ToOwned::to_owned)
            .or_else(|| env::var("YOUR_NAME").ok())
            .unwrap_or_else(|| DEFAULT_YOUR_NAME.to_owned());
        let their_name = args
            .their_name()
            .map(ToOwned::to_owned)
            .or_else(|| env::var("THEIR_NAME").ok())
            .unwrap_or_else(|| DEFAULT_THEIR_NAME.to_owned());
        let starting_prompt = args
            .prompt()
            .map(ToOwned::to_owned)
            .or_else(|| env::var("STARTING_PROMPT").ok())
            .unwrap_or_else(|| DEFAULT_PROMPT.to_owned());
        let openai_model_name = args
            .model()
            .map(ToOwned::to_owned)
            .or_else(|| env::var("OPENAI_MODEL_NAME").ok())
            .unwrap_or_else(|| DEFAULT_MODEL_NAME.to_owned());
        let expected_response_time = env::var("EXPECTED_RESPONSE_TIME")
            .context("checking for expected_response_time in env")
            .and_then(|t| {
                t.parse()
                    .context("parsing expected_response_time from env")
                    .map(Duration::from_millis)
            })
            .unwrap_or(DEFAULT_EXPECTED_RESPONSE_TIME);
        let prompt_context_length = args
            .prompt_context_length()
            .or_else(|| {
                env::var("PROMPT_CONTEXT_LENGTH")
                    .ok()
                    .and_then(|s| s.parse().ok())
            })
            .unwrap_or(DEFAULT_PROMPT_CONTEXT_LENGTH);
        let database_file_path = args.db_path().map(PathBuf::from).unwrap_or_else(|| {
            env::var("DATABASE_FILE_PATH")
                .map(PathBuf::from)
                .unwrap_or_else(|_| PathBuf::from(DEFAULT_DB_PATH))
        });
        let user_input_poll_duration = env::var("USER_INPUT_POLL_DURATION")
            .context("checking for user_input_poll_duration in env")
            .and_then(|t| {
                t.parse()
                    .context("parsing user_input_poll_duration from env")
                    .map(Duration::from_millis)
            })
            .unwrap_or(Duration::from_millis(10));
        let token_limit = args
            .token_limit()
            .or_else(|| env::var("TOKEN_LIMIT").ok().and_then(|s| s.parse().ok()))
            .unwrap_or(DEFAULT_TOKEN_LIMIT);

        Ok(Self {
            your_name,
            their_name,
            starting_prompt,
            openai_model_name,
            expected_response_time,
            prompt_context_length,
            database_file_path,
            user_input_poll_duration,
            token_limit,
        })
    }

    pub fn your_name(&self) -> &str {
        &self.your_name
    }

    pub fn their_name(&self) -> &str {
        &self.their_name
    }

    pub fn starting_prompt(&self) -> &str {
        &self.starting_prompt
    }

    pub fn openai_model_name(&self) -> &str {
        &self.openai_model_name
    }

    pub fn expected_response_time(&self) -> Duration {
        self.expected_response_time
    }

    pub fn prompt_context_length(&self) -> usize {
        self.prompt_context_length
    }

    // The frontend will block for at most <user_input_poll_duration> waiting for a new input event from the user.
    pub fn user_input_poll_duration(&self) -> Duration {
        self.user_input_poll_duration
    }

    pub fn database_file_path(&self) -> &Path {
        self.database_file_path.as_path()
    }

    pub fn token_limit(&self) -> u32 {
        self.token_limit
    }
}
