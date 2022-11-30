mod text_completion;

use crate::{
    message::Message, openai_api::text_completion::TextCompletionResponse, state::State, BOT_NAME,
};
use once_cell::sync::Lazy;
use std::env;
use text_completion::TextCompletionRequest;
use tracing::{debug, trace};

const COMPLETIONS_URI: &str = "https://api.openai.com/v1/completions";
static OPENAI_API_KEY: Lazy<String> = Lazy::new(|| std::env::var("OPENAI_API_KEY").unwrap());
static OPENAI_ORGANIZATION_ID: Lazy<String> =
    Lazy::new(|| std::env::var("OPENAI_ORGANIZATION_ID").unwrap());

pub struct Client {
    inner: reqwest::Client,
}

impl Client {
    pub fn new() -> Self {
        Self {
            inner: reqwest::Client::new(),
        }
    }

    // TODO this is fallible and should return a result
    pub async fn fetch_next_response_to_messages(&self, state: &State) -> Message {
        let prompt = create_prompt_from_messages(&state.messages);
        let body = TextCompletionRequest::builder().prompt(prompt).build();

        let res = self
            .inner
            .post(COMPLETIONS_URI)
            .bearer_auth(OPENAI_API_KEY.as_str())
            .header("Content-Type", "application/json")
            .header("OpenAI-Organization", OPENAI_ORGANIZATION_ID.as_str())
            .json(&body)
            .send()
            .await
            // TODO gracefully handle errors
            .expect("request is valid");

        trace!(
            response = ?res
        );

        let body: TextCompletionResponse = res.json().await.unwrap();

        let message = Message {
            id: state.next_message_id(),
            sender: BOT_NAME.into(),
            content: body.message(),
            timestamp: chrono::Utc::now(),
        };

        trace!(
            message.timestamp = message.timestamp.to_rfc2822().as_str(),
            message.id = message.id,
            message.content = message.content,
            "bot sent message"
        );

        message
    }

    pub async fn list_models(&self) {
        let res = self
            .inner
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
}

fn create_prompt_from_messages(messages: &[Message]) -> String {
    // Really, the final prompt will be longer than this due to also including a starting prompt,
    // names, and timestamps, but this is a good starting point.
    let messages_len = messages.iter().map(|m| m.content.len()).sum::<usize>();
    let mut prompt = String::with_capacity(messages_len);

    if let Ok(starting_prompt) = env::var("STARTING_PROMPT") {
        prompt.push_str(&starting_prompt);
        prompt.push('\n')
    }

    for message in messages {
        prompt.push_str(
            format!(
                "{}: {}\n{}\n",
                &message.sender,
                &message.timestamp.to_rfc2822(),
                &message.content
            )
            .as_str(),
        );
    }

    prompt
}

#[cfg(test)]
mod tests {
    use super::Client;
    use crate::state::State;
    use std::sync::Once;

    static INIT: Once = Once::new();
    fn init() {
        INIT.call_once(|| {
            dotenv::dotenv().unwrap();
            tracing_subscriber::fmt::init();
        });
    }

    #[tokio::test]
    #[ignore = "This requires an actual API key and will cause charges to be applied to your account."]
    async fn test_fetch_next_response_to_messages() {
        init();

        let state = State::load();
        let client = Client::new();
        let _res = client.fetch_next_response_to_messages(&state).await;
    }
}
