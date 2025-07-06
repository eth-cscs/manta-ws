mod backend_api;
mod commands;
mod common;
mod error;
mod handlers;
mod http_response;
mod jwt_utils;
mod log;
mod manta_backend_dispatcher;

use ::manta_backend_dispatcher::{
  interfaces::{
    bss::BootParametersTrait, cfs::CfsTrait, hsm::group::GroupTrait,
    pcs::PCSTrait,
  },
  types::{BootParameters, K8sAuth, K8sDetails},
};
use axum::{
  Json, Router, debug_handler,
  extract::{
    ConnectInfo, Path, Query, WebSocketUpgrade,
    ws::{Message, Utf8Bytes, WebSocket},
  },
  http::{HeaderMap, StatusCode},
  response::{IntoResponse, Response},
  routing::{delete, get, post, put},
};
use axum_extra::{TypedHeader, headers};
use bytes::Bytes;
use common::config::types::MantaConfiguration;
use config::Config;
use csm_rs::{
  common::vault::http_client::fetch_shasta_k8s_secrets_from_vault,
  hsm::hw_inventory::hw_component::types::NodeSummary,
};
use directories::ProjectDirs;
use futures::{AsyncBufReadExt, SinkExt, StreamExt, TryStreamExt};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
  fs::File, io::Read, net::SocketAddr, ops::ControlFlow, path::PathBuf,
  sync::Arc,
};
use tokio::{io::AsyncWriteExt, sync::Semaphore};
use tower_http::{
  cors::CorsLayer,
  services::ServeDir,
  trace::{DefaultMakeSpan, TraceLayer},
};
use tracing_subscriber::{
  prelude::__tracing_subscriber_SubscriberExt, util::SubscriberInitExt,
};

use crate::jwt_utils::get_claims_from_jwt_token;

use tokio_util::io::ReaderStream;

use anyhow::{Result, bail};

use crate::handlers::*;

use manta_backend_dispatcher::StaticBackendDispatcher;

use commands::{delete_redfish, get_all_redfish, get_redfish, post_redfish};
use utoipa::{OpenApi, ToSchema, openapi::OpenApi as OpenApiDoc};

#[derive(OpenApi)]
#[openapi(
  info(
    title = "Manta API",
    description = "API for managing Manta services",
    version = "0.1.2"
  ),
  paths(root, test_ping, test_whoami, get_openapi, get_version, create_user,)
)]
pub struct ApiDoc;

#[tokio::main]
async fn main() {
  // initialize tracing
  tracing_subscriber::registry()
    .with(
      tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| "example_websockets=debug,tower_http=debug".into()),
    )
    .with(tracing_subscriber::fmt::layer())
    .init();

  let assets_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets");

  // build our application with a route
  let app = Router::new()
    .fallback_service(
      ServeDir::new(assets_dir).append_index_html_on_directories(true),
    )
    // `GET /` goes to `root`
    .route("/", get(root))
    .route("/test/whoami", get(test_whoami))
    .route("/test/ping", get(test_ping))
    .route("/test/ws", get(test_ws))
    .route("/openapi", get(get_openapi))
    .route("/version", get(get_version))
    .route("/users", post(create_user))
    .route("/cfs/health", get(get_cfs_health_check))
    .route("/bos/health", get(get_bos_health_check))
    .route("/kernel-parameters", get(get_kernel_parameters))
    .route("/bss/boot-parameters", get(get_all_bss_boot_parameters))
    .route("/bss/boot-parameters/{xname}", get(get_bss_boot_parameters))
    .route("/bss/boot-parameters", post(post_bss_boot_parameters))
    .route(
      "/bss/boot-parameters/{xname}",
      delete(delete_bss_boot_parameters),
    )
    .route("/redfish", get(get_all_redfish))
    .route("/redfish/{xname}", get(get_redfish))
    .route("/redfish", post(post_redfish))
    .route("/redfish/{xname}", delete(delete_redfish))
    .route("/authenticate", get(authenticate))
    .route("/console/{xname}", get(ws_console))
    .route("/cfssession/{cfssession}", get(get_cfs_session))
    .route("/cfssession/{cfssession}/logs", get(ws_cfs_session_logs))
    .route("/group", get(get_all_groups))
    .route("/group/{group}", get(get_group_details))
    .route("/group/{group}/hardware", get(get_hsm_hardware))
    .route("/node/{node}/power-off", get(power_off_node))
    .route("/node/{node}/power-on", get(power_on_node))
    .route("/node/{node}/power-reset", get(power_reset_node))
    .route("/node/{node}/power-status", get(power_status_node))
    .route(
      "/node-migration/target/{target}/parent/{parent}",
      put(node_migration),
    )
    .layer(CorsLayer::very_permissive())
    .layer(
      TraceLayer::new_for_http()
        .make_span_with(DefaultMakeSpan::default().include_headers(true)),
    );

  // run our app with hyper
  // `axum::Server` is a re-export of `hyper::Server`
  let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
  println!("listening on {}", addr);
  //    axum::Server::bind(&addr)
  //        .serve(app.into_make_service_with_connect_info::<SocketAddr>())
  //        .await
  //        .unwrap();
  axum::serve(tokio::net::TcpListener::bind(addr).await.unwrap(), app)
    .await
    .unwrap()
}

// the input to our `create_user` handler
#[derive(Deserialize, ToSchema)]
struct CreateUser {
  username: String,
}

// the output to our `create_user` handler
#[derive(Serialize, ToSchema)]
struct User {
  id: u64,
  username: String,
}

#[utoipa::path(
    get,
    path = "/",
    responses(
        (status = 200, description = "Hello world message", body = String)
    )
)]
async fn root() -> &'static str {
  println!("Hello, World!");
  "Hello, World!"
}

#[utoipa::path(
    get,
    path = "/version",
    responses(
        (status = 200, description = "Get manta-ws version", body = String)
    )
)]
async fn get_version() -> &'static str {
  env!("CARGO_PKG_VERSION")
}

