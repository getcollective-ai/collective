#![allow(clippy::multiple_crate_versions)]
//! API for `OpenAI`

use std::{
    fmt::{Display, Formatter},
    future::Future,
};

use anyhow::{bail, Context};
use derive_more::Constructor;
use futures_util::{Stream, StreamExt, TryStreamExt};
pub use reqwest;
use reqwest::Response;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use utils::default;

/// Grab the `OpenAI` key from the environment
///
/// # Errors
/// Will return `Err` if the key `OPENAI_KEY` does not exist
#[inline]
pub fn openai_key() -> anyhow::Result<String> {
    std::env::var("OPENAI_KEY").context("no OpenAI key specified")
}

/// The `OpenAI` client
#[derive(Clone)]
pub struct Client {
    client: reqwest::Client,
    api_key: String,
}

impl Client {
    /// Create a new [`Client`] client
    #[must_use]
    pub fn new(client: reqwest::Client, api_key: impl Into<String>) -> Self {
        let api_key = api_key.into();
        Self { client, api_key }
    }

    /// # Errors
    /// Will return `Err` if no `OpenAI` key is defined
    pub fn simple() -> anyhow::Result<Self> {
        let key = openai_key()?;
        Ok(Self::new(reqwest::Client::default(), key))
    }
}

/// ```json
/// {"model": "text-davinci-003", "prompt": "Say this is a test", "temperature": 0, "max_tokens": 7}
/// ```
#[derive(Clone, Serialize)]
pub struct TextRequest<'a> {
    pub model: Completions,
    pub prompt: &'a str,
    pub temperature: f64,

    /// Up to 4 sequences where the API will stop generating further tokens. The returned text will
    /// not contain the stop sequence.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub stop: Vec<&'a str>,

    /// number of completions
    pub n: Option<usize>,
    pub max_tokens: usize,
}

impl Default for TextRequest<'_> {
    fn default() -> Self {
        Self {
            model: Completions::Davinci,
            prompt: "",
            temperature: 0.0,
            stop: Vec::new(),
            n: None,
            max_tokens: 1_000,
        }
    }
}

/// ```json
/// {"input": "Your text string goes here", "model":"text-embedding-ada-002"}
/// ```
#[derive(Copy, Clone, Serialize, Deserialize)]
struct EmbedRequest<'a> {
    input: &'a str,
    model: &'a str,
}

#[derive(Clone, Serialize, Deserialize)]
struct TextResponseChoice {
    text: String,
}

#[derive(Clone, Serialize, Deserialize)]
struct TextResponse {
    choices: Vec<TextResponseChoice>,
}

#[derive(Clone, Serialize, Deserialize)]
struct EmbedDataFrame {
    embedding: Vec<f32>,
}

#[derive(Clone, Serialize, Deserialize)]
struct EmbedResponse {
    data: Vec<EmbedDataFrame>,
}

#[derive(Serialize, Deserialize)]
struct DavinciiData<'a> {
    model: &'a str,
    prompt: &'a str,
    temperature: f64,
    max_tokens: usize,
}

/// The text model we are using. See <https://openai.com/api/pricing/>
#[derive(Copy, Clone, Default, PartialEq, Eq, Debug)]
pub enum Model {
    /// The Davinci model
    #[default]
    Davinci,
    /// The Curie model
    Curie,
    /// The Babbage model
    Babbage,
    /// The Ada model
    Ada,
}

#[derive(Serialize, Deserialize, Default, Debug, PartialEq, Eq)]
pub enum ChatModel {
    #[serde(rename = "gpt-4")]
    #[default]
    Gpt4,
    #[serde(rename = "gpt-3.5-turbo")]
    Turbo,

    #[serde(rename = "gpt-3.5-turbo-0301")]
    Turbo0301,
}

/// ```json
/// {"role": "system", "content": "You are a helpful assistant."},
/// {"role": "user", "content": "Who won the world series in 2020?"},
/// {"role": "assistant", "content": "The Los Angeles Dodgers won the World Series in 2020."},
/// {"role": "user", "content": "Where was it played?"}
/// ```
#[derive(Serialize, Deserialize, Debug, Copy, Clone, PartialOrd, PartialEq, Ord, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    System,
    User,
    Assistant,
}

#[derive(Serialize, Deserialize, Debug, Clone, Constructor)]
pub struct Msg {
    /// Usually
    pub role: Role,
    pub content: String,
}

impl Msg {
    pub fn system(content: impl Into<String>) -> Self {
        Self::new(Role::System, content.into())
    }

