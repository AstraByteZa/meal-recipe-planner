//! Settings read from the environment at startup.

/// Shortest JWT secret accepted. Anything shorter is easy to guess offline.
const MIN_SECRET_BYTES: usize = 32;

pub struct Config {
    pub database_url: String,
    pub jwt_secret: String,
    pub bind_addr: String,
    pub registration_open: bool,
}

impl Config {
    pub fn from_env() -> Result<Self, String> {
        let jwt_secret = required("JWT_SECRET")?;
        if jwt_secret.len() < MIN_SECRET_BYTES {
            return Err(format!("JWT_SECRET must be at least {MIN_SECRET_BYTES} bytes"));
        }
        Ok(Self {
            database_url: required("DATABASE_URL")?,
            jwt_secret,
            bind_addr: std::env::var("BIND_ADDR").unwrap_or_else(|_| "0.0.0.0:8080".to_string()),
            registration_open: parse_flag(std::env::var("REGISTRATION_OPEN").ok().as_deref(), true),
        })
    }
}

fn required(name: &str) -> Result<String, String> {
    std::env::var(name).map_err(|_| format!("{name} must be set"))
}

/// Reads a boolean flag. Unset means `default`. `false`, `0`, `no` and `off` mean false.
fn parse_flag(value: Option<&str>, default: bool) -> bool {
    match value {
        None => default,
        Some(text) => !matches!(text.trim().to_ascii_lowercase().as_str(), "false" | "0" | "no" | "off"),
    }
}

#[cfg(test)]
mod tests {
    use super::parse_flag;

    #[test]
    fn flags_default_when_unset() {
        assert!(parse_flag(None, true));
        assert!(!parse_flag(None, false));
    }

    #[test]
    fn falsy_words_disable_the_flag() {
        for text in ["false", "FALSE", " 0 ", "no", "off"] {
            assert!(!parse_flag(Some(text), true), "{text} should be false");
        }
        assert!(parse_flag(Some("true"), false));
    }
}