#[utoipa::path(
    get,
    path = "/test/whoami",
    responses(
        (status = 200, description = "Test current user", body = String),
        (status = UNAUTHORIZED, description = "Authentication header/token missing")
    )
)]
async fn test_whoami(headers: HeaderMap) -> Result<String, StatusCode> {
  // Get auth token
  let auth_token = if let Some(auth_header) = headers.get("authorization") {
    auth_header.to_str().unwrap().split(" ").nth(1).unwrap()
  } else {
    return Err(StatusCode::UNAUTHORIZED);
  };

  let claims_json = get_claims_from_jwt_token(auth_token).unwrap();

  Ok(format!(
    "Hello {}!!!",
    claims_json["name"].as_str().unwrap()
  ))
}

#[utoipa::path(
    get,
    path = "/openapi",
    responses(
        (status = 200, description = "Get openapi json", body = String)
    )
)]
async fn get_openapi() -> impl IntoResponse {
  let openapi: OpenApiDoc = ApiDoc::openapi();
  Json(openapi)
}

#[utoipa::path(
    get,
    path = "/test/ping",
    responses(
        (status = 200, description = "Ping health endpoint", body = String)
    )
)]
async fn test_ping() -> &'static str {
  "Pong!"
}

#[utoipa::path(
    get,
    path = "/test/ws",
    responses(
        (status = 200, description = "Websocket test endpoint", body = String)
    )
)]
async fn test_ws(ws: WebSocketUpgrade) -> axum::response::Response {
  println!("Websocket test endpoint");
  ws.on_upgrade(handle_socket_test_ws)
}

async fn handle_socket_test_ws(mut socket: WebSocket) {
  while let Some(msg) = socket.recv().await {
    let msg = if let Ok(msg) = msg {
      println!("Received message: {:?}", msg);
      msg
    } else {
      // client disconnected
      return;
    };

    if socket.send(msg).await.is_err() {
      // client disconnected
      return;
    }
  }
}

#[utoipa::path(
    post,
    path = "/users",
    request_body = CreateUser,
    responses(
        (status = 201, description = "User created", body = User),
        (status = 400, description = "Invalid user data")
    )
)]
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

async fn get_cfs_session(
  headers: HeaderMap,
  Path(cfs_session_name): Path<String>,
) -> Result<Json<Value>, StatusCode> {
  // Configuration
  let settings = common::config::get_configuration().await.unwrap();

  let configuration: MantaConfiguration = settings.try_deserialize().unwrap();

  let site_name: String = configuration.site;
  let site_detail_value_opt = configuration.sites.get(&site_name);

  let site = match site_detail_value_opt {
    Some(site_detail_value) => site_detail_value,
    None => {
      eprintln!("ERROR - Site '{}' not found in configuration", site_name);
      std::process::exit(1);
    }
  };

  let backend_tech = &site.backend;
  let shasta_base_url = &site.shasta_base_url;

  let root_ca_cert_file = &site.root_ca_cert_file;

  let shasta_root_cert =
    common::config::get_csm_root_cert_content(&root_ca_cert_file).unwrap();

  // Backend
  let backend = StaticBackendDispatcher::new(
    &backend_tech,
    &shasta_base_url,
    &shasta_root_cert,
  );

  // Get auth token
  let auth_token = if let Some(auth_header) = headers.get("authorization") {
    auth_header.to_str().unwrap().split(" ").nth(1).unwrap()
  } else {
    return Err(StatusCode::UNAUTHORIZED);
  };

  let hsm_group_available_vec: Vec<String> =
    backend.get_group_name_available(&auth_token).await.unwrap();

  let cfs_session_vec = backend
    .get_and_filter_sessions(
      &auth_token,
      shasta_base_url,
      &shasta_root_cert,
      Some(hsm_group_available_vec),
      None,
      None,
      None,
      None,
      Some(&cfs_session_name),
      None,
      None,
    )
    .await
    .unwrap_or_else(|e| {
      tracing::error!("Failed to get CFS sessions. Reason:\n{e}");
      std::process::exit(1);
    });

  Ok(Json(serde_json::to_value(cfs_session_vec).unwrap()))
}

async fn ws_cfs_session_logs(
  headers: HeaderMap,
  Path(cfs_session_name): Path<String>,
  ws: WebSocketUpgrade,
  user_agent: Option<TypedHeader<headers::UserAgent>>,
  ConnectInfo(addr): ConnectInfo<SocketAddr>,
) -> impl IntoResponse {
  // Configuration
  let settings = common::config::get_configuration().await.unwrap();

  let configuration: MantaConfiguration = settings.try_deserialize().unwrap();

  let site_name: String = configuration.site.clone();
  let site_detail_value_opt = configuration.sites.get(&site_name).cloned();

  let site = site_detail_value_opt.unwrap();

  let k8s_details = site
    .k8s
    .expect("ERROR - k8s section not found in configuration");

  let backend_tech = &site.backend;
  let shasta_base_url = &site.shasta_base_url;

  let root_ca_cert_file = &site.root_ca_cert_file;

  let shasta_root_cert =
    common::config::get_csm_root_cert_content(&root_ca_cert_file).unwrap();

  // Backend
  let backend = StaticBackendDispatcher::new(
    &backend_tech,
    &shasta_base_url,
    &shasta_root_cert,
  );

  // Get auth token
  let auth_header = headers.get("authorization").unwrap();
  let auth_token = auth_header
    .to_str()
    .unwrap()
    .split(" ")
    .nth(1)
    .unwrap()
    .to_string();

  let user_agent = if let Some(TypedHeader(user_agent)) = user_agent {
    user_agent.to_string()
  } else {
    String::from("Unknown browser")
  };

  println!("`{user_agent}` at {addr} connected.");
  // finalize the upgrade process by returning upgrade callback.
  // we can customize the callback by sending additional info such as address.
  ws.on_upgrade(move |socket| {
    get_cfs_session_logs(
      socket,
      addr,
      backend,
      auth_token,
      site_name,
      cfs_session_name,
      k8s_details,
    )
  })
}

