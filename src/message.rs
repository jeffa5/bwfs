#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub enum Request {
    Unlock { password: String },
    Status,
    Refresh,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub enum Response {
    Status { locked: bool },
    Success,
    Failure,
}
