use std::result::Result;
use axum::http::{Response, StatusCode};
use serde_json::{json};

/// Utility function to clean up returning a response from a command handler
pub trait Responder {
    /// Turns self into `Result<Response>`
    fn to_response(self) -> Result<Response<()>, axum::Error>;
}

/// If someone wants to return `worker::Response`
impl Responder for Response<()> {
    fn to_response(self) -> Result<Response<()>, axum::Error> {
        Ok(self)
    }
}

/// Returning a String will do a *regular* response
impl Responder for String {
    fn to_response(self) -> Result<Response<serde_json::Value>, axum::Error> {
        let body = json!({
            "type": 4,
            "data": {
                "content": self
            }
        });

        Response::builder()
            .header("Content-Type", "application/json")
            .status(StatusCode::OK)
            .body(&body)
    }
}

impl Responder for &str {
    fn to_response(self) -> Result<Response<()>, axum::Error> {
        let body = json!({
            "type": 4,
            "data": {
                "content": self
            }
        });

        Response::builder()
            .header("Content-Type", "application/json")
            .status(StatusCode::OK)
            .body(&body)
    }
}