    pub fn user(content: impl Into<String>) -> Self {
        Self::new(Role::User, content.into())
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        Self::new(Role::Assistant, content.into())
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub enum Delta {
    /// Usually
    Role(Role),
    Content(String),
}

impl Display for Msg {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.content)
    }
}

#[allow(clippy::trivially_copy_pass_by_ref)]
fn real_is_one(input: &f64) -> bool {
    (*input - 1.0).abs() < f64::EPSILON
}

#[allow(clippy::trivially_copy_pass_by_ref)]
const fn int_is_one(input: &usize) -> bool {
    *input == 1
}

const fn empty<T>(input: &[T]) -> bool {
    input.is_empty()
}

#[derive(Serialize, Debug)]
pub struct ChatRequest {
    pub model: ChatModel,
    pub messages: Vec<Msg>,

    /// What sampling temperature to use, between 0 and 2. Higher values like 0.8 will make the
    /// output more random, while lower values like 0.2 will make it more focused and
    /// deterministic.
    ///
    /// OpenAI generally recommend altering this or top_p but not both.
    #[serde(skip_serializing_if = "real_is_one")]
    pub temperature: f64,

    /// An alternative to sampling with temperature, called nucleus sampling, where the model
    /// considers the results of the tokens with top_p probability mass. So 0.1 means only the
    /// tokens comprising the top 10% probability mass are considered.
    ///
    /// OpenAI generally recommends altering this or temperature but not both.
    #[serde(skip_serializing_if = "real_is_one")]
    pub top_p: f64,

    /// How many chat completion choices to generate for each input message.
    #[serde(skip_serializing_if = "int_is_one")]
    pub n: usize,

    #[serde(skip_serializing_if = "empty")]
    pub stop: Vec<String>,
}

impl ChatRequest {
    pub fn model(self, model: ChatModel) -> Self {
        Self { model, ..self }
    }

    pub fn temperature(self, temperature: f64) -> Self {
        Self {
            temperature,
            ..self
        }
    }

    pub fn message(self, message: Msg) -> Self {
        Self {
            messages: {
                let mut messages = self.messages;
                messages.push(message);
                messages
            },
            ..self
        }
    }

    pub fn top_p(self, top_p: f64) -> Self {
        Self { top_p, ..self }
    }

    pub fn n(self, n: usize) -> Self {
        Self { n, ..self }
    }

    pub fn stop_at(self, stop: impl Into<String>) -> Self {
        Self {
            stop: {
                let mut s = self.stop;
                s.push(stop.into());
                s
            },
            ..self
        }
    }
}

impl<'a> From<&'a str> for ChatRequest {
    fn from(input: &'a str) -> Self {
        Self {
            messages: vec![Msg::user(input)],
            ..default()
        }
    }
}

impl<'a> From<&'a String> for ChatRequest {
    fn from(input: &'a String) -> Self {
        Self::from(input.as_str())
    }
}

// From for ChatRequest with &[ChatMessage]
impl<'a> From<&'a [Msg]> for ChatRequest {
    fn from(input: &'a [Msg]) -> Self {
        Self {
            messages: input.to_vec(),
            ..default()
        }
    }
}

// From for [ChatMessage; N]
impl<const N: usize> From<[Msg; N]> for ChatRequest {
    fn from(input: [Msg; N]) -> Self {
        Self {
            messages: input.to_vec(),
            ..default()
        }
    }
}

