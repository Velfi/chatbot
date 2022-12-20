use super::State;
use crate::{
    app::{
        backend::state::{check_for_bot_response, Inner},
        Event, TurnToSpeak,
    },
    aws_api::{speech_to_text, text_to_speech, upload_speech},
    message::create_prompt_from_messages,
    openai_api::fetch_response_to_prompt,
};
use std::{mem, time::Instant};
use tokio::sync::oneshot;
use tracing::{debug, trace};

pub fn handle_backend_events(state: &mut State) -> Result<(), anyhow::Error> {
    trace!("handling backend events...");

    // Run event handling logic shared between the text and call backends
    state.handle_backend_events()
}

pub fn run_bot_state_machine(state: &mut State) -> Result<(), anyhow::Error> {
    match &mut state.inner {
        Inner::SendRequest => {
            trace!("handling bot's turn...");
            let id = state.conversation.len() as u64;
            let prompt = create_prompt_from_messages(
                state.env.starting_prompt(),
                &state.conversation,
                state.env.prompt_context_length(),
            );
            let their_name = state.env.their_name().to_owned();
            let openai_model_name = state.env.openai_model_name().to_owned();
            let token_limit = state.env.token_limit();

            let (tx, rx) = oneshot::channel();

            tokio::spawn(async move {
                let config = aws_config::load_from_env().await;
                let s3 = &aws_sdk_s3::Client::new(&config);
                let transcribe = &aws_sdk_transcribe::Client::new(&config);
                let _polly = &aws_sdk_polly::Client::new(&config);

                let _res = upload_speech(s3, ()).await.unwrap();
                // TODO send event to frontend saying that the audio is being processed
                //    and noting what the user said.
                let _res = speech_to_text(transcribe, ()).await.unwrap();
                let res = fetch_response_to_prompt(
                    id,
                    prompt,
                    their_name,
                    openai_model_name,
                    token_limit,
                )
                .await
                // TODO don't unwrap here
                .unwrap();

                tx.send(res).unwrap();
            });

            state.inner = Inner::LoadingBotResponse {
                start_time: Instant::now(),
                rx,
            };
        }
        Inner::LoadingBotResponse { start_time, rx } => {
            trace!("loading bot response...");
            if start_time.elapsed() > state.env.expected_response_time() {
                trace!(
                    "{} is taking longer than {:?} to respond",
                    state.env.their_name(),
                    state.env.expected_response_time()
                );
                let start_time = *start_time;
                // TODO is this really necessary?
                let rx = mem::replace(rx, oneshot::channel().1);
                state.inner = Inner::TakingAWhileToLoadBotResponse { start_time, rx };

                return Ok(());
            }

            state
                .frontend_tx
                .send(Event::StatusUpdated(
                    "Waiting for bot's response".to_owned(),
                ))
                .map_err(|e| {
                    anyhow::anyhow!("failed to notify frontend of status update: {}", e)
                })?;

            // TODO this code is copied in the below handler, how can this be avoided?
            if let Some(message) = check_for_bot_response(state.env.their_name(), rx) {
                state.conversation.push(message);
                state
                    .frontend_tx
                    .send(Event::ConversationUpdated(state.conversation.clone()))
                    .map_err(|e| {
                        anyhow::anyhow!("failed to notify frontend of conversation update: {e}")
                    })?;
                state
                    .frontend_tx
                    .send(Event::StatusUpdated(format!(
                        "Bot responded in {:?}",
                        start_time.elapsed()
                    )))
                    .map_err(|e| {
                        anyhow::anyhow!("failed to notify frontend of conversation update: {e}")
                    })?;
                state.turn_to_speak = TurnToSpeak::User;
                state.inner = Inner::SendRequest;
            }
        }
        Inner::TakingAWhileToLoadBotResponse { start_time, rx } => {
            trace!("loading bot response (taking a while)...");
            state
                .frontend_tx
                .send(Event::StatusUpdated(format!(
                    "Waiting for bot's response, It's taking a while ({}s)",
                    start_time.elapsed().as_secs()
                )))
                .map_err(|e| {
                    anyhow::anyhow!("failed to notify frontend of status update: {}", e)
                })?;

            if let Some(message) = check_for_bot_response(state.env.their_name(), rx) {
                debug!("received response from {}", state.env.their_name());
                state.conversation.push(message);
                state
                    .frontend_tx
                    .send(Event::ConversationUpdated(state.conversation.clone()))
                    .map_err(|e| {
                        anyhow::anyhow!("failed to notify frontend of conversation update: {}", e)
                    })?;
                state
                    .frontend_tx
                    .send(Event::StatusUpdated(format!(
                        "Bot slowly responded in {:?}",
                        start_time.elapsed()
                    )))
                    .map_err(|e| {
                        anyhow::anyhow!("failed to notify frontend of conversation update: {e}")
                    })?;
                state.turn_to_speak = TurnToSpeak::User;
                state.inner = Inner::SendRequest;
            }
        }
    }

    Ok(())
}
