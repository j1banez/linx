use thiserror::Error;

pub const DEFAULT_CODE_LEN: usize = 6;
pub const MIN_CODE_LEN: usize = 4;
pub const MAX_CODE_LEN: usize = 32;
pub const BASE62: &[u8] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Code(String);

#[derive(Debug, Error, PartialEq, Eq)]
pub enum CodeError {
    #[error("string is empty")]
    Empty,
    #[error("code too long (max {max})")]
    TooLong { max: usize },
    #[error("code must contain only base62 characters")]
    NotBase62,
}

impl TryFrom<&str> for Code {
    type Error = CodeError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let code = value.trim();

        if code.is_empty() {
            return Err(CodeError::Empty);
        }

        if code.len() > MAX_CODE_LEN {
            return Err(CodeError::TooLong { max: MAX_CODE_LEN });
        }

        if !is_base62(code) {
            return Err(CodeError::NotBase62);
        }

        Ok(Self(code.to_owned()))
    }
}

impl TryFrom<String> for Code {
    type Error = CodeError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Code::try_from(value.as_str())
    }
}

impl std::fmt::Display for Code {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl Code {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

fn is_base62(s: &str) -> bool {
    !s.is_empty() && s.bytes().all(|b| BASE62.contains(&b))
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn validate_code_ok() {
        assert!(Code::try_from("AbC123").is_ok());
    }

    #[test]
    fn validate_code_rejects_empty() {
        assert!(Code::try_from("").is_err());
    }

    #[test]
    fn validate_code_rejects_too_long() {
        assert!(Code::try_from("qqqwedfrtg45yhju8OK7YHgtfred5680x").is_err());
    }

    #[test]
    fn validate_code_rejects_symbols() {
        assert!(Code::try_from("abc-123").is_err());
    }
}