async fn get_cfs_session_logs(
  mut socket: WebSocket,
  _who: SocketAddr,
  backend: StaticBackendDispatcher,
  shasta_token: String,
  site_name: String,
  cfs_session_name: String,
  k8s_details: K8sDetails,
) {
  let logs_stream = backend
    .get_session_logs_stream(
      &shasta_token,
      &site_name,
      &cfs_session_name,
      &k8s_details,
    )
    .await
    .unwrap();

  let mut lines = logs_stream.lines();

  while let Some(line) = lines.try_next().await.unwrap() {
    if line.is_empty() {
      // FIXME: This is a hack to make sure that the logs are displayed properly
      // because for some reason websocat stops displaying logs if an empty line is
      // sent
      let _ = socket.send(Message::Text(Utf8Bytes::from(" "))).await;
    } else {
      let _ = socket.send(Message::Text(Utf8Bytes::from(line))).await;
    }
  }
}

pub fn get_configuration_file_path() -> PathBuf {
  // XDG Base Directory Specification
  let project_dirs = ProjectDirs::from(
    "local", /*qualifier*/
    "cscs",  /*organization*/
    "manta", /*application*/
  );

  PathBuf::from(project_dirs.unwrap().config_dir())
}

/// Reads configuration parameters related to manta from environment variables or file. If both
/// defiend, then environment variables takes preference
pub fn get_configuration() -> Config {
  let mut config_path = get_configuration_file_path();
  config_path.push("config.toml"); // ~/.config/manta/config is the file

  ::config::Config::builder()
    .add_source(::config::File::from(config_path))
    .add_source(
      ::config::Environment::with_prefix("MANTA")
        .try_parsing(true)
        .prefix_separator("_"),
    )
    .build()
    .unwrap()
}

pub fn get_csm_root_cert_content(site: &str) -> Vec<u8> {
  let mut config_path = get_configuration_file_path();
  config_path.push(site.to_string() + "_root_cert.pem");

  let mut buf = Vec::new();
  let root_cert_file_rslt = File::open(config_path);

  let _ = match root_cert_file_rslt {
    Ok(mut file) => file.read_to_end(&mut buf),
    Err(_) => {
      eprintln!("Root cert file for CSM not found. Exit");
      std::process::exit(1);
    }
  };

  buf
}

async fn authenticate(headers: HeaderMap) -> Result<String, StatusCode> {
  let settings = get_configuration();

  let site_detail_hashmap = settings.get_table("sites").unwrap();
  let site_detail_value = site_detail_hashmap
    .get("alps")
    .unwrap()
    .clone()
    .into_table()
    .unwrap();

  /* let shasta_base_url = site_detail_value
  .get("shasta_base_url")
  .unwrap()
  .to_string(); */
  let keycloak_base_url = site_detail_value
    .get("keycloak_base_url")
    .unwrap()
    .to_string();
  // let k8s_api_url = site_detail_value.get("k8s_api_url").unwrap().to_string();

  // let settings_hsm_group_name_opt = settings.get_string("hsm_group").ok();

  let shasta_root_cert = get_csm_root_cert_content("alps");

  let base64_user_credentials =
    if let Some(usercredentials) = headers.get("authorization") {
      usercredentials.to_str().unwrap()
    } else {
      return Err(StatusCode::UNAUTHORIZED);
    };

  let user_credentials_raw = String::from_utf8(
    base64::decode(base64_user_credentials.split(" ").nth(1).unwrap()).unwrap(),
  )
  .unwrap();

  let mut user_credentials = user_credentials_raw.split(":");

  let username = user_credentials.next().unwrap();
  let password = user_credentials.next().unwrap();

  let auth_token_result =
    csm_rs::common::authentication::get_token_from_shasta_endpoint(
      &keycloak_base_url,
      &shasta_root_cert,
      username,
      password,
    )
    .await;

  println!("DEBUG - TEST 2");

  match auth_token_result {
    Ok(auth_token) => Ok(auth_token),
    Err(error) => {
      eprintln!("ERROR - Authentication failed. Reason:\n{:#?}", error);
      Err(StatusCode::FORBIDDEN)
    }
  }
}

/// The handler for the HTTP request (this gets called when the HTTP GET lands at the start
/// of websocket negotiation). After this completes, the actual switching from HTTP to
/// websocket protocol will occur.
/// This is the last point where we can extract TCP/IP metadata such as IP address of the client
/// as well as things from HTTP headers such as user-agent of the browser etc.
async fn ws_console(
  headers: HeaderMap,
  Path(xname): Path<String>,
  ws: WebSocketUpgrade,
  user_agent: Option<TypedHeader<headers::UserAgent>>,
) -> impl IntoResponse {
  let user_agent = if let Some(TypedHeader(user_agent)) = user_agent {
    user_agent.to_string()
  } else {
    String::from("Unknown browser")
  };

  println!("`{user_agent}` connected.");
  // finalize the upgrade process by returning upgrade callback.
  // we can customize the callback by sending additional info such as address.
  ws.on_upgrade(move |socket| handle_socket(headers, socket, xname))
}

