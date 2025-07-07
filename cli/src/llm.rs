use std::{borrow::Cow, env};

use serde::{Deserialize, Serialize};

const LLM_URL_ENV_VAR: &str = "STYLUS_PORT_LLM_URL";
const LLM_URL_API_KEY_VAR: &str = "STYLUS_PORT_LLM_API_KEY";
const LLM_MODEL_ENV_VAR: &str = "STYLUS_PORT_LLM_MODEL";

const OLLAMA_DUMMY_API_KEY: &str = "ollama";
const GENERATE_ISSUE_TEMPERATURE: f32 = 0.5;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("{0} is not set")]
    EnvVarNotSet(&'static str),
    #[error("Invalid response with {0} messages")]
    InvalidResponse(usize),
    #[error("LLM request error: {0}")]
    Request(#[from] ureq::Error),
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
enum Role {
    Assistant,
    System,
    User,
}

#[derive(Serialize, Deserialize)]
struct Message<'a> {
    role: Role,
    content: Cow<'a, str>,
}

#[derive(Serialize)]
struct Request<'a> {
    model: &'a str,
    temperature: f32,
    n: u32,
    messages: Vec<Message<'a>>,
}

#[derive(Deserialize)]
struct Choice<'a> {
    message: Message<'a>,
}

#[derive(Deserialize)]
struct Response<'a> {
    choices: Vec<Choice<'a>>,
}

pub fn model() -> Result<String, Error> {
    env::var(LLM_MODEL_ENV_VAR).map_err(|_| Error::EnvVarNotSet(LLM_MODEL_ENV_VAR))
}

pub fn execute(system: &str, user: &str) -> Result<String, Error> {
    let url = env::var(LLM_URL_ENV_VAR).map_err(|_| Error::EnvVarNotSet(LLM_URL_ENV_VAR))?;

    let api_key = env::var(LLM_URL_API_KEY_VAR).unwrap_or_else(|_| OLLAMA_DUMMY_API_KEY.to_owned());

    let model = &env::var(LLM_MODEL_ENV_VAR).map_err(|_| Error::EnvVarNotSet(LLM_MODEL_ENV_VAR))?;

    let req = Request {
        model,
        temperature: GENERATE_ISSUE_TEMPERATURE,
        messages: vec![
            Message {
                role: Role::System,
                content: Cow::Borrowed(system),
            },
            Message {
                role: Role::User,
                content: Cow::Borrowed(user),
            },
        ],
        n: 1,
    };

    let res: Response = ureq::post(url)
        .header("Authorization", format!("Bearer {api_key}"))
        .send_json(&req)?
        .body_mut()
        .read_json()?;

    if res.choices.len() != 1 {
        return Err(Error::InvalidResponse(res.choices.len()));
    }

    let message_str = res
        .choices
        .into_iter()
        .next()
        .unwrap()
        .message
        .content
        .into_owned();

    Ok(message_str)
}
