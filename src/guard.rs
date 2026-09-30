use axum::http::HeaderMap;
use worker::wasm_bindgen::JsValue;
use worker::*;

use crate::error::ApiError;

/// Shared protection for public AI endpoints: per-IP rate limiting plus an
/// optional Turnstile verification. Both are inert until configured
/// (the `RATE_LIMITER` binding and the `TURNSTILE_SECRET` secret).
pub async fn check(
    env: &Env,
    headers: &HeaderMap,
    route: &str,
    token: &str,
) -> Result<(), ApiError> {
    let ip = client_ip(headers);

    if let Ok(limiter) = env.rate_limiter("RATE_LIMITER") {
        let outcome = limiter
            .limit(format!("{route}:{ip}"))
            .await
            .map_err(|e| ApiError::bad_gateway(format!("rate limiter error: {e}")))?;
        if !outcome.success {
            return Err(ApiError::too_many("Too many requests — try again in a minute."));
        }
    }

    verify_turnstile(env, token).await
}

fn client_ip(headers: &HeaderMap) -> String {
    headers
        .get("cf-connecting-ip")
        .or_else(|| headers.get("x-forwarded-for"))
        .and_then(|v| v.to_str().ok())
        .unwrap_or("unknown")
        .split(',')
        .next()
        .unwrap_or("unknown")
        .trim()
        .to_string()
}

async fn verify_turnstile(env: &Env, token: &str) -> Result<(), ApiError> {
    let secret = match env.secret("TURNSTILE_SECRET") {
        Ok(secret) => secret.to_string(),
        Err(_) => return Ok(()),
    };

    if token.is_empty() {
        return Err(ApiError::bad_request("missing Turnstile token"));
    }

    let body = format!("secret={}&response={}", urlencode(&secret), urlencode(token));
    let headers = Headers::new();
    headers
        .set("Content-Type", "application/x-www-form-urlencoded")
        .map_err(|_| ApiError::bad_gateway("verification setup failed"))?;
    let mut init = RequestInit::new();
    init.with_method(Method::Post)
        .with_headers(headers)
        .with_body(Some(JsValue::from_str(&body)));

    let request =
        Request::new_with_init("https://challenges.cloudflare.com/turnstile/v0/siteverify", &init)
            .map_err(|_| ApiError::bad_gateway("verification setup failed"))?;
    let mut response = Fetch::Request(request)
        .send()
        .await
        .map_err(|_| ApiError::bad_gateway("verification failed"))?;
    let text = response
        .text()
        .await
        .map_err(|_| ApiError::bad_gateway("verification failed"))?;

    #[derive(serde::Deserialize)]
    struct Verify {
        success: bool,
    }
    let verify: Verify =
        serde_json::from_str(&text).map_err(|_| ApiError::bad_gateway("verification failed"))?;

    if verify.success {
        Ok(())
    } else {
        Err(ApiError::bad_request("Turnstile verification failed"))
    }
}

fn urlencode(value: &str) -> String {
    value
        .bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                (b as char).to_string()
            }
            _ => format!("%{b:02X}"),
        })
        .collect()
}
