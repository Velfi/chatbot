use anyhow::Context;
use std::{
    env,
    path::{Path, PathBuf},
    time::Duration,
};

const DEFAULT_PROMPT: &str = "The following is a conversation that 'User' is having with an AI assistant named 'Bot'. The assistant is helpful, creative, clever, and very friendly.";
const DEFAULT_YOUR_NAME: &str = "User";
const DEFAULT_THEIR_NAME: &str = "Bot";
const DEFAULT_MODEL_NAME: &str = "text-davinci-003";
// How many words the model should generate? TODO verify this comment
const DEFAULT_TOKEN_LIMIT: u32 = 100;
const DEFAULT_EXPECTED_RESPONSE_TIME: Duration = Duration::from_secs(5);

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
    pub fn new() -> Result<Self, anyhow::Error> {
        let your_name = env::var("YOUR_NAME").unwrap_or(DEFAULT_YOUR_NAME.to_owned());
        let their_name = env::var("THEIR_NAME").unwrap_or(DEFAULT_THEIR_NAME.to_owned());
        let starting_prompt = env::var("STARTING_PROMPT").unwrap_or(DEFAULT_PROMPT.to_owned());
        let openai_model_name =
            env::var("OPENAI_MODEL_NAME").unwrap_or(DEFAULT_MODEL_NAME.to_owned());
        let expected_response_time = env::var("EXPECTED_RESPONSE_TIME")
            .context("checking for expected_response_time in env")
            .and_then(|t| {
                t.parse()
                    .context("parsing expected_response_time from env")
                    .map(|millis| Duration::from_millis(millis))
            })
            .unwrap_or(DEFAULT_EXPECTED_RESPONSE_TIME);
        let prompt_context_length = env::var("PROMPT_CONTEXT_LENGTH")
            .context("checking for expected_response_time in env")
            .and_then(|t| t.parse().context("parsing prompt_context_length from env"))
            .unwrap_or(5);
        let database_file_path = PathBuf::from(
            env::var("DATABASE_FILE_PATH")
                .as_deref()
                .unwrap_or("chatbot.db"),
        );
        let user_input_poll_duration = env::var("USER_INPUT_POLL_DURATION")
            .context("checking for user_input_poll_duration in env")
            .and_then(|t| {
                t.parse()
                    .context("parsing user_input_poll_duration from env")
                    .map(|millis| Duration::from_millis(millis))
            })
            .unwrap_or(Duration::from_millis(10));
        let token_limit = env::var("RESPONSE_TOKEN_LIMIT")
            .context("checking for response_token_limit in env")
            .and_then(|t| t.parse().context("parsing response_token_limit from env"))
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