/// Actual websocket statemachine (one will be spawned per connection)
async fn handle_socket(headers: HeaderMap, socket: WebSocket, xname: String) {
  // Configuration
  let settings = common::config::get_configuration().await.unwrap();

  let configuration: MantaConfiguration = settings.try_deserialize().unwrap();

  let site_name: String = configuration.site;
  let site_detail_value_opt = configuration.sites.get(&site_name);

  let site = match site_detail_value_opt {
    Some(site_detail_value) => site_detail_value,
    None => {
      eprintln!("ERROR - Site '{}' not found in configuration", site_name);
      std::process::exit(1);
    }
  };

  let k8s_details = site
    .k8s
    .as_ref()
    .expect("ERROR - k8s section not found in configuration");

  // let backend_tech = &site.backend;
  // let shasta_base_url = &site.shasta_base_url;

  // let root_ca_cert_file = &site.root_ca_cert_file;

  /* let shasta_root_cert =
  common::config::get_csm_root_cert_content(&root_ca_cert_file).unwrap(); */

  // Backend
  /* let backend = StaticBackendDispatcher::new(
    &backend_tech,
    &shasta_base_url,
    &shasta_root_cert,
  ); */

  // Get auth token
  let auth_header = headers.get("authorization").unwrap().to_str().unwrap();
  let auth_token = auth_header.split(" ").nth(1).unwrap().to_string();

  let shasta_k8s_secrets = match &k8s_details.authentication {
    K8sAuth::Native {
      certificate_authority_data,
      client_certificate_data,
      client_key_data,
    } => {
      serde_json::json!({ "certificate-authority-data": certificate_authority_data, "client-certificate-data": client_certificate_data, "client-key-data": client_key_data })
    }
    K8sAuth::Vault {
      base_url: vault_base_url,
    } => fetch_shasta_k8s_secrets_from_vault(
      &vault_base_url,
      &auth_token,
      &site_name,
    )
    .await
    .unwrap(),
  };

  // By splitting socket we can send and receive at the same time. In this example we will send
  // unsolicited messages to client based on some sort of server's internal event (i.e .timer).
  let (mut sender, mut receiver) = socket.split();

  // CONSOLE

  let mut attached = csm_rs::node::console::get_container_attachment_to_conman(
    &xname.to_string(),
    &k8s_details.api_url,
    shasta_k8s_secrets,
  )
  .await
  .expect("ERROR - Unable to attach to container");

  // Hook stream from k8s conman container to the websocket
  let stdout_stream = ReaderStream::new(attached.stdout().unwrap());

  let mut stdin_writer = attached.stdin().unwrap();

  // This task will receive messages from the conman container and send them to the client
  let _send_task = tokio::spawn(async move {
    let _ = sender
      .send(Message::Text(Utf8Bytes::from(format!(
        "Connected to {}\n\r",
        xname
      ))))
      .await;

    let _ = sender
      .send(Message::Text(Utf8Bytes::from(
        "User &. key combination to exit the console\n\r".to_string(),
      )))
      .await;

    let _ = stdout_stream
      .map(|bytes: Result<Bytes, _>| {
        let bytes = bytes.unwrap(); // Handle error properly in production
        //let vec = &bytes.to_vec().unwrap();
        let text = std::str::from_utf8(&bytes).unwrap(); // Convert Bytes to &str
        Ok(Message::Text(Utf8Bytes::from(text)))
      })
      .forward(sender)
      .await;
  });

  // This second task will receive messages from client and print them on server console
  let _recv_task = tokio::spawn(async move {
    while let Some(message) = receiver.next().await {
      match message.as_ref() {
        Ok(Message::Close(_)) => {
          println!("Client sent CLOSE message:\n{:?}", message.unwrap());
          break;
        }
        Err(e) => {
          println!("Connection interrupted:\n{:?}", e);
          break;
        }
        _ => {
          let msg = message.unwrap();
          let value = msg.to_text().unwrap();
          println!("Message from xterm web client:\n{:?}", value);
          let _ = stdin_writer.write_all(value.as_bytes()).await;
        }
      }
    }
  })
  .await;
}

/// helper to print contents of messages to stdout. Has special treatment for Close.
fn process_message(msg: Message, who: SocketAddr) -> ControlFlow<(), ()> {
  match msg {
    Message::Text(t) => {
      println!(">>> {} sent str: {:?}", who, t);
    }
    Message::Binary(d) => {
      println!(">>> {} sent {} bytes: {:?}", who, d.len(), d);
    }
    Message::Close(c) => {
      if let Some(cf) = c {
        println!(
          ">>> {} sent close with code {} and reason `{}`",
          who, cf.code, cf.reason
        );
      } else {
        println!(">>> {} somehow sent close message without CloseFrame", who);
      }
      return ControlFlow::Break(());
    }

    Message::Pong(v) => {
      println!(">>> {} sent pong with {:?}", who, v);
    }
    // You should never need to manually handle Message::Ping, as axum's websocket library
    // will do so for you automagically by replying with Pong and copying the v according to
    // spec. But if you need the contents of the pings you can see them here.
    Message::Ping(v) => {
      println!(">>> {} sent ping with {:?}", who, v);
    }
  }
  ControlFlow::Continue(())
}

async fn get_service_health(
  headers: HeaderMap,
  service: &str,
) -> Result<Json<serde_json::Value>> {
  // Configuration
  let settings = common::config::get_configuration().await?;

  let configuration: MantaConfiguration = settings.try_deserialize()?;

  let site_name: String = configuration.site;
  let site_detail_value_opt = configuration.sites.get(&site_name);

  let site = match site_detail_value_opt {
    Some(site_detail_value) => site_detail_value,
    None => bail!("ERROR - Site '{}' not found in configuration", site_name),
  };

  let backend_tech = &site.backend;
  let shasta_base_url = &site.shasta_base_url;

  let root_ca_cert_file = &site.root_ca_cert_file;

  let shasta_root_cert =
    common::config::get_csm_root_cert_content(&root_ca_cert_file)?;

  // Backend
  let backend = StaticBackendDispatcher::new(
    &backend_tech,
    &shasta_base_url,
    &shasta_root_cert,
  );

  // Get auth token
  let auth_header = headers.get("authorization").unwrap().to_str().unwrap();
  let auth_token = auth_header.split(" ").nth(1).unwrap().to_string();

  let response: Value = match service {
    // NOTE: sending always 500 error is a BAD practice, we
    // should do proper error handling by making mesa to return the right error code,
    // then create the right HTTP status code based on it
    "cfs" => {
      csm_rs::cfs::common::health_check(
        &auth_token,
        &shasta_base_url,
        &shasta_root_cert,
      )
      .await?
    }
    "bos" => {
      csm_rs::bos::health_check::get(
        &auth_token,
        &shasta_base_url,
        &shasta_root_cert,
      )
      .await?
    }
    _ => bail!("Invalid service name"),
  };

  Ok(Json(response))
}

