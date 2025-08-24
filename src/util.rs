use thiserror::Error;
use tracing::warn;

pub trait ResLog<T> {
    #[track_caller]
    fn warn(self) -> Option<T>;
}

impl<T, E: std::fmt::Display> ResLog<T> for Result<T, E> {
    #[track_caller]
    fn warn(self) -> Option<T> {
        match self {
            Ok(x) => Some(x),
            Err(err) => {
                let loc = std::panic::Location::caller();
                warn!(
                    %err,
                    "Unexpected error at {}:{}:{}",
                    loc.file().replace("\\", "/"),
                    loc.line(),
                    loc.column()
                );
                None
            }
        }
    }
}

#[derive(Debug, Error)]
pub enum EnvVarError {
    #[error("Invalid/Missing {key} or {key}_FILE")]
    InvalidOrMissingKey { key: String },
    #[error("Invalid value for {key}: {err}")]
    InvalidValue { key: String, err: String },
}

pub fn env_var<T: std::str::FromStr>(key: &str) -> Result<T, EnvVarError>
where
    T::Err: std::fmt::Display,
{
    match std::env::var(key) {
        Ok(x) => x.parse().map_err(|err: T::Err| EnvVarError::InvalidValue {
            key: key.to_string(),
            err: err.to_string(),
        }),
        Err(_) => std::env::var(format!("{key}_FILE"))
            .map_err(|_| EnvVarError::InvalidOrMissingKey {
                key: key.to_string(),
            })?
            .parse()
            .map_err(|err: T::Err| EnvVarError::InvalidValue {
                key: format!("{key}_FILE"),
                err: err.to_string(),
            }),
    }
}
