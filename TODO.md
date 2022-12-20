# TODO

- I finished the backend refactor, paving the way for "call" mode. Now, I need to update "call"
  mode's Strategy functions to make all the AWS requests.
- Adapt the audio recording example to allow the user to record and playback audio by way of interacting with the frontend. The user should have the option to review and edit the audio transcription before sending it to the bot.
- Replace some `unwrap`s and `expect`s with `Result`s.
- enable manual scrolling of the conversation
- Correctly handle saving new messages (but not old ones) when resuming a conversation
- Enable viewing of old conversations without resuming them
- When starting a new conversation, display the prompt in the conversation box
