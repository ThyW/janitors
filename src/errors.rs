use std::{fmt::Display, path::PathBuf};

pub type JResult<T = ()> = anyhow::Result<T>;

#[derive(Clone, Debug)]
pub enum JError {
    MissingValue(String),
    InvalidPath(PathBuf),
}

impl std::error::Error for JError {}

impl Display for JError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingValue(v) => write!(f, "Missing value: {v}"),
            Self::InvalidPath(v) => write!(f, "Invalid path: {}", v.display()),
        }
    }
}

impl<Ref: AsRef<str>> From<&Ref> for JError {
    fn from(value: &Ref) -> Self {
        let value = value.as_ref();
        Self::MissingValue(value.to_string())
    }
}
