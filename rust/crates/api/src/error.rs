use std::env::VarError;
use std::fmt::{Display, Formatter};
use std::time::Duration;

#[derive(Debug)]
pub enum ApiError {
    MissingCredentials {
        provider: &'static str,
        env_vars: &'static [&'static str],
    },
    ExpiredOAuthToken,
    Auth(String),
    InvalidApiKeyEnv(VarError),
    Http(reqwest::Error),
    Io(std::io::Error),
    Json(serde_json::Error),
    Api {
        status: reqwest::StatusCode,
        error_type: Option<String>,
        message: Option<String>,
        body: String,
        retryable: bool,
    },
    RetriesExhausted {
        attempts: u32,
        last_error: Box<ApiError>,
    },
    InvalidSseFrame(&'static str),
    BackoffOverflow {
        attempt: u32,
        base_delay: Duration,
    },
}

impl ApiError {
    #[must_use]
    pub const fn missing_credentials(
        provider: &'static str,
        env_vars: &'static [&'static str],
    ) -> Self {
        Self::MissingCredentials { provider, env_vars }
    }

    #[must_use]
    pub fn is_retryable(&self) -> bool {
        match self {
            Self::Http(error) => error.is_connect() || error.is_timeout() || error.is_request(),
            Self::Api { retryable, .. } => *retryable,
            Self::RetriesExhausted { last_error, .. } => last_error.is_retryable(),
            Self::MissingCredentials { .. }
            | Self::ExpiredOAuthToken
            | Self::Auth(_)
            | Self::InvalidApiKeyEnv(_)
            | Self::Io(_)
            | Self::Json(_)
            | Self::InvalidSseFrame(_)
            | Self::BackoffOverflow { .. } => false,
        }
    }
}

impl Display for ApiError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingCredentials { provider, env_vars } => write!(
                f,
                "missing {provider} credentials; export {} before calling the {provider} API",
                env_vars.join(" or ")
            ),
            Self::ExpiredOAuthToken => {
                write!(
                    f,
                    "saved OAuth token is expired and no refresh token is available"
                )
            }
            Self::Auth(message) => write!(f, "auth error: {message}"),
            Self::InvalidApiKeyEnv(error) => {
                write!(f, "failed to read credential environment variable: {error}")
            }
            Self::Http(error) => write!(f, "http error: {error}"),
            Self::Io(error) => write!(f, "io error: {error}"),
            Self::Json(error) => write!(f, "json error: {error}"),
            Self::Api {
                status,
                error_type,
                message,
                body,
                ..
            } => match (error_type, message) {
                (Some(error_type), Some(message)) => {
                    write!(f, "api returned {status} ({error_type}): {message}")
                }
                _ => write!(f, "api returned {status}: {body}"),
            },
            Self::RetriesExhausted {
                attempts,
                last_error,
            } => write!(f, "api failed after {attempts} attempts: {last_error}"),
            Self::InvalidSseFrame(message) => write!(f, "invalid sse frame: {message}"),
            Self::BackoffOverflow {
                attempt,
                base_delay,
            } => write!(
                f,
                "retry backoff overflowed on attempt {attempt} with base delay {base_delay:?}"
            ),
        }
    }
}

impl std::error::Error for ApiError {}

impl From<reqwest::Error> for ApiError {
    fn from(value: reqwest::Error) -> Self {
        Self::Http(value)
    }
}

impl From<std::io::Error> for ApiError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<serde_json::Error> for ApiError {
    fn from(value: serde_json::Error) -> Self {
        Self::Json(value)
    }
}

impl From<VarError> for ApiError {
    fn from(value: VarError) -> Self {
        Self::InvalidApiKeyEnv(value)
    }
}

#[cfg(test)]
mod tests {
    use std::env::VarError;
    use std::io;
    use std::time::Duration;

    use super::ApiError;

    fn typed_api_error(retryable: bool) -> ApiError {
        ApiError::Api {
            status: reqwest::StatusCode::TOO_MANY_REQUESTS,
            error_type: Some(String::from("rate_limit_error")),
            message: Some(String::from("slow down")),
            body: String::from("raw"),
            retryable,
        }
    }

    #[test]
    fn classifies_retryable_failures_including_exhausted_retries() {
        assert!(typed_api_error(true).is_retryable());
        assert!(!typed_api_error(false).is_retryable());

        let exhausted = ApiError::RetriesExhausted {
            attempts: 3,
            last_error: Box::new(typed_api_error(true)),
        };
        assert!(exhausted.is_retryable());
        assert_eq!(
            exhausted.to_string(),
            "api failed after 3 attempts: api returned 429 Too Many Requests (rate_limit_error): slow down"
        );

        let terminal = ApiError::RetriesExhausted {
            attempts: 2,
            last_error: Box::new(typed_api_error(false)),
        };
        assert!(!terminal.is_retryable());
    }

    #[test]
    fn formats_credential_auth_and_transport_errors() {
        let missing =
            ApiError::missing_credentials("anthropic", &["ANTHROPIC_API_KEY", "CLAUDE_API_KEY"]);
        assert!(!missing.is_retryable());
        assert_eq!(
            missing.to_string(),
            "missing anthropic credentials; export ANTHROPIC_API_KEY or CLAUDE_API_KEY before calling the anthropic API"
        );
        let _error: &dyn std::error::Error = &missing;

        assert!(!ApiError::ExpiredOAuthToken.is_retryable());
        assert!(ApiError::ExpiredOAuthToken.to_string().contains("expired"));

        let auth = ApiError::Auth(String::from("bad token"));
        assert_eq!(auth.to_string(), "auth error: bad token");
        assert!(!auth.is_retryable());

        let env_error = ApiError::from(VarError::NotPresent);
        assert!(matches!(
            env_error,
            ApiError::InvalidApiKeyEnv(VarError::NotPresent)
        ));
        assert!(env_error.to_string().contains("environment variable"));

        let io_error = ApiError::from(io::Error::new(io::ErrorKind::NotFound, "missing"));
        assert!(matches!(io_error, ApiError::Io(_)));
        assert!(io_error.to_string().contains("io error: missing"));

        let json_error =
            ApiError::from(serde_json::from_str::<serde_json::Value>("{").unwrap_err());
        assert!(matches!(json_error, ApiError::Json(_)));
        assert!(json_error.to_string().contains("json error"));

        let frame = ApiError::InvalidSseFrame("missing data");
        assert_eq!(frame.to_string(), "invalid sse frame: missing data");
        assert!(!frame.is_retryable());

        let overflow = ApiError::BackoffOverflow {
            attempt: 8,
            base_delay: Duration::from_secs(30),
        };
        assert!(!overflow.is_retryable());
        assert!(overflow
            .to_string()
            .contains("attempt 8 with base delay 30s"));
    }

    #[test]
    fn formats_api_errors_without_a_typed_message_from_the_body() {
        let error = ApiError::Api {
            status: reqwest::StatusCode::BAD_REQUEST,
            error_type: None,
            message: Some(String::from("ignored without a type")),
            body: String::from("nope"),
            retryable: false,
        };

        assert_eq!(error.to_string(), "api returned 400 Bad Request: nope");
        assert!(!error.is_retryable());
    }
}
