use chrono::serde::ts_seconds;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::env;

// How many words the model should generate? TODO verify this comment
const DEFAULT_NUMBER_OF_TOKENS: u32 = 40;
// The most expensive model, but also the most powerful
const DEFAULT_MODEL: &str = "text-davinci-003";

#[derive(Debug, Serialize)]
pub struct TextCompletionRequest {
    pub prompt: Cow<'static, str>,
    pub model: Cow<'static, str>,
    pub temperature: f32,
    pub max_tokens: u32,
}

impl TextCompletionRequest {
    pub fn builder() -> TextCompletionRequestBuilder {
        Default::default()
    }
}

#[derive(Default)]
pub struct TextCompletionRequestBuilder {
    prompt: Option<Cow<'static, str>>,
    model: Option<Cow<'static, str>>,
    temperature: Option<f32>,
    max_tokens: Option<u32>,
}

impl TextCompletionRequestBuilder {
    pub fn prompt(mut self, prompt: impl Into<Cow<'static, str>>) -> Self {
        self.prompt = Some(prompt.into());
        self
    }

    pub fn model(mut self, model: impl Into<Cow<'static, str>>) -> Self {
        self.model = Some(model.into());
        self
    }

    pub fn temperature(mut self, temperature: f32) -> Self {
        self.temperature = Some(temperature);
        self
    }

    pub fn max_tokens(mut self, max_tokens: u32) -> Self {
        self.max_tokens = Some(max_tokens);
        self
    }

    pub fn build(self) -> TextCompletionRequest {
        TextCompletionRequest {
            prompt: self.prompt.expect("prompt is required"),
            // Use model from builder,
            //     or else model from env var,
            //     or else default model
            model: self
                .model
                .or(env::var("OPENAI_GPT_MODEL_NAME").ok().map(Into::into))
                .unwrap_or(DEFAULT_MODEL.into()),
            temperature: self.temperature.unwrap_or_default(),
            max_tokens: self.max_tokens.unwrap_or(DEFAULT_NUMBER_OF_TOKENS),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct TextCompletionResponse {
    id: String,
    object: String,
    #[serde(with = "ts_seconds")]
    created: DateTime<Utc>,
    model: String,
    choices: Vec<Choice>,
    usage: Usage,
}

impl TextCompletionResponse {
    pub fn message(&self) -> String {
        self.choices
            .first()
            .expect("choices is not empty")
            .text
            .trim()
            .to_owned()
    }
}

#[derive(Debug, Deserialize)]
struct Choice {
    text: String,
    index: u32,
    logprobs: Option<u32>,
    finish_reason: String,
}

#[derive(Debug, Deserialize)]
struct Usage {
    prompt_tokens: u32,
    completion_tokens: u32,
    total_tokens: u32,
}
