use failure::Error;
use std::error::Error as StdError;
use std::fmt;

#[derive(Debug)]
pub struct ApiError {
	inner: Error,
}

impl ApiError {
	pub fn new(inner: Error) -> Self {
		Self { inner }
	}
}

impl StdError for ApiError {}

impl fmt::Display for ApiError {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f, "{}", self.inner)
	}
}