async fn get_cfs_health_check(headers: HeaderMap) -> Response {
  let response_rslt = get_service_health(headers, "cfs").await;

  match response_rslt {
    Ok(response) => return response.into_response(),
    Err(e) => {
      return (StatusCode::INTERNAL_SERVER_ERROR, Json(e.to_string()))
        .into_response();
    }
  }
}

async fn get_bos_health_check(headers: HeaderMap) -> Response {
  let response_rslt = get_service_health(headers, "bos").await;

  match response_rslt {
    Ok(response) => return response.into_response(),
    Err(e) => {
      return (StatusCode::INTERNAL_SERVER_ERROR, Json(e.to_string()))
        .into_response();
    }
  }
}

async fn get_all_bss_boot_parameters(headers: HeaderMap) -> Response {
  // Configuration
  let settings = common::config::get_configuration().await.unwrap();

  let configuration: MantaConfiguration = settings.try_deserialize().unwrap();

  let site_name: String = configuration.site;
  let site_detail_value_opt = configuration.sites.get(&site_name);

  let site = match site_detail_value_opt {
    Some(site_detail_value) => site_detail_value,
    None => {
      eprintln!("ERROR - Site '{}' not found in configuration", site_name);
      std::process::exit(1);
    }
  };

  let backend_tech = &site.backend;
  let shasta_base_url = &site.shasta_base_url;

  let root_ca_cert_file = &site.root_ca_cert_file;

  let shasta_root_cert =
    common::config::get_csm_root_cert_content(&root_ca_cert_file).unwrap();

  // Backend
  let backend = StaticBackendDispatcher::new(
    &backend_tech,
    &shasta_base_url,
    &shasta_root_cert,
  );

  // Get auth token
  let auth_header = headers.get("authorization").unwrap().to_str().unwrap();
  let auth_token = auth_header.split(" ").nth(1).unwrap();

  let boot_parameters_rslt = backend.get_all_bootparameters(auth_token).await;

  match boot_parameters_rslt {
    Ok(boot_parameters_vec) => {
      return (StatusCode::OK, Json(boot_parameters_vec)).into_response();
    }
    Err(e) => {
      return (StatusCode::INTERNAL_SERVER_ERROR, Json(e.to_string()))
        .into_response();
    }
  }
}

async fn get_bss_boot_parameters(
  headers: HeaderMap,
  Path(xname): Path<String>,
) -> Response {
  // Configuration
  let settings = common::config::get_configuration().await.unwrap();

  let configuration: MantaConfiguration = settings.try_deserialize().unwrap();

  let site_name: String = configuration.site;
  let site_detail_value_opt = configuration.sites.get(&site_name);

  let site = match site_detail_value_opt {
    Some(site_detail_value) => site_detail_value,
    None => {
      eprintln!("ERROR - Site '{}' not found in configuration", site_name);
      std::process::exit(1);
    }
  };

  let backend_tech = &site.backend;
  let shasta_base_url = &site.shasta_base_url;

  let root_ca_cert_file = &site.root_ca_cert_file;

  let shasta_root_cert =
    common::config::get_csm_root_cert_content(&root_ca_cert_file).unwrap();

  // Backend
  let backend = StaticBackendDispatcher::new(
    &backend_tech,
    &shasta_base_url,
    &shasta_root_cert,
  );

  // Get auth token
  let auth_header = headers.get("authorization").unwrap().to_str().unwrap();
  let auth_token = auth_header.split(" ").nth(1).unwrap();

  let boot_parameters_rslt =
    backend.get_bootparameters(auth_token, &[xname]).await;

  match boot_parameters_rslt {
    Ok(response) => return (StatusCode::OK, Json(response)).into_response(),
    Err(e) => {
      return (StatusCode::INTERNAL_SERVER_ERROR, Json(e.to_string()))
        .into_response();
    }
  }
}

async fn post_bss_boot_parameters(
  headers: HeaderMap,
  Json(boot_parameters): Json<BootParameters>,
) -> Response {
  // Configuration
  let settings = common::config::get_configuration().await.unwrap();

  let configuration: MantaConfiguration = settings.try_deserialize().unwrap();

  let site_name: String = configuration.site;
  let site_detail_value_opt = configuration.sites.get(&site_name);

  let site = match site_detail_value_opt {
    Some(site_detail_value) => site_detail_value,
    None => {
      eprintln!("ERROR - Site '{}' not found in configuration", site_name);
      std::process::exit(1);
    }
  };

  let backend_tech = &site.backend;
  let shasta_base_url = &site.shasta_base_url;

  let root_ca_cert_file = &site.root_ca_cert_file;

  let shasta_root_cert =
    common::config::get_csm_root_cert_content(&root_ca_cert_file).unwrap();

  // Backend
  let backend = StaticBackendDispatcher::new(
    &backend_tech,
    &shasta_base_url,
    &shasta_root_cert,
  );

  // Get auth token
  let auth_header = headers.get("authorization").unwrap().to_str().unwrap();
  let auth_token = auth_header.split(" ").nth(1).unwrap();

  let bss_boot_parameters_rslt = backend
    .add_bootparameters(auth_token, &boot_parameters)
    .await;

  match bss_boot_parameters_rslt {
    Ok(response) => return (StatusCode::OK, Json(response)).into_response(),
    Err(e) => {
      return (StatusCode::INTERNAL_SERVER_ERROR, Json(e.to_string()))
        .into_response();
    }
  }
}

