mod jwt_utils;

use axum::{
    http::StatusCode,
    response::IntoResponse,
    routing::{get, head, post},
    Json, Router,
};
use hyper::HeaderMap;
// use jsonwebtoken::{decode, DecodingKey, Validation};
use base64::{decode, encode};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::net::SocketAddr;

use crate::jwt_utils::get_claims_from_jwt_token;

// #[derive(Debug, Serialize, Deserialize)]
// struct Claims {
//    sub: String,
//    name: String
// }

#[tokio::main]
async fn main() {
    // initialize tracing
    tracing_subscriber::fmt::init();

    // build our application with a route
    let app = Router::new()
        .route("/test/whoami", get(test_whoami))
        .route("/test/ping", get(test_ping))
        // `GET /` goes to `root`
        .route("/", get(root))
        // `POST /users` goes to `create_user`
        .route("/users", post(create_user));

    // run our app with hyper
    // `axum::Server` is a re-export of `hyper::Server`
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    tracing::debug!("listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();

    println
}

async fn root() -> &'static str {
    "Hello, World!"
}

async fn test_whoami(headers: HeaderMap) {
    let token = headers.get("authentication").unwrap().to_str().unwrap();

    let claims_json = get_claims_from_jwt_token(token).unwrap();

    println!("Hello {}!!!", claims_json["name"].as_str().unwrap());
}

async fn test_ping() -> &'static str {
    "Pong!"
}

async fn create_user(
    // this argument tells axum to parse the request body
    // as JSON into a `CreateUser` type
    Json(payload): Json<CreateUser>,
) -> impl IntoResponse {
    // insert your application logic here
    let user = User {
        id: 1337,
        username: payload.username,
    };

    // this will be converted into a JSON response
    // with a status code of `201 Created`
    (StatusCode::CREATED, Json(user))
}

// the input to our `create_user` handler
#[derive(Deserialize)]
struct CreateUser {
    username: String,
}

// the output to our `create_user` handler
#[derive(Serialize)]
struct User {
    id: u64,
    username: String,
}
