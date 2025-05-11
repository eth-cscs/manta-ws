use axum::{
  http::StatusCode,
  response::{IntoResponse, Response},
};
use std::collections::HashMap;

use crate::log::*;

pub struct ErrorResponse {
  pub code: axum::http::StatusCode,
  pub reason: String,
}

impl ErrorResponse {
  pub fn new(code: StatusCode, reason: String) -> ErrorResponse {
    ErrorResponse {
      code: code,
      reason: reason,
    }
  }

  pub fn respond(self) -> Response {
    let mut message = HashMap::new();
    message.insert("error", self.reason.to_owned());

    let json_message;
    match serde_json::to_string(&message) {
      Ok(good_message) => json_message = good_message,
      Err(_) => json_message = "{ error: \"Unknown error\" }".to_string(),
    }

    log(format!("ERROR {}", self.reason));

    (self.code, json_message).into_response()
  }
}

pub fn error_respond(code: StatusCode, reason: String) -> Response {
  ErrorResponse::new(code, reason).respond()
}