async fn delete_bss_boot_parameters(
  headers: HeaderMap,
  Json(boot_parameters): Json<BootParameters>,
) -> Response {
  // Configuration
  let settings = common::config::get_configuration().await.unwrap();

  let configuration: MantaConfiguration = settings.try_deserialize().unwrap();

  let site_name: String = configuration.site;
  let site_detail_value_opt = configuration.sites.get(&site_name);

  let site = match site_detail_value_opt {
    Some(site_detail_value) => site_detail_value,
    None => {
      eprintln!("ERROR - Site '{}' not found in configuration", site_name);
      std::process::exit(1);
    }
  };

  let backend_tech = &site.backend;
  let shasta_base_url = &site.shasta_base_url;

  let root_ca_cert_file = &site.root_ca_cert_file;

  let shasta_root_cert =
    common::config::get_csm_root_cert_content(&root_ca_cert_file).unwrap();

  // Backend
  let backend = StaticBackendDispatcher::new(
    &backend_tech,
    &shasta_base_url,
    &shasta_root_cert,
  );

  // Get auth token
  let auth_header = headers.get("authorization").unwrap().to_str().unwrap();
  let auth_token = auth_header.split(" ").nth(1).unwrap();

  let bss_boot_parameters_rslt = backend
    .delete_bootparameters(auth_token, &boot_parameters)
    .await;

  match bss_boot_parameters_rslt {
    Ok(response) => return (StatusCode::OK, Json(response)).into_response(),
    Err(e) => {
      return (StatusCode::INTERNAL_SERVER_ERROR, Json(e.to_string()))
        .into_response();
    }
  }
}

async fn get_all_groups(headers: HeaderMap) -> Response {
  // Configuration
  let settings = common::config::get_configuration().await.unwrap();

  let configuration: MantaConfiguration = settings.try_deserialize().unwrap();

  let site_name: String = configuration.site;
  let site_detail_value_opt = configuration.sites.get(&site_name);

  let site = match site_detail_value_opt {
    Some(site_detail_value) => site_detail_value,
    None => {
      eprintln!("ERROR - Site '{}' not found in configuration", site_name);
      std::process::exit(1);
    }
  };

  let backend_tech = &site.backend;
  let shasta_base_url = &site.shasta_base_url;

  let root_ca_cert_file = &site.root_ca_cert_file;

  let shasta_root_cert =
    common::config::get_csm_root_cert_content(&root_ca_cert_file).unwrap();

  // Backend
  let backend = StaticBackendDispatcher::new(
    &backend_tech,
    &shasta_base_url,
    &shasta_root_cert,
  );

  // Get auth token
  let auth_header = headers.get("authorization").unwrap().to_str().unwrap();
  let auth_token = auth_header.split(" ").nth(1).unwrap();

  let hsm_group_available_name_vec = backend
    .get_group_available(auth_token)
    .await
    .unwrap()
    .iter()
    .map(|hsm_group| hsm_group.label.clone())
    .collect::<Vec<String>>();

  let response_rslt = backend.get_all_groups(&auth_token).await;

  match response_rslt {
    Ok(mut response) => {
      // Filter out groups that are not available
      response.retain(|hsm_group| {
        hsm_group_available_name_vec.contains(&hsm_group.label)
      });

      // Convert response to JSON
      return (StatusCode::OK, Json(response)).into_response();
    }
    Err(e) => {
      return (StatusCode::INTERNAL_SERVER_ERROR, Json(e.to_string()))
        .into_response();
    }
  }
}

async fn get_group_details(
  Path(group): Path<String>,
  headers: HeaderMap,
) -> Response {
  // Configuration
  let settings = common::config::get_configuration().await.unwrap();

  let configuration: MantaConfiguration = settings.try_deserialize().unwrap();

  let site_name: String = configuration.site;
  let site_detail_value_opt = configuration.sites.get(&site_name);

  let site = match site_detail_value_opt {
    Some(site_detail_value) => site_detail_value,
    None => {
      eprintln!("ERROR - Site '{}' not found in configuration", site_name);
      std::process::exit(1);
    }
  };

  let backend_tech = &site.backend;
  let shasta_base_url = &site.shasta_base_url;

  let root_ca_cert_file = &site.root_ca_cert_file;

  let shasta_root_cert =
    common::config::get_csm_root_cert_content(&root_ca_cert_file).unwrap();

  // Backend
  let backend = StaticBackendDispatcher::new(
    &backend_tech,
    &shasta_base_url,
    &shasta_root_cert,
  );

  // Get auth token
  let auth_header = headers.get("authorization").unwrap().to_str().unwrap();
  let auth_token = auth_header.split(" ").nth(1).unwrap();

  let group = backend.get_group(&auth_token, &group).await.unwrap();

  let hsm_groups_node_list = group.get_members();

  let response_rslt = csm_rs::node::utils::get_node_details(
    &auth_token,
    &shasta_base_url,
    &shasta_root_cert,
    hsm_groups_node_list,
  )
  .await;

  match response_rslt {
    Ok(response) => {
      return (StatusCode::OK, Json(response)).into_response();
    }
    Err(e) => {
      return (StatusCode::INTERNAL_SERVER_ERROR, Json(e.to_string()))
        .into_response();
    }
  }
}

