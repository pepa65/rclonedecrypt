use anyhow::Result;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum DecryptionError {
	#[error("Invalid file format")]
	InvalidFormat,
	#[error("Invalid password")]
	InvalidPassword,
	#[error("IO error: {0}")]
	Io(#[from] std::io::Error),
	#[error("Base64 decode error: {0}")]
	Base64(#[from] base64::DecodeError),
	#[error("Hex decode error: {0}")]
	Hex(#[from] hex::FromHexError),
}

pub type DecryptionResult<T> = Result<T, DecryptionError>;
