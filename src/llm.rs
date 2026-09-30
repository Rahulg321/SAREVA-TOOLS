use serde::de::DeserializeOwned;
use worker::wasm_bindgen::JsValue;
use worker::*;

const WORKERS_AI_MODEL: &str = "@cf/meta/llama-3.3-70b-instruct-fp8-fast";

/// Run a chat completion. Backend defaults to Workers AI; set the `LLM_BACKEND`
/// var to `"deepseek"` to route through DeepSeek's OpenAI-compatible endpoint.
pub async fn complete(
    env: &Env,
    system: &str,
    user: &str,
    max_tokens: u32,
) -> Result<String, String> {
    if backend(env) == "deepseek" {
        deepseek(env, system, user, max_tokens).await
    } else {
        workers_ai(env, system, user, max_tokens).await
    }
}

/// Run a completion and parse the result as JSON (model is asked to return raw JSON).
pub async fn complete_json<T: DeserializeOwned>(
    env: &Env,
    system: &str,
    user: &str,
    max_tokens: u32,
) -> Result<T, String> {
    let raw = complete(env, system, user, max_tokens).await?;
    serde_json::from_str::<T>(strip_fences(&raw))
        .map_err(|e| format!("model returned invalid JSON: {e}"))
}

fn backend(env: &Env) -> String {
    env.var("LLM_BACKEND")
        .ok()
        .map(|v| v.to_string())
        .unwrap_or_default()
}

async fn workers_ai(
    env: &Env,
    system: &str,
    user: &str,
    max_tokens: u32,
) -> Result<String, String> {
    #[derive(serde::Serialize)]
    struct ChatInput<'a> {
        messages: Vec<ChatMessage<'a>>,
        max_tokens: u32,
    }

    #[derive(serde::Serialize)]
    struct ChatMessage<'a> {
        role: &'a str,
        content: &'a str,
    }

    let ai = env.ai("AI").map_err(|e| e.to_string())?;
    let input = ChatInput {
        messages: vec![
            ChatMessage {
                role: "system",
                content: system,
            },
            ChatMessage {
                role: "user",
                content: user,
            },
        ],
        max_tokens,
    };

    let value: serde_json::Value = ai
        .run(WORKERS_AI_MODEL, input)
        .await
        .map_err(|e| e.to_string())?;

    // Workers AI returns `{ response: "<text>" }` for most text models, but some
    // return the parsed object (or a nested object) directly — handle all shapes.
    let text = match &value {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Object(map) => match map.get("response") {
            Some(serde_json::Value::String(s)) => s.clone(),
            Some(other) => other.to_string(),
            None => value.to_string(),
        },
        other => other.to_string(),
    };
    Ok(text)
}

async fn deepseek(
    env: &Env,
    system: &str,
    user: &str,
    max_tokens: u32,
) -> Result<String, String> {
    let key = env
        .secret("DEEPSEEK_API_KEY")
        .map_err(|_| "DEEPSEEK_API_KEY is not configured".to_string())?
        .to_string();

    let payload = serde_json::json!({
        "model": "deepseek-chat",
        "messages": [
            { "role": "system", "content": system },
            { "role": "user", "content": user }
        ],
        "max_tokens": max_tokens,
        "stream": false
    });
    let body = payload.to_string();

    let headers = Headers::new();
    headers
        .set("Content-Type", "application/json")
        .map_err(|e| e.to_string())?;
    headers
        .set("Authorization", &format!("Bearer {key}"))
        .map_err(|e| e.to_string())?;

    let mut init = RequestInit::new();
    init.with_method(Method::Post)
        .with_headers(headers)
        .with_body(Some(JsValue::from_str(&body)));

    let request = Request::new_with_init("https://api.deepseek.com/chat/completions", &init)
        .map_err(|e| e.to_string())?;
    let mut response = Fetch::Request(request).send().await.map_err(|e| e.to_string())?;
    let text = response.text().await.map_err(|e| e.to_string())?;

    #[derive(serde::Deserialize)]
    struct Resp {
        choices: Vec<Choice>,
    }
    #[derive(serde::Deserialize)]
    struct Choice {
        message: Message,
    }
    #[derive(serde::Deserialize)]
    struct Message {
        content: String,
    }

    let parsed: Resp =
        serde_json::from_str(&text).map_err(|e| format!("deepseek bad response: {e}"))?;
    parsed
        .choices
        .into_iter()
        .next()
        .map(|c| c.message.content)
        .ok_or_else(|| "deepseek returned no choices".to_string())
}

fn strip_fences(raw: &str) -> &str {
    let trimmed = raw.trim();
    let without_open = trimmed
        .strip_prefix("```json")
        .or_else(|| trimmed.strip_prefix("```"))
        .unwrap_or(trimmed);
    without_open
        .trim_end()
        .strip_suffix("```")
        .unwrap_or(without_open)
        .trim()
}