async fn get_hsm_hardware(
  headers: HeaderMap,
  Path(group): Path<String>,
) -> Response {
  // Configuration
  let settings = common::config::get_configuration().await.unwrap();

  let configuration: MantaConfiguration = settings.try_deserialize().unwrap();

  let site_name: String = configuration.site;
  let site_detail_value_opt = configuration.sites.get(&site_name);

  let site = match site_detail_value_opt {
    Some(site_detail_value) => site_detail_value,
    None => {
      eprintln!("ERROR - Site '{}' not found in configuration", site_name);
      std::process::exit(1);
    }
  };

  let backend_tech = &site.backend;
  let shasta_base_url = &site.shasta_base_url;

  let root_ca_cert_file = &site.root_ca_cert_file;

  let shasta_root_cert =
    common::config::get_csm_root_cert_content(&root_ca_cert_file).unwrap();

  // Backend
  let backend = StaticBackendDispatcher::new(
    &backend_tech,
    &shasta_base_url,
    &shasta_root_cert,
  );

  // Get auth token
  let auth_header = headers.get("authorization").unwrap().to_str().unwrap();
  let auth_token = auth_header.split(" ").nth(1).unwrap();

  let hsm_group = csm_rs::hsm::group::http_client::get(
    &auth_token,
    &shasta_base_url,
    &shasta_root_cert,
    Some(&[&group]),
    None,
  )
  .await
  .unwrap();

  let hsm_group_target_members =
    csm_rs::hsm::group::utils::get_member_vec_from_hsm_group(
      &hsm_group.first().unwrap(),
    );

  let mut hsm_summary: Vec<NodeSummary> = Vec::new();

  let mut tasks = tokio::task::JoinSet::new();

  let sem = Arc::new(Semaphore::new(5)); // CSM 1.3.1 higher number of concurrent tasks won't
  // make it faster

  // Get HW inventory details for target HSM group
  for hsm_member in hsm_group_target_members.clone() {
    let shasta_token_string = auth_token.to_string(); // TODO: make it static
    let shasta_base_url_string = shasta_base_url.to_string(); // TODO: make it static
    let shasta_root_cert_vec = shasta_root_cert.to_vec();
    let hsm_member_string = hsm_member.to_string(); // TODO: make it static
    //
    let permit = Arc::clone(&sem).acquire_owned().await;

    tracing::info!("Getting HW inventory details for node '{}'", hsm_member);

    tasks.spawn(async move {
      let _permit = permit; // Wait semaphore to allow new tasks https://github.com/tokio-rs/tokio/discussions/2648#discussioncomment-34885
      csm_rs::hsm::hw_inventory::hw_component::http_client::get(
        &shasta_token_string,
        &shasta_base_url_string,
        &shasta_root_cert_vec,
        &hsm_member_string,
      )
      .await
      .unwrap()
    });
  }

  while let Some(message_rslt) = tasks.join_next().await {
    match message_rslt {
      Ok(node_summary) => {
        hsm_summary.push(node_summary);
      }
      Err(e) => {
        tracing::error!("Failed procesing/fetching node hw information");
        return (StatusCode::INTERNAL_SERVER_ERROR, Json(e.to_string()))
          .into_response();
      }
    }
  }

  tracing::debug!("DEBUG - result:\n{:?}", hsm_summary);

  return (StatusCode::OK, Json(hsm_summary)).into_response();
}

async fn power_off_node(
  Path(node): Path<String>,
  headers: HeaderMap,
) -> Response {
  tracing::info!("Power OFF node {}", node);

  // Configuration
  let settings = common::config::get_configuration().await.unwrap();

  let configuration: MantaConfiguration = settings.try_deserialize().unwrap();

  let site_name: String = configuration.site;
  let site_detail_value_opt = configuration.sites.get(&site_name);

  let site = match site_detail_value_opt {
    Some(site_detail_value) => site_detail_value,
    None => {
      eprintln!("ERROR - Site '{}' not found in configuration", site_name);
      std::process::exit(1);
    }
  };

  let backend_tech = &site.backend;
  let shasta_base_url = &site.shasta_base_url;

  let root_ca_cert_file = &site.root_ca_cert_file;

  let shasta_root_cert =
    common::config::get_csm_root_cert_content(&root_ca_cert_file).unwrap();

  // Backend
  let backend = StaticBackendDispatcher::new(
    &backend_tech,
    &shasta_base_url,
    &shasta_root_cert,
  );

  // Get auth token
  let auth_header = headers.get("authorization").unwrap().to_str().unwrap();
  let auth_token = auth_header.split(" ").nth(1).unwrap();

  let response_rslt = backend.power_off_sync(auth_token, &[node], true).await;

  match response_rslt {
    Ok(_) => return (StatusCode::OK, ()).into_response(),
    Err(e) => {
      return (StatusCode::INTERNAL_SERVER_ERROR, Json(e.to_string()))
        .into_response();
    }
  }
}

#[debug_handler]
async fn power_on_node(
  headers: HeaderMap,
  Path(node): Path<String>,
) -> Response {
  tracing::info!("Power ON node {}", node);

  // Configuration
  let settings = common::config::get_configuration().await.unwrap();

  let configuration: MantaConfiguration = settings.try_deserialize().unwrap();

  let site_name: String = configuration.site;
  let site_detail_value_opt = configuration.sites.get(&site_name);

  let site = match site_detail_value_opt {
    Some(site_detail_value) => site_detail_value,
    None => {
      eprintln!("ERROR - Site '{}' not found in configuration", site_name);
      std::process::exit(1);
    }
  };

  let backend_tech = &site.backend;
  let shasta_base_url = &site.shasta_base_url;

  let root_ca_cert_file = &site.root_ca_cert_file;

  let shasta_root_cert =
    common::config::get_csm_root_cert_content(&root_ca_cert_file).unwrap();

  // Backend
  let backend = StaticBackendDispatcher::new(
    &backend_tech,
    &shasta_base_url,
    &shasta_root_cert,
  );

  // Get auth token
  let auth_header = headers.get("authorization").unwrap().to_str().unwrap();
  let auth_token = auth_header.split(" ").nth(1).unwrap();

  let response_rslt = backend.power_on_sync(auth_token, &[node]).await;

  match response_rslt {
    Ok(_) => return (StatusCode::OK, ()).into_response(),
    Err(e) => {
      return (StatusCode::INTERNAL_SERVER_ERROR, Json(e.to_string()))
        .into_response();
    }
  }
}

