use serde::{Deserialize, Serialize};
use std::borrow::Cow;

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

    pub fn _temperature(mut self, temperature: f32) -> Self {
        self.temperature = Some(temperature);
        self
    }

    /// Set the maximum number of tokens to generate.
    ///
    /// A helpful rule of thumb is that one token generally corresponds to ~4 characters of text for
    /// common English text. This translates to roughly ¾ of a word (so 100 tokens ~= 75 words).
    ///
    /// See [this doc](https://beta.openai.com/tokenizer) for more information on tokens.
    pub fn max_tokens(mut self, max_tokens: u32) -> Self {
        self.max_tokens = Some(max_tokens);
        self
    }

    pub fn build(self) -> Result<TextCompletionRequest, anyhow::Error> {
        Ok(TextCompletionRequest {
            prompt: self.prompt.expect("prompt is required"),
            model: self
                .model
                .map(Into::into)
                .ok_or_else(|| anyhow::anyhow!("model is required"))?,
            temperature: self.temperature.unwrap_or_default(),
            max_tokens: self
                .max_tokens
                .ok_or_else(|| anyhow::anyhow!("max_tokens is required"))?,
        })
    }
}

#[derive(Debug, Deserialize)]
pub struct TextCompletionResponse {
    // id: String,
    // object: String,
    // #[serde(with = "ts_seconds")]
    // created: DateTime<Utc>,
    // model: String,
    choices: Vec<Choice>,
    // usage: Usage,
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
    // index: u32,
    // logprobs: Option<u32>,
    // finish_reason: String,
}

// #[derive(Debug, Deserialize)]
// struct Usage {
//     prompt_tokens: u32,
//     completion_tokens: u32,
//     total_tokens: u32,
// }
