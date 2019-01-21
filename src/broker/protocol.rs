use std::fmt::{Display, Formatter, Result};
use colored::*;

#[derive(Serialize, Deserialize, Debug)]
pub enum ProtocolError {
    UnknownError,
    InvalidRequest,
    InvalidSignature,
    InvalidChallenge,
    TooManySubscriptions,
}

impl Display for ProtocolError {
    fn fmt(&self, f: &mut Formatter) -> Result {
        match *self {
            ProtocolError::UnknownError => write!(f, "{}", "unknown error!"),
            ProtocolError::InvalidRequest => write!(f, "{}", "invalid request!"),
            ProtocolError::InvalidSignature => write!(f, "{}", "invalid signature!"),
            ProtocolError::InvalidChallenge => write!(f, "{}", "invalid challenge!"),
            ProtocolError::TooManySubscriptions => write!(f, "{}", "too many subscriptions!"),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type")]
pub enum ProtocolRequest {
    Challenge,
    Subscribe { address: String, signature: String },
    PostSlate { from: String, to: String, str: String, signature: String },
    Unsubscribe { address: String },
}

impl Display for ProtocolRequest {
    fn fmt(&self, f: &mut Formatter) -> Result {
        match *self {
            ProtocolRequest::Challenge => write!(f, "{}", "Challenge".bright_purple()),
            ProtocolRequest::Subscribe { ref address, signature: _ } => write!(f, "{} to {}", "Subscribe".bright_purple(), address.bright_green()),
            ProtocolRequest::Unsubscribe { ref address } => write!(f, "{} from {}", "Unsubscribe".bright_purple(), address.bright_green()),
            ProtocolRequest::PostSlate { ref from, ref to, str: _, signature: _ } => write!(f, "{} from {} to {}", "PostSlate".bright_purple(), from.bright_green(), to.bright_green()),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type")]
pub enum ProtocolResponse {
    Ok,
    Error { kind: ProtocolError, description: String },
    Challenge { str: String },
    Slate { from: String, message: String, signature: String, challenge: String },
}

impl Display for ProtocolResponse {
    fn fmt(&self, f: &mut Formatter) -> Result {
        match *self {
            ProtocolResponse::Ok => write!(f, "{}", "Ok".cyan()),
            ProtocolResponse::Error { ref kind, description: _ } => write!(f, "{}: {}", "ERROR".bright_red(), kind),
            ProtocolResponse::Challenge { ref str } => write!(f, "{} {}", "Challenge".cyan(), str.bright_green()),
            ProtocolResponse::Slate { ref from, message: _, signature: _, challenge: _ } => write!(f, "{} from {}", "Slate".cyan(), from.bright_green()),
        }
    }
}