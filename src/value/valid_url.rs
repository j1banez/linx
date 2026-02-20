use thiserror::Error;
use url::Url;

const MAX_URL_LEN: usize = 2048;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidUrl(String);

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ValidUrlError {
    #[error("url cannot be empty")]
    Empty,
    #[error("url too long (max {max})")]
    TooLong { max: usize },
    #[error("url contains control characters")]
    WithControl,
    #[error("invalid url")]
    Invalid,
    #[error("only http/https urls are allowed")]
    NotHttp,
    #[error("urls with credentials are not allowed")]
    WithCredentials,
}

impl TryFrom<&str> for ValidUrl {
    type Error = ValidUrlError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let url = value.trim();

        if url.is_empty() {
            return Err(ValidUrlError::Empty);
        }

        if url.len() > MAX_URL_LEN {
            return Err(ValidUrlError::TooLong { max: MAX_URL_LEN });
        }

        if url.chars().any(|c| c.is_control()) {
            return Err(ValidUrlError::WithControl);
        }

        let url = Url::parse(url).map_err(|_| ValidUrlError::Invalid)?;

        match url.scheme() {
            "http" | "https" => {}
            _ => return Err(ValidUrlError::NotHttp),
        }

        if !url.username().is_empty() || url.password().is_some() {
            return Err(ValidUrlError::WithCredentials);
        }

        Ok(Self(url.to_string()))
    }
}

impl TryFrom<String> for ValidUrl {
    type Error = ValidUrlError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        ValidUrl::try_from(value.as_str())
    }
}

impl std::fmt::Display for ValidUrl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl ValidUrl {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_url_ok_http_https() {
        assert!(ValidUrl::try_from("https://example.com").is_ok());
        assert!(ValidUrl::try_from("http://example.com/path?q=1#frag").is_ok());
    }

    #[test]
    fn validate_url_trims_whitespace() {
        let url = ValidUrl::try_from("  https://example.com  ").unwrap();
        assert_eq!(url.to_string(), "https://example.com/");
    }

    #[test]
    fn validate_url_rejects_empty() {
        assert!(ValidUrl::try_from("").is_err());
        assert!(ValidUrl::try_from("   ").is_err());
    }

    #[test]
    fn validate_url_rejects_invalid_url() {
        assert!(ValidUrl::try_from("not a url").is_err());
        assert!(ValidUrl::try_from("http://").is_err());
        assert!(ValidUrl::try_from("https://").is_err());
    }

    #[test]
    fn validate_url_rejects_non_http_schemes() {
        assert!(ValidUrl::try_from("ftp://example.com").is_err());
        assert!(ValidUrl::try_from("file:///etc/passwd").is_err());
        assert!(ValidUrl::try_from("javascript:alert(1)").is_err());
        assert!(ValidUrl::try_from("data:text/plain,hello").is_err());
    }

    #[test]
    fn validate_url_rejects_control_characters() {
        assert!(ValidUrl::try_from("https://example.com/\nfoo").is_err());
        assert!(ValidUrl::try_from("https://example.com/\rfoo").is_err());
        assert!(ValidUrl::try_from("https://example.com/\tfoo").is_err());
        assert!(ValidUrl::try_from("https://example.com/\r\nX: y").is_err());
    }

    #[test]
    fn validate_url_rejects_credentials() {
        assert!(ValidUrl::try_from("https://user@example.com").is_err());
        assert!(ValidUrl::try_from("https://user:pass@example.com").is_err());
    }

    #[test]
    fn validate_url_rejects_too_long() {
        let base = "https://example.com/";
        let long_path = "a".repeat(2048 - base.len() + 1);
        let raw = format!("{base}{long_path}");
        assert!(ValidUrl::try_from(raw).is_err());
    }

    #[test]
    fn validate_url_normalizes_some_inputs() {
        let url = ValidUrl::try_from("HTTP://EXAMPLE.COM").unwrap();
        assert_eq!(url.to_string(), "http://example.com/");
    }
}
