struct Error(anyhow::Error);

impl IntoResponse for Error {
  fn into_response(self) -> Response {
    let status = StatusCode::INTERNAL_SERVER_ERROR;
    let body = format!("Internal Server Error: {}", self.0);
    Response::builder()
      .status(status)
      .header(header::CONTENT_TYPE, "text/plain")
      .body(Body::from(body))
      .unwrap()
  }
}
