mod text_completion;

use crate::{message::Message, openai_api::text_completion::TextCompletionResponse};
use once_cell::sync::Lazy;
use text_completion::TextCompletionRequest;
use tracing::{debug, instrument};

const COMPLETIONS_URI: &str = "https://api.openai.com/v1/completions";
static OPENAI_API_KEY: Lazy<String> = Lazy::new(|| std::env::var("OPENAI_API_KEY").unwrap());
static OPENAI_ORGANIZATION_ID: Lazy<String> =
    Lazy::new(|| std::env::var("OPENAI_ORGANIZATION_ID").unwrap());

// TODO this is fallible and should return a result
#[instrument]
pub async fn fetch_response_to_prompt(
    id: u64,
    prompt: String,
    their_name: String,
    model: String,
    max_tokens: u32,
) -> Result<Message, anyhow::Error> {
    let client = reqwest::Client::new();
    let body = TextCompletionRequest::builder()
        .prompt(prompt.to_owned())
        .model(model.to_owned())
        .max_tokens(max_tokens)
        .build()?;

    debug!(?body, "sending request to OpenAI Completions API...");
    let res = client
        .post(COMPLETIONS_URI)
        .bearer_auth(OPENAI_API_KEY.as_str())
        .header("Content-Type", "application/json")
        .header("OpenAI-Organization", OPENAI_ORGANIZATION_ID.as_str())
        .json(&body)
        .send()
        .await
        // TODO gracefully handle errors
        .expect("request is valid");

    debug!(
        response = ?res,
        "received response from OpenAI Completions API"
    );

    let body: TextCompletionResponse = res.json().await.unwrap();
    // Sometimes the bot will prefix responses with it's name. We want to remove that since we
    // handle that in the UI.
    // TODO don't unwrap here
    let content = body
        .message()
        .trim_start_matches(&format!("{their_name}:"))
        .to_owned();

    let message = Message {
        id,
        sender: their_name.into(),
        content,
        timestamp: chrono::Utc::now(),
    };

    debug!(
        message.timestamp = message.timestamp.to_rfc2822().as_str(),
        message.id = message.id,
        message.content = message.content,
        "bot sent message"
    );

    Ok(message)
}

pub async fn list_models() {
    let client = reqwest::Client::new();
    let res = client
        .get("https://api.openai.com/v1/models")
        .bearer_auth(OPENAI_API_KEY.as_str())
        .header("OpenAI-Organization", OPENAI_ORGANIZATION_ID.as_str())
        .send()
        .await
        .expect("request is valid");

    debug!("{:#?}", res);
    let res_body = res.bytes().await.expect("response body is valid");
    let body_str = String::from_utf8(res_body.to_vec()).expect("response body is valid utf8");
    debug!("{body_str}");
}

// TODO test these APIs somehow
