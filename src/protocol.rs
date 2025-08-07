


use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub enum Request {
    Unlock { password: String },
    Set { key: String, value: String },
    Get { key: String },
    Lock,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum Response {
    Ok(Option<String>),
    Error(String),
}
