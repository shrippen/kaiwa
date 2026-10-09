//! HTTP behind a trait, so providers are tested with recorded payloads.

use std::sync::Mutex;

use serde_json::Value;

use crate::AiError;

pub trait Transport: Send + Sync {
    /// POST `body` as JSON to `url` with `headers`; the parsed JSON answer.
    fn post_json(
        &self,
        url: &str,
        headers: &[(&str, String)],
        body: &Value,
    ) -> Result<Value, AiError>;
}

/// The real network, blocking (the engine calls providers from a worker thread).
pub struct UreqTransport {
    agent: ureq::Agent,
}

impl Default for UreqTransport {
    fn default() -> Self {
        let config = ureq::Agent::config_builder()
            .timeout_global(Some(std::time::Duration::from_secs(60)))
            .http_status_as_error(false)
            .build();
        Self {
            agent: config.into(),
        }
    }
}

impl Transport for UreqTransport {
    fn post_json(
        &self,
        url: &str,
        headers: &[(&str, String)],
        body: &Value,
    ) -> Result<Value, AiError> {
        let mut request = self.agent.post(url);
        for (name, value) in headers {
            request = request.header(*name, value.as_str());
        }
        let mut response = request
            .send_json(body)
            .map_err(|e| AiError::Network(e.to_string()))?;
        let status = response.status().as_u16();
        let text = response
            .body_mut()
            .read_to_string()
            .map_err(|e| AiError::Network(e.to_string()))?;
        if !(200..300).contains(&status) {
            return Err(AiError::Status { status, body: text });
        }
        serde_json::from_str(&text).map_err(|e| AiError::Format(format!("{e}: {text}")))
    }
}

/// One request a [`FakeTransport`] received: url, headers, body.
pub type SentRequest = (String, Vec<(String, String)>, Value);

/// A recorded exchange for tests: answers every request with `response` and keeps the requests.
pub struct FakeTransport {
    pub response: Value,
    pub requests: Mutex<Vec<SentRequest>>,
}

impl FakeTransport {
    pub fn new(response: Value) -> Self {
        Self {
            response,
            requests: Mutex::new(Vec::new()),
        }
    }
}

impl Transport for FakeTransport {
    fn post_json(
        &self,
        url: &str,
        headers: &[(&str, String)],
        body: &Value,
    ) -> Result<Value, AiError> {
        let headers = headers
            .iter()
            .map(|(n, v)| (n.to_string(), v.clone()))
            .collect();
        self.requests
            .lock()
            .expect("not poisoned")
            .push((url.to_string(), headers, body.clone()));
        Ok(self.response.clone())
    }
}
