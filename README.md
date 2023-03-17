# Chatbot

A terminal-based client for talking to OpenAI's GPT models.

## Getting Started

An Open AI account is necessary to talk to the bot. Once you have that, you'll need to set some
configuration. Either set these in environment variables or create a .env file containing them:

```env
OPENAI_API_KEY=<your api key>
OPENAI_ORGANIZATION_ID=<your org id>
```

* *You can find your organization ID [here][organization-id].*
* *You can find your API key(s) [here][API-key].*

Once those are set, run the app and start chatting:

```sh
cargo run
```

To exit the app when you're done talking, hit ESC. Your conversation will be saved to a SQLite database in the app directory. Next time you start the app you can pick up where you left off.

### Costs

Using this app will cost a small amount of money, based on your usage of the OpenAI API.
Specifically, you'll be paying for a language model, the prices of which are found [here](pricing).

> Prices are per 1,000 tokens. You can think of tokens as pieces of words, where 1, 000 tokens is
> about 750 words.

The default model used by this app, Davinci, costs 2¢ per 1000 tokens. There are cheaper models, but
I find their capability to be lacking and I don't recommend them. The total number of tokens used
whenever you press ENTER varies.

> Completions requests are billed based on the number of tokens sent in your prompt plus the number
> of tokens in the completion(s) returned by the API.

Each request includes a customizable prompt and several of the last messages between you and the
bot. During testing, my conversations used, on average:

* 250 tokens for prompts
* 70 tokens for bot responses

With a combined average of 320 tokens, that's a cost of 0.64¢ per ENTER press.

[You can read more on Completions pricing here](completions-pricing).

### Advanced Configuration

This app supports configuration of lots of stuff through environment variables.

<table>
  <thead>
      <tr>
          <th>Environment Variable</th>
          <th>Default</th>
          <th>Description</th>
      </tr>
  </thead>
  <tr>
    <td>DATABASE_FILE_PATH</td>
    <td>"chatbot.db"</td>
    <td>The file comprising your chat log database. A new one will be created if none exists.</td>
  </tr>
  <tr>
    <td>EXPECTED_RESPONSE_TIME</td>
    <td>5 seconds</td>
    <td>
      The amount of time to wait before considering the bot's response late.
      This only affects the the status update in the lower right.
    </td>
  </tr>
  <tr>
    <td>OPENAI_API_KEY</td>
    <td><em>(required)</em></td>
    <td>An OpenAI API key. Find your key(s) [here](API-key).</td>
  </tr>
  <tr>
    <td>OPENAI_MODEL_NAME</td>
    <td>"text-davinci-003"</td>
    <td>The GPT model to use for txt completions. Find more models [here](models).</td>
  </tr>
  <tr>
    <td>OPENAI_ORGANIZATION_ID</td>
    <td><em>(required)</em></td>
    <td>An OpenAI Organization ID. Find yours [here](organization-id).</td>
  </tr>
  <tr>
    <td>PROMPT_CONTEXT_LENGTH</td>
    <td>5</td>
    <td>The number of chat messages to send as part of the prompt. Longer lengths will give the bot more context but will cost more money.
  </tr>
  <tr>
    <td>RESPONSE_TOKEN_LIMIT</td>
    <td>100</td>
    <td>The maximum number of tokens to generate for the bot's response. *[OpenAI docs](max-tokens)*</td>
  </tr>
  <tr>
    <td>STARTING_PROMPT</td>
    <td>"The following is a conversation that 'User' is having with an AI assistant named 'Bot'. The assistant is helpful, creative, clever, and very friendly."
    <td>The prompt that will be prepended to the last few chat messages to fetch the bot's response. See [here](prompt-design) for prompt design tips.
  </tr>
  <tr>
    <td>THEIR_NAME</td>
    <td>"Bot"</td>
    <td>A name representing the bot that you're talking to in chat logs.</td>
  </tr>
  <tr>
    <td>USER_INPUT_POLL_DURATION</td>
    <td>10 milliseconds</td>
    <td>
      How long to wait when polling for user input. The longer this is, the less resources it takes
      to run the app, but the more laggy typing feels. 
    </td>
  </tr>
  <tr>
    <td>YOUR_NAME</td>
    <td>"User"</td>
    <td>A name representing you, the user, in chat logs.</td>
  </tr>
</table>

## If you have issues and want to debug them

This app uses `tracing` to record logs. If you run the app with `RUST_LOG=trace` then it'll write
logs to a file called 'debug.log'. You can `tail` that log to see live updates.

[models]: https://beta.openai.com/docs/models/gpt-3
[tokenizer]: https://beta.openai.com/tokenizer
[API-key]: https://beta.openai.com/account/api-keys
[organization-id]: https://beta.openai.com/account/org-settings
[pricing]: https://openai.com/api/pricing/
[completions-pricing]: https://openai.com/api/pricing/#faq-completions-pricing
[max-tokens]: https://beta.openai.com/docs/api-reference/completions/create#completions/create-max_tokens
[prompt-design]: https://beta.openai.com/docs/guides/completion/prompt-design