async fn power_reset_node(
  headers: HeaderMap,
  Path(node): Path<String>,
) -> Response {
  tracing::debug!("Power RESET node {}", node);

  // Configuration
  let settings = common::config::get_configuration().await.unwrap();

  let configuration: MantaConfiguration = settings.try_deserialize().unwrap();

  let site_name: String = configuration.site;
  let site_detail_value_opt = configuration.sites.get(&site_name);

  let site = match site_detail_value_opt {
    Some(site_detail_value) => site_detail_value,
    None => {
      eprintln!("ERROR - Site '{}' not found in configuration", site_name);
      std::process::exit(1);
    }
  };

  let backend_tech = &site.backend;
  let shasta_base_url = &site.shasta_base_url;

  let root_ca_cert_file = &site.root_ca_cert_file;

  let shasta_root_cert =
    common::config::get_csm_root_cert_content(&root_ca_cert_file).unwrap();

  // Backend
  let backend = StaticBackendDispatcher::new(
    &backend_tech,
    &shasta_base_url,
    &shasta_root_cert,
  );

  // Get auth token
  let auth_header = headers.get("authorization").unwrap().to_str().unwrap();
  let auth_token = auth_header.split(" ").nth(1).unwrap();

  let response_rslt = backend.power_on_sync(auth_token, &[node]).await;

  match response_rslt {
    Ok(_) => return (StatusCode::OK, ()).into_response(),
    Err(e) => {
      return (StatusCode::INTERNAL_SERVER_ERROR, Json(e.to_string()))
        .into_response();
    }
  }
}

// TODO: these need to be imported from csm-rs and ochami-rs ? or dispatcher ?
#[derive(Deserialize, Debug)]
pub struct PowerStatusQueryParams {
  power_state_filter: Option<String>,
  management_state_filter: Option<String>,
}

async fn power_status_node(
  headers: HeaderMap,
  Path(node): Path<String>,
  Query(query_param): Query<PowerStatusQueryParams>,
) -> Result<impl IntoResponse, impl IntoResponse> {
  tracing::debug!("Power STATUS node {}", node);

  // Configuration
  let settings = common::config::get_configuration().await.unwrap();

  let configuration: MantaConfiguration = settings.try_deserialize().unwrap();

  let site_name: String = configuration.site;
  let site_detail_value_opt = configuration.sites.get(&site_name);

  let site = match site_detail_value_opt {
    Some(site_detail_value) => site_detail_value,
    None => {
      eprintln!("ERROR - Site '{}' not found in configuration", site_name);
      std::process::exit(1);
    }
  };

  let backend_tech = &site.backend;
  let shasta_base_url = &site.shasta_base_url;

  let root_ca_cert_file = &site.root_ca_cert_file;

  let shasta_root_cert =
    common::config::get_csm_root_cert_content(&root_ca_cert_file).unwrap();

  // Backend
  let backend = StaticBackendDispatcher::new(
    &backend_tech,
    &shasta_base_url,
    &shasta_root_cert,
  );

  // Get auth token
  let auth_header = headers.get("authorization").unwrap().to_str().unwrap();
  let auth_token = auth_header.split(" ").nth(1).unwrap();

  let response = backend
    .power_status(
      auth_token,
      &[node],
      query_param.power_state_filter.as_deref(), // Convert Option<String> to Option<&str>
      query_param.management_state_filter.as_deref(), // Convert Option<String> to Option<&str>
                                                      //power_state_filter,
                                                      //management_state_filter
    )
    .await;

  match response {
    Ok(response) => Ok(Json(response)),
    Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
  }
}

#[derive(Deserialize, Debug)]
pub struct NodeMigrationQueryParams {
  ids: String,
  create_hsm_group: bool,
}

async fn node_migration(
  Path((target, parent)): Path<(String, String)>,
  Query(query_param): Query<NodeMigrationQueryParams>,
  headers: HeaderMap,
) -> Response {
  tracing::info!(
    "Migrate nodes '{}' from parent '{}' to target {}. Create HSM group if doesn't exists? {}",
    query_param.ids,
    parent,
    target,
    query_param.create_hsm_group
  );

  let ids = query_param.ids;
  let create_hsm_group = query_param.create_hsm_group;

  let settings = get_configuration();

  let site_detail_hashmap = settings.get_table("sites").unwrap();
  let site_detail_value = site_detail_hashmap
    .get("alps")
    .unwrap()
    .clone()
    .into_table()
    .unwrap();

  let shasta_base_url = site_detail_value
    .get("shasta_base_url")
    .unwrap()
    .to_string();

  let shasta_root_cert = get_csm_root_cert_content("alps");

  // Get auth token
  let auth_token = if let Some(auth_header) = headers.get("authorization") {
    auth_header.to_str().unwrap().split(" ").nth(1).unwrap()
  } else {
    return (StatusCode::UNAUTHORIZED).into_response();
  };

  let new_target_hsm_members = ids
    .split(',')
    .map(|xname| xname.trim())
    .collect::<Vec<&str>>();

  if csm_rs::hsm::group::http_client::get(
    auth_token,
    &shasta_base_url,
    &shasta_root_cert,
    Some(&[&target]),
    None,
  )
  .await
  .is_ok()
  {
    tracing::debug!("The HSM group {} exists, good.", target);
  } else {
    if create_hsm_group {
      tracing::info!(
        "HSM group {} does not exist, but the option to create the group has been selected, creating it now.",
        target.to_string()
      );
      csm_rs::hsm::group::http_client::create_new_group(
        auth_token,
        &shasta_base_url,
        &shasta_root_cert,
        &target,
        &[],
        "false",
        "",
        &[],
      )
      .await
      .expect("Unable to create new HSM group");
    } else {
      tracing::error!(
        "HSM group {} does not exist, but the option to create the group was NOT specificied, cannot continue.",
        target.to_string()
      );
      return (StatusCode::UNPROCESSABLE_ENTITY).into_response();
    }
  }

  let _ = csm_rs::hsm::group::utils::migrate_hsm_members(
    auth_token,
    &shasta_base_url,
    &shasta_root_cert,
    &target,
    &parent,
    new_target_hsm_members,
    true,
  )
  .await;

  return ().into_response();
}