impl Default for ChatRequest {
    fn default() -> Self {
        Self {
            model: ChatModel::default(),
            messages: vec![],
            temperature: 1.0,
            top_p: 1.0,
            n: 1,
            stop: Vec::new(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ChatChoice {
    pub message: Msg,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ChatResponse {
    pub id: String,
    pub object: String,
    pub created: u64,
    pub choices: Vec<ChatChoice>,
}

/// The text model we are using. See <https://openai.com/api/pricing/>
#[derive(Deserialize, Serialize, Copy, Clone, Default, Eq, PartialEq, Debug)]
#[allow(unused)]
pub enum Completions {
    /// The Davinci model
    #[serde(rename = "text-davinci-003")]
    #[default]
    Davinci,

    /// The Curie model
    #[serde(rename = "text-curie-001")]
    Curie,
    /// The Babbage model
    #[serde(rename = "text-babbage-001")]
    Babbage,
    /// The Ada model
    #[serde(rename = "text-ada-001")]
    Ada,
}

impl Model {
    const fn embed_repr(self) -> Option<&'static str> {
        match self {
            Self::Davinci | Self::Curie | Self::Babbage => None,
            Self::Ada => Some("text-embedding-ada-002"),
        }
    }

    #[allow(unused)]
    const fn text_repr(self) -> &'static str {
        match self {
            Self::Davinci => "text-davinci-003",
            Self::Curie => "text-curie-001",
            Self::Babbage => "text-babbage-001",
            Self::Ada => "text-ada-001",
        }
    }
}

impl Client {
    fn request(
        &self,
        url: &str,
        request: impl Serialize,
    ) -> impl Future<Output = reqwest::Result<Response>> {
        self.client
            .post(url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&request)
            .send()
    }

    /// Calls the embedding API
    ///
    /// - turns an `input` [`str`] into a vector
    ///
    /// # Errors
    /// Returns `Err` if there is a network error communicating to `OpenAI`
    pub async fn embed(&self, input: &str) -> anyhow::Result<Vec<f32>> {
        let request = EmbedRequest {
            input,
            model: unsafe { Model::Ada.embed_repr().unwrap_unchecked() },
        };

        let embed: EmbedResponse = self
            .request("https://api.openai.com/v1/embeddings", request)
            .await
            .context("could not complete embed request")?
            .json()
            .await?;

        let result = embed
            .data
            .into_iter()
            .next()
            .context("no data for embedding")?
            .embedding;

        Ok(result)
    }

    /// # Errors
    /// Returns `Err` if there is a network error communicating to `OpenAI`
    pub async fn raw_chat(&self, req: ChatRequest) -> anyhow::Result<ChatResponse> {
        let response: String = self
            .request("https://api.openai.com/v1/chat/completions", req)
            .await
            .context("could not complete chat request")?
            .text()
            .await?;

        let response = match serde_json::from_str(&response) {
            Ok(response) => response,
            Err(e) => {
                return Err(anyhow::anyhow!(
                    "could not parse chat response {response}: {e}"
                ));
            }
        };

        Ok(response)
    }

    /// # Errors
    /// Returns `Err` if there is a network error communicating to `OpenAI`
    pub async fn chat(&self, req: impl Into<ChatRequest>) -> anyhow::Result<String> {
        let req = req.into();
        let response = self.raw_chat(req).await?;
        let choice = response
            .choices
            .into_iter()
            .next()
            .context("no choices for chat")?;

        Ok(choice.message.content)
    }

    /// # Errors
    /// Returns `Err` if there is a network error communicating to `OpenAI`
    pub async fn stream_text(
        &self,
        req: TextRequest<'_>,
    ) -> anyhow::Result<impl Stream<Item = anyhow::Result<String>>> {
        #[derive(Clone, Serialize)]
        pub struct TextStreamRequest<'a> {
            stream: bool,

            #[serde(flatten)]
            req: TextRequest<'a>,
        }

        #[derive(Deserialize, Debug)]
        pub struct TextStreamData {
            pub text: Option<String>,
        }

        #[derive(Deserialize, Debug)]
        pub struct TextStreamResponse {
            pub choices: Vec<TextStreamData>,
        }

        let req = TextStreamRequest { stream: true, req };

        let response = self
            .request("https://api.openai.com/v1/completions", req)
            .await
            .context("could not complete chat request")?;

        let stream = response
            .bytes_stream()
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
            .into_async_read();

        let mut messages = event_stream_processor::get_messages(stream);

        let (tx, rx) = mpsc::channel(100);

        fn message_to_data(
            message: anyhow::Result<event_stream_processor::Message>,
        ) -> anyhow::Result<Option<String>> {
            let message = message?;
            let data = message.data.context("no data")?;

            if &data == "[DONE]" {
                return Ok(None);
            }

            let Ok(data) = serde_json::from_str::<TextStreamResponse>(&data) else {
                return Ok(None);
            };

            let choice = data.choices.into_iter().next().context("no choices")?;

            let Some(content) = choice.text else {
                return Ok(Some(String::new()));
            };

            Ok(Some(content))
        }

        tokio::spawn(async move {
            while let Some(msg) = messages.next().await {
                let msg = message_to_data(msg);
                match msg {
                    Ok(None) => {
                        return;
                    }
                    Ok(Some(msg)) => {
                        if tx.send(Ok(msg)).await.is_err() {
                            return;
                        }
                    }
                    Err(e) => {
                        if tx.send(Err(e)).await.is_err() {
                            return;
                        }
                    }
                }
            }
        });

        Ok(ReceiverStream::from(rx))
    }

    /// # Errors
    /// Returns `Err` if there is a network error communicating to `OpenAI`
    pub async fn stream_chat(
        &self,
        req: ChatRequest,
    ) -> anyhow::Result<impl Stream<Item = anyhow::Result<String>>> {
        #[derive(Serialize)]
        struct ChatStreamRequest {
            stream: bool,

            #[serde(flatten)]
            req: ChatRequest,
        }

        #[derive(Serialize, Deserialize, Debug, Clone)]
        struct ChatStreamMessage {
            pub delta: Delta,
        }

        #[derive(Serialize, Deserialize, Debug, Clone)]
        struct ChatStreamResponse {
            pub choices: Vec<ChatStreamMessage>,
        }

        let req = ChatStreamRequest { stream: true, req };

        let response = self
            .request("https://api.openai.com/v1/chat/completions", req)
            .await
            .context("could not complete chat request")?;

        let stream = response
            .bytes_stream()
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
            .into_async_read();

        let mut messages = event_stream_processor::get_messages(stream);

        let (tx, rx) = mpsc::channel(100);

        fn message_to_data(
            message: anyhow::Result<event_stream_processor::Message>,
        ) -> anyhow::Result<Option<String>> {
            let message = message?;
            let data = message.data.context("no data")?;

            if &data == "[DONE]" {
                return Ok(None);
            }

            let Ok(data) = serde_json::from_str::<ChatStreamResponse>(&data) else {
                return Ok(None);
            };

            let choice = data.choices.into_iter().next().context("no choices")?;

            let Delta::Content(content) = choice.delta else {
                return Ok(Some(String::new()));
            };

            Ok(Some(content))
        }

        tokio::spawn(async move {
            while let Some(msg) = messages.next().await {
                let msg = message_to_data(msg);
                match msg {
                    Ok(None) => {
                        return;
                    }
                    Ok(Some(msg)) => {
                        if tx.send(Ok(msg)).await.is_err() {
                            return;
                        }
                    }
                    Err(e) => {
                        if tx.send(Err(e)).await.is_err() {
                            return;
                        }
                    }
                }
            }
        });

        Ok(ReceiverStream::from(rx))
    }

    /// # Errors
    /// Will return `Err` if cannot properly contact `OpenAI` API.
    pub async fn text(&self, request: TextRequest<'_>) -> anyhow::Result<Vec<String>> {
        let text = self
            .request("https://api.openai.com/v1/completions", request)
            .await
            .context("could not complete text request")?
            .text()
            .await
            .context("could not convert into text")?;

        let json: TextResponse = match serde_json::from_str(&text) {
            Ok(res) => res,
            Err(e) => bail!("error {e} parsing json {text}"),
        };

        let choices = json.choices.into_iter().map(|e| e.text).collect();
        Ok(choices)
    }
}

#[cfg(test)]
mod tests {
    use futures_util::TryStreamExt;
    use once_cell::sync::Lazy;
    use pretty_assertions::assert_eq;

    use crate::{ChatChoice, ChatModel, ChatRequest, Completions, Model, Msg, Role};

    static API: Lazy<crate::Client> =
        Lazy::new(|| crate::Client::simple().expect("could not create client"));

    #[tokio::test]
    async fn test_chat_raw() {
        let req = ChatRequest {
            model: ChatModel::Turbo,
            messages: vec![
                Msg {
                    role: Role::System,
                    content: "You are a helpful assistant that translates English to French."
                        .to_string(),
                },
                Msg {
                    role: Role::User,
                    content: "Translate the following English text to French: Hello".to_string(),
                },
            ],
            ..ChatRequest::default()
        };

        let choices = API.raw_chat(req).await.unwrap().choices;

        let [ChatChoice { message }] = choices.as_slice() else {
            panic!("no choices");
        };

        let message = message
            // prune all non-alphanumeric characters
            .content
            .replace(|c: char| !c.is_ascii_alphanumeric(), "")
            .to_ascii_lowercase();

        assert_eq!(message, "bonjour");
    }

    #[tokio::test]
    async fn test_chat() {
        let req = ChatRequest {
            model: ChatModel::Turbo,
            messages: vec![
                Msg {
                    role: Role::System,
                    content: "You are a helpful assistant that translates English to French."
                        .to_string(),
                },
                Msg {
                    role: Role::User,
                    content: "Translate the following English text to French: Hello".to_string(),
                },
            ],
            ..ChatRequest::default()
        };

        let res = API.chat(req).await.unwrap();

        let choice = res
            // prune all non-alphanumeric characters
            .replace(|c: char| !c.is_ascii_alphanumeric(), "")
            .to_ascii_lowercase();

        assert_eq!(choice, "bonjour");
    }

    /// test no panic
    #[test]
    fn test_text_request() {
        // test default does not panic
        let a = crate::TextRequest::default();
    }

    #[test]
    fn test_message() {
        {
            let msg = Msg::system("hello");
            assert_eq!("hello", format!("{}", msg));
            let msg = serde_json::to_string(&msg).unwrap();
            assert_eq!(msg, r#"{"role":"system","content":"hello"}"#);
        }

        {
            let msg = Msg::user("hello");
            assert_eq!("hello", format!("{}", msg));
            let msg = serde_json::to_string(&msg).unwrap();
            assert_eq!(msg, r#"{"role":"user","content":"hello"}"#);
        }

        {
            let msg = Msg::assistant("hello");
            assert_eq!("hello", format!("{}", msg));
            let msg = serde_json::to_string(&msg).unwrap();
            assert_eq!(msg, r#"{"role":"assistant","content":"hello"}"#);
        }
    }

    #[test]
    fn test_chat_builder() {
        let req = ChatRequest::default()
            .model(ChatModel::Turbo)
            .temperature(1.2)
            .message(Msg::system("hello"))
            .message(Msg::user("hello"))
            .top_p(1.0)
            .n(3)
            .stop_at("\n")
            .stop_at("#####");

        assert_eq!(req.model, ChatModel::Turbo);
        assert_eq!(req.temperature, 1.2);
        assert_eq!(req.messages.len(), 2);
        assert_eq!(req.top_p, 1.0);
        assert_eq!(req.n, 3);
        assert_eq!(req.stop, vec!["\n", "#####"]);
    }

    #[test]
    fn test_chat_from() {
        let req = ChatRequest::from("hello");
        assert_eq!(req.messages.len(), 1);
        assert_eq!(req.messages[0].content, "hello");
        assert_eq!(req.messages[0].role, Role::User);
        assert_eq!(req.n, 1);

        let req = ChatRequest::from(&"hello".to_string());
        assert_eq!(req.messages.len(), 1);
        assert_eq!(req.messages[0].content, "hello");
        assert_eq!(req.messages[0].role, Role::User);
        assert_eq!(req.n, 1);

        let messages = [Msg::user("hello"), Msg::assistant("world")];
        let req = ChatRequest::from(messages.as_slice());
        assert_eq!(req.messages.len(), 2);
        assert_eq!(req.messages[0].content, "hello");
        assert_eq!(req.messages[0].role, Role::User);
        assert_eq!(req.messages[1].content, "world");
        assert_eq!(req.messages[1].role, Role::Assistant);
        assert_eq!(req.n, 1);

        let messages = [Msg::user("hello"), Msg::assistant("world")];
        let req = ChatRequest::from(messages);
        assert_eq!(req.messages.len(), 2);
        assert_eq!(req.messages[0].content, "hello");
        assert_eq!(req.messages[0].role, Role::User);
        assert_eq!(req.messages[1].content, "world");
        assert_eq!(req.messages[1].role, Role::Assistant);
        assert_eq!(req.n, 1);
    }

    #[test]
    fn test_completions() {
        let completion = Completions::default();
        assert_eq!(completion, Completions::Davinci);
    }

    #[test]
    fn test_chat_model() {
        let model = ChatModel::default();
        assert_eq!(model, ChatModel::Gpt4);
    }

    #[test]
    fn test_model() {
        let model = Model::default();
        assert_eq!(model, Model::Davinci);
        assert_eq!(model.embed_repr(), None);
        assert_eq!(model.text_repr(), "text-davinci-003");

        let model = Model::Curie;
        assert_eq!(model.embed_repr(), None);
        assert_eq!(model.text_repr(), "text-curie-001");

        let model = Model::Babbage;
        assert_eq!(model.embed_repr(), None);
        assert_eq!(model.text_repr(), "text-babbage-001");

        let model = Model::Ada;
        assert_eq!(model.embed_repr().unwrap(), "text-embedding-ada-002");
        assert_eq!(model.text_repr(), "text-ada-001")
    }

    #[tokio::test]
    async fn test_embed() {
        let embed_response = API.embed("hello").await.unwrap();
        // the amount of output dimensions
        assert_eq!(embed_response.len(), 1536);
    }

    #[tokio::test]
    async fn test_chat_stream() {
        let req = ChatRequest {
            model: ChatModel::Turbo,
            messages: vec![
                Msg {
                    role: Role::System,
                    content: "You are a helpful assistant".to_string(),
                },
                Msg {
                    role: Role::User,
                    content: "Translate 'bonjour' to English".to_string(),
                },
            ],
            ..ChatRequest::default()
        };

        let choices = API.stream_chat(req).await.unwrap();

        // convert choices to a vector
        let choices: Vec<_> = choices.try_collect().await.unwrap();
        let choices = choices.join("\n");
        assert!(!choices.is_empty());
    }
}
