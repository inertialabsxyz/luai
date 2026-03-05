use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct LlmClient {
    api_key: String,
    model: String,
    client: reqwest::blocking::Client,
}

#[derive(Debug, Serialize)]
struct ApiRequest {
    model: String,
    max_tokens: u32,
    system: String,
    messages: Vec<Message>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Deserialize)]
struct ApiResponse {
    content: Vec<ContentBlock>,
}

#[derive(Debug, Deserialize)]
struct ContentBlock {
    text: String,
}

#[derive(Debug)]
pub enum LlmError {
    Http(reqwest::Error),
    Api(String),
    NoContent,
}

impl std::fmt::Display for LlmError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LlmError::Http(e) => write!(f, "HTTP error: {e}"),
            LlmError::Api(msg) => write!(f, "API error: {msg}"),
            LlmError::NoContent => write!(f, "empty response from API"),
        }
    }
}

impl From<reqwest::Error> for LlmError {
    fn from(e: reqwest::Error) -> Self {
        LlmError::Http(e)
    }
}

impl LlmClient {
    pub fn new(api_key: String, model: String) -> Self {
        let client = reqwest::blocking::Client::new();
        LlmClient {
            api_key,
            model,
            client,
        }
    }

    /// Generate a Lua program from a system prompt and conversation history.
    /// Returns the raw text response from the LLM.
    pub fn generate(
        &self,
        system_prompt: &str,
        messages: &[Message],
    ) -> Result<String, LlmError> {
        let request = ApiRequest {
            model: self.model.clone(),
            max_tokens: 4096,
            system: system_prompt.to_string(),
            messages: messages.to_vec(),
        };

        let resp = self
            .client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&request)
            .send()?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().unwrap_or_default();
            return Err(LlmError::Api(format!("status {status}: {body}")));
        }

        let api_resp: ApiResponse = resp.json()?;
        let text = api_resp
            .content
            .into_iter()
            .map(|b| b.text)
            .collect::<Vec<_>>()
            .join("");

        if text.is_empty() {
            return Err(LlmError::NoContent);
        }

        Ok(text)
    }
}

/// Strip markdown code fences from LLM output.
/// Handles ```lua ... ```, ``` ... ```, and bare code.
pub fn strip_code_fences(raw: &str) -> String {
    let trimmed = raw.trim();

    // Try ```lua or ```
    if let Some(rest) = trimmed.strip_prefix("```lua") {
        if let Some(code) = rest.strip_suffix("```") {
            return code.trim().to_string();
        }
    }
    if let Some(rest) = trimmed.strip_prefix("```") {
        if let Some(code) = rest.strip_suffix("```") {
            return code.trim().to_string();
        }
    }

    trimmed.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_bare_code() {
        let input = "return 42";
        assert_eq!(strip_code_fences(input), "return 42");
    }

    #[test]
    fn strip_lua_fence() {
        let input = "```lua\nreturn 42\n```";
        assert_eq!(strip_code_fences(input), "return 42");
    }

    #[test]
    fn strip_plain_fence() {
        let input = "```\nreturn 42\n```";
        assert_eq!(strip_code_fences(input), "return 42");
    }

    #[test]
    fn strip_with_whitespace() {
        let input = "  ```lua\n  local x = 1\n  return x\n  ```  ";
        assert_eq!(strip_code_fences(input), "local x = 1\n  return x");
    }
}
