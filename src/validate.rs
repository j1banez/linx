use crate::error::AppError;
use url::Url;

const MAX_URL_LEN: usize = 2048;
pub const DEFAULT_CODE_LEN: usize = 6;
pub const MIN_CODE_LEN: usize = 4;
pub const MAX_CODE_LEN: usize = 32;
pub const BASE62: &[u8] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";

pub fn is_base62(s: &str) -> bool {
    !s.is_empty() && s.bytes().all(|b| BASE62.contains(&b))
}

pub fn validate_and_normalize_code(raw: &str) -> Result<String, AppError> {
    let code = raw.trim();

    if code.is_empty() {
        return Err(AppError::BadRequest("code cannot be empty".into()));
    }

    if code.len() > MAX_CODE_LEN {
        return Err(AppError::BadRequest(format!(
            "code too long (max {MAX_CODE_LEN})"
        )));
    }

    if !is_base62(&code) {
        return Err(AppError::BadRequest(
            "code must contain only base62 characters".into(),
        ));
    }

    Ok(code.to_string())
}

pub fn validate_and_normalize_url(raw: &str) -> Result<String, AppError> {
    let s = raw.trim();

    if s.is_empty() {
        return Err(AppError::BadRequest("url cannot be empty".into()));
    }

    if s.len() > MAX_URL_LEN {
        return Err(AppError::BadRequest(format!(
            "url too long (max {MAX_URL_LEN})"
        )));
    }

    // Prevents CR/LF and other control characters (avoids header injection, weird logs, etc.).
    if s.chars().any(|c| c.is_control()) {
        return Err(AppError::BadRequest(
            "url contains control characters".into(),
        ));
    }

    let url = Url::parse(s).map_err(|_| AppError::BadRequest("invalid url".into()))?;

    match url.scheme() {
        "http" | "https" => {}
        _ => {
            return Err(AppError::BadRequest(
                "only http/https urls are allowed".into(),
            ));
        }
    }

    if !url.username().is_empty() || url.password().is_some() {
        return Err(AppError::BadRequest(
            "urls with credentials are not allowed".into(),
        ));
    }

    Ok(url.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    // is_base62

    #[test]
    fn is_base62_accepts_alphanumeric() {
        assert!(is_base62("abcXYZ012"));
        assert!(is_base62("0"));
        assert!(is_base62("Z"));
        assert!(is_base62("z"));
    }

    #[test]
    fn is_base62_rejects_non_base62_chars() {
        assert!(!is_base62(""));
        assert!(!is_base62("hello-world"));
        assert!(!is_base62("hello_world"));
        assert!(!is_base62("hello world"));
        assert!(!is_base62("é"));
        assert!(!is_base62("/"));
        assert!(!is_base62("%2F"));
        assert!(!is_base62("?"));
        assert!(!is_base62("!"));
    }

    // validate_and_normalize_code

    #[test]
    fn validate_code_ok() {
        assert!(validate_and_normalize_code("AbC123").is_ok());
    }

    #[test]
    fn validate_code_rejects_empty() {
        assert!(validate_and_normalize_code("").is_err());
    }

    #[test]
    fn validate_code_rejects_too_long() {
        assert!(validate_and_normalize_code("qqqwedfrtg45yhju8OK7YHgtfred5680x").is_err());
    }

    #[test]
    fn validate_code_rejects_symbols() {
        assert!(validate_and_normalize_code("abc-123").is_err());
    }

    // validate_and_normalize_url

    #[test]
    fn validate_url_ok_http_https() {
        assert!(validate_and_normalize_url("https://example.com").is_ok());
        assert!(validate_and_normalize_url("http://example.com/path?q=1#frag").is_ok());
    }

    #[test]
    fn validate_url_trims_whitespace() {
        let url = validate_and_normalize_url("  https://example.com  ").unwrap();
        assert_eq!(url, "https://example.com/");
    }

    #[test]
    fn validate_url_rejects_empty() {
        assert!(validate_and_normalize_url("").is_err());
        assert!(validate_and_normalize_url("   ").is_err());
    }

    #[test]
    fn validate_url_rejects_invalid_url() {
        assert!(validate_and_normalize_url("not a url").is_err());
        assert!(validate_and_normalize_url("http://").is_err());
        assert!(validate_and_normalize_url("https://").is_err());
    }

    #[test]
    fn validate_url_rejects_non_http_schemes() {
        assert!(validate_and_normalize_url("ftp://example.com").is_err());
        assert!(validate_and_normalize_url("file:///etc/passwd").is_err());
        assert!(validate_and_normalize_url("javascript:alert(1)").is_err());
        assert!(validate_and_normalize_url("data:text/plain,hello").is_err());
    }

    #[test]
    fn validate_url_rejects_control_characters() {
        assert!(validate_and_normalize_url("https://example.com/\nfoo").is_err());
        assert!(validate_and_normalize_url("https://example.com/\rfoo").is_err());
        assert!(validate_and_normalize_url("https://example.com/\tfoo").is_err());
        // CRLF injection style
        assert!(validate_and_normalize_url("https://example.com/\r\nX: y").is_err());
    }

    #[test]
    fn validate_url_rejects_credentials() {
        assert!(validate_and_normalize_url("https://user@example.com").is_err());
        assert!(validate_and_normalize_url("https://user:pass@example.com").is_err());
    }

    #[test]
    fn validate_url_rejects_too_long() {
        // Build a URL longer than MAX_URL_LEN
        let base = "https://example.com/";
        let long_path = "a".repeat(2048 - base.len() + 1);
        let raw = format!("{base}{long_path}");
        assert!(validate_and_normalize_url(&raw).is_err());
    }

    #[test]
    fn validate_url_normalizes_some_inputs() {
        // url::Url normalizes host to lowercase and ensures trailing slash for bare host
        let url = validate_and_normalize_url("HTTP://EXAMPLE.COM").unwrap();
        assert_eq!(url, "http://example.com/");
    }
}
