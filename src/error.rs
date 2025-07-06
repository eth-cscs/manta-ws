use axum::{
  body::Body,
  response::{IntoResponse, Response},
};
use hyper::{StatusCode, header};

// struct Error(manta_backend_dispatcher::error::Error);

/* impl IntoResponse for manta_backend_dispatcher::error::Error {
  fn into_response(self) -> Response {
    // self.to_string()
    let status = StatusCode::INTERNAL_SERVER_ERROR;
    let body = self.to_string();
    Response::builder()
      .status(status)
      .header(header::CONTENT_TYPE, "text/plain")
      .body(Body::from(body))
      .unwrap()
  }
} */

/* impl IntoResponse for axum::Error {
  fn into_response(self) -> Response {
    // self.to_string()
    let status = StatusCode::INTERNAL_SERVER_ERROR;
    let body = self.to_string();
    Response::builder()
      .status(status)
      .header(header::CONTENT_TYPE, "text/plain")
      .body(Body::from(body))
      .unwrap()
  }
} */
