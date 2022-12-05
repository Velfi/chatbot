# Chatbot

A personal chatbot built on top of OpenAI's GPT-3 API.

## Getting Started

Either set these environment variables or create a .env file containing them:
```env
OPENAI_API_KEY=<your api key>
OPENAI_ORGANIZATION_ID=<your org id>
OPENAI_GPT_MODEL_NAME=<the model to send questions to>
STARTING_PROMPT=<a prompt to get the bot in the right mindset>
```

Valid models are listed [here][models]. If none is set, then `text-davinci-003` will be used.
For a prompt, I've been using 'Two long-time friends have been conversing with one another. One is named "Bot" and the other is name "User".'

Once those things are set, just run the app and start chatting. To exit the app when you're done
talking, hit ESC.

### Costs

Talking to the bot costs about 1c per minute of talking to the bot.

## If you have issues and want to debug them

This app uses `tracing` to record logs. If you run the app with `RUST_LOG=trace` then it'll write
logs to a file called 'debug.log'.

[models]: https://beta.openai.com/docs/models/gpt-3
[tokenizer]: https://beta.openai.com/tokenizer