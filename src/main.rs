mod jwt_utils;

use axum::{
    debug_handler,
    extract::{
        ws::{CloseFrame, Message, WebSocket},
        ConnectInfo, Path, WebSocketUpgrade,
    },
    headers,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router, TypedHeader,
};
use base64::decode;
use bytes::Bytes;
use config::Config;
use directories::ProjectDirs;
use hyper::HeaderMap;
use mesa::hsm::hw_inventory::hw_component::r#struct::NodeSummary;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    borrow::Cow, error::Error, fs::File, io::Read, net::SocketAddr, ops::ControlFlow,
    path::PathBuf, sync::Arc, time::Duration,
};
use tokio::{io::AsyncWriteExt, runtime::Runtime, sync::Semaphore};
use tower_http::{
    cors::CorsLayer,
    services::ServeDir,
    trace::{DefaultMakeSpan, TraceLayer},
};
use tracing_subscriber::{prelude::__tracing_subscriber_SubscriberExt, util::SubscriberInitExt};

use crate::jwt_utils::get_claims_from_jwt_token;

use futures_util::{SinkExt, Stream, StreamExt, TryStreamExt};
use tokio_util::io::{ReaderStream, SinkWriter};

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
        .fallback_service(ServeDir::new(assets_dir).append_index_html_on_directories(true))
        .route("/test/whoami", get(test_whoami))
        .route("/test/ping", get(test_ping))
        // `GET /` goes to `root`
        .route("/", get(root))
        // `POST /users` goes to `create_user`
        .route("/users", post(create_user))
        .route("/cfs/health", get(get_cfs_health_check))
        .route("/bos/health", get(get_bos_health_check))
        .route("/authenticate", get(authenticate))
        .route("/console/:xname", get(ws_console))
        .route("/cfssession", get(get_cfs_session))
        .route("/cfssession/:cfssession/logs", get(ws_cfs_session_logs))
        .route("/hsm", get(get_hsm))
        .route("/hsm/:hsm", get(get_hsm_details))
        .route("/hsm/:hsm/hardware", get(get_hsm_hardware))
        .route("/node/:node/power-off", get(power_off_node))
        .route("/node/:node/power-on", get(power_on_node))
        .route("/node/:node/power-reset", get(power_reset_node))
        .layer(CorsLayer::very_permissive())
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(DefaultMakeSpan::default().include_headers(true)),
        );

    // run our app with hyper
    // `axum::Server` is a re-export of `hyper::Server`
    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    tracing::debug!("listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service_with_connect_info::<SocketAddr>())
        .await
        .unwrap();
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

async fn root() -> &'static str {
    println!("Hello, World!");
    "Hello, World!"
}

async fn test_whoami(headers: HeaderMap) -> String {
    let token = headers.get("authorization").unwrap().to_str().unwrap();

    let claims_json = get_claims_from_jwt_token(token).unwrap();

    format!("Hello {}!!!", claims_json["name"].as_str().unwrap())
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

async fn ws_cfs_session_logs(
    Path(cfs_session_name): Path<String>,
    ws: WebSocketUpgrade,
    user_agent: Option<TypedHeader<headers::UserAgent>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
) -> impl IntoResponse {
    let user_agent = if let Some(TypedHeader(user_agent)) = user_agent {
        user_agent.to_string()
    } else {
        String::from("Unknown browser")
    };
    println!("`{user_agent}` at {addr} connected.");
    // finalize the upgrade process by returning upgrade callback.
    // we can customize the callback by sending additional info such as address.
    ws.on_upgrade(move |socket| get_cfs_session_logs(socket, addr, cfs_session_name))
}

async fn get_cfs_session_logs(mut socket: WebSocket, who: SocketAddr, cfs_session_name: String) {
    let settings = get_configuration();

    let site_detail_hashmap = settings.get_table("sites").unwrap();
    let site_detail_value = site_detail_hashmap
        .get("alps")
        .unwrap()
        .clone()
        .into_table()
        .unwrap();

    let vault_base_url = site_detail_value.get("vault_base_url").unwrap().to_string();
    let vault_role_id = site_detail_value.get("vault_role_id").unwrap().to_string();
    let vault_secret_path = site_detail_value
        .get("vault_secret_path")
        .unwrap()
        .to_string();
    let k8s_api_url = site_detail_value.get("k8s_api_url").unwrap().to_string();

    // GET K8S CLIENT

    let shasta_k8s_secrets = mesa::common::vault::http_client::fetch_shasta_k8s_secrets(
        &vault_base_url,
        &vault_secret_path,
        &vault_role_id,
    )
    .await;

    let client =
        mesa::common::kubernetes::get_k8s_client_programmatically(&k8s_api_url, shasta_k8s_secrets)
            .await
            .unwrap();

    // GET CFS SESSION

    /* let cfs_session_table_data_list =
    manta::cfs::session::get_sessions(shasta_token, shasta_base_url, None, Some(&cfs_session_name), Some(&1))
        .await; */

    // cfs_session_name = cfs_session_table_data_list.first().unwrap()[0];

    // GET CFS SESSION LOGS

    socket
        .send(Message::Text(format!(
            "Fetching CFS session logs for {} ...",
            cfs_session_name
        )))
        .await;

    let mut logs_stream = mesa::common::kubernetes::get_cfs_session_container_ansible_logs_stream(
        client,
        &cfs_session_name,
    )
    .await
    .unwrap();

    while let Some(line) = logs_stream.try_next().await.unwrap() {
        socket.send(Message::Text(format!("{}", &line))).await;
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

pub async fn get_hsm_name_available_from_jwt_or_all(
    shasta_token: &str,
    shasta_base_url: &str,
    shasta_root_cert: &[u8],
) -> Vec<String> {
    let mut realm_access_role_vec = get_claims_from_jwt_token(shasta_token)
        .unwrap()
        .pointer("/realm_access/roles")
        .unwrap_or(&serde_json::json!([]))
        .as_array()
        .unwrap_or(&Vec::new())
        .iter()
        .map(|role_value| role_value.as_str().unwrap().to_string())
        .collect::<Vec<String>>();

    realm_access_role_vec
        .retain(|role| !role.eq("offline_access") && !role.eq("uma_authorization"));

    if !realm_access_role_vec.is_empty() {
        realm_access_role_vec
    } else {
        mesa::hsm::group::http_client::get_all(shasta_token, shasta_base_url, shasta_root_cert)
            .await
            .unwrap()
            .iter()
            .map(|hsm_group| hsm_group.label.clone())
            .collect::<Vec<String>>()
    }
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

    let shasta_base_url = site_detail_value
        .get("shasta_base_url")
        .unwrap()
        .to_string();
    let keycloak_base_url = site_detail_value
        .get("keycloak_base_url")
        .unwrap()
        .to_string();
    let k8s_api_url = site_detail_value.get("k8s_api_url").unwrap().to_string();

    let settings_hsm_group_name_opt = settings.get_string("hsm_group").ok();

    let shasta_root_cert = get_csm_root_cert_content("alps");

    let base64_user_credentials = if let Some(usercredentials) = headers.get("authorization") {
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

    let auth_token_result = mesa::common::authentication::get_token_from_shasta_endpoint(
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
    Path(xname): Path<String>,
    ws: WebSocketUpgrade,
    user_agent: Option<TypedHeader<headers::UserAgent>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let cookie_header = headers.get("cookie").unwrap().to_str().unwrap();

    let user_agent = if let Some(TypedHeader(user_agent)) = user_agent {
        user_agent.to_string()
    } else {
        String::from("Unknown browser")
    };

    println!("`{user_agent}` at {addr} connected.");
    // finalize the upgrade process by returning upgrade callback.
    // we can customize the callback by sending additional info such as address.
    ws.on_upgrade(move |socket| handle_socket(socket, addr, xname))
}

/// Actual websocket statemachine (one will be spawned per connection)
async fn handle_socket(socket: WebSocket, who: SocketAddr, xname: String) {
    let settings = get_configuration();

    let site_detail_hashmap = settings.get_table("sites").unwrap();
    let site_detail_value = site_detail_hashmap
        .get("alps")
        .unwrap()
        .clone()
        .into_table()
        .unwrap();

    let vault_base_url = site_detail_value.get("vault_base_url").unwrap().to_string();
    let vault_role_id = site_detail_value.get("vault_role_id").unwrap().to_string();
    let vault_secret_path = site_detail_value
        .get("vault_secret_path")
        .unwrap()
        .to_string();
    let k8s_api_url = site_detail_value.get("k8s_api_url").unwrap().to_string();

    // By splitting socket we can send and receive at the same time. In this example we will send
    // unsolicited messages to client based on some sort of server's internal event (i.e .timer).
    let (mut sender, mut receiver) = socket.split();

    // CONSOLE

    let mut attached = mesa::node::console::get_container_attachment_to_conman(
        &xname.to_string(),
        &vault_base_url,
        &vault_secret_path,
        &vault_role_id,
        &k8s_api_url,
    )
    .await;

    // Hook stream from k8s conman container to the websocket
    let stdout_stream = ReaderStream::new(attached.stdout().unwrap());

    let mut stdin_writer = attached.stdin().unwrap();

    let send_task = tokio::spawn(async move {
        sender
            .send(Message::Text(format!("Connected to {}\n\r", xname)))
            .await;

        sender
            .send(Message::Text(
                "User &. key combination to exit the console\n\r".to_string(),
            ))
            .await;

        stdout_stream
            .map(|bytes| {
                Ok(Message::Text(
                    String::from_utf8(bytes.unwrap().to_vec()).unwrap(),
                ))
            })
            .forward(sender)
            .await;
    });

    // This second task will receive messages from client and print them on server console
    let recv_task = tokio::spawn(async move {
        while let Some(message) = receiver.next().await {
            match message.as_ref().unwrap() {
                Message::Close(_) => {
                    println!("Client sent CLOSE message:\n{:#?}", message.unwrap());
                    break;
                }
                _ => {
                    let msg = message.unwrap();
                    let value = msg.to_text().unwrap();
                    println!("Message from xterm web client:\n{:#?}", value);
                    stdin_writer.write_all(value.as_bytes()).await;
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
) -> Result<Json<serde_json::Value>, StatusCode> {
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

    let shasta_token = if let Some(usercredentials) = headers.get("authorization") {
        usercredentials.to_str().unwrap().split(" ").nth(1).unwrap()
    } else {
        return Err(StatusCode::UNAUTHORIZED);
    };

    let response: Value = match service {
        // NOTE: sending always 500 error is a BAD practice, we
        // should do proper error handling by making mesa to return the right error code,
        // then create the right HTTP status code based on it
        "cfs" => {
            mesa::cfs::common::health_check(&shasta_token, &shasta_base_url, &shasta_root_cert)
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        }
        "bos" => {
            mesa::bos::common::health_check(&shasta_token, &shasta_base_url, &shasta_root_cert)
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        }
        _ => return Err(StatusCode::INTERNAL_SERVER_ERROR),
    };

    Ok(Json(response))
}

async fn get_cfs_health_check(headers: HeaderMap) -> Result<Json<serde_json::Value>, StatusCode> {
    let response = get_service_health(headers, "cfs")
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    // NOTE: sending always 500 error is a BAD practice, we
    // should do proper error handling by making mesa to return the right error code,
    // then create the right HTTP status code based on it

    Ok(response)
}

async fn get_bos_health_check(headers: HeaderMap) -> Result<Json<serde_json::Value>, StatusCode> {
    let response = get_service_health(headers, "bos")
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    // NOTE: sending always 500 error is a BAD practice, we
    // should do proper error handling by making mesa to return the right error code,
    // then create the right HTTP status code based on it

    Ok(response)
}

async fn get_hsm(headers: HeaderMap) -> Result<Json<serde_json::Value>, StatusCode> {
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

    let shasta_token = if let Some(usercredentials) = headers.get("authorization") {
        usercredentials.to_str().unwrap().split(" ").nth(1).unwrap()
    } else {
        return Err(StatusCode::UNAUTHORIZED);
    };

    let hsm_group_available_name_vec =
        get_hsm_name_available_from_jwt_or_all(shasta_token, &shasta_base_url, &shasta_root_cert)
            .await;

    let response_rslt =
        mesa::hsm::group::http_client::get_all(&shasta_token, &shasta_base_url, &shasta_root_cert)
            .await;

    // let response_data = axum::Json(serde_json::to_value(response.as_ref().unwrap()).unwrap());

    if let Ok(mut response) = response_rslt {
        response.retain(|hsm_group| hsm_group_available_name_vec.contains(&hsm_group.label));

        return Ok(Json(serde_json::to_value(response).unwrap())); // FIX THIS: make cojin::shasta::hsm::http_client::get_hsm_groups to return Value instead of Vec<Value>
    } else {
        eprintln!("ERROR:\n{:#?}", response_rslt.unwrap());
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }
}

async fn get_hsm_details(
    Path(hsm): Path<String>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, StatusCode> {
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

    let shasta_token = if let Some(usercredentials) = headers.get("authorization") {
        usercredentials.to_str().unwrap().split(" ").nth(1).unwrap()
    } else {
        return Err(StatusCode::UNAUTHORIZED);
    };

    let hsm_group = mesa::hsm::group::http_client::get(
        &shasta_token,
        &shasta_base_url,
        &shasta_root_cert,
        Some(&hsm),
    )
    .await
    .unwrap();

    let hsm_groups_node_list =
        mesa::hsm::group::utils::get_member_vec_from_hsm_group(&hsm_group.first().unwrap());

    let response = mesa::node::utils::get_node_details(
        &shasta_token,
        &shasta_base_url,
        &shasta_root_cert,
        hsm_groups_node_list,
    )
    .await;

    Ok(Json(serde_json::to_value(response).unwrap()))
}

async fn get_hsm_hardware(
    Path(hsm): Path<String>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, StatusCode> {
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

    let shasta_token = if let Some(usercredentials) = headers.get("authorization") {
        usercredentials.to_str().unwrap().split(" ").nth(1).unwrap()
    } else {
        return Err(StatusCode::UNAUTHORIZED);
    };

    let hsm_group = mesa::hsm::group::http_client::get(
        &shasta_token,
        &shasta_base_url,
        &shasta_root_cert,
        Some(&hsm),
    )
    .await
    .unwrap();

    let hsm_group_target_members =
        mesa::hsm::group::utils::get_member_vec_from_hsm_group(&hsm_group.first().unwrap());

    let mut hsm_summary: Vec<NodeSummary> = Vec::new();

    let mut tasks = tokio::task::JoinSet::new();

    let sem = Arc::new(Semaphore::new(5)); // CSM 1.3.1 higher number of concurrent tasks won't
                                           // make it faster

    // Get HW inventory details for target HSM group
    for hsm_member in hsm_group_target_members.clone() {
        let shasta_token_string = shasta_token.to_string(); // TODO: make it static
        let shasta_base_url_string = shasta_base_url.to_string(); // TODO: make it static
        let shasta_root_cert_vec = shasta_root_cert.to_vec();
        let hsm_member_string = hsm_member.to_string(); // TODO: make it static
                                                        //
        let permit = Arc::clone(&sem).acquire_owned().await;

        tracing::info!("Getting HW inventory details for node '{}'", hsm_member);
        tasks.spawn(async move {
            let _permit = permit; // Wait semaphore to allow new tasks https://github.com/tokio-rs/tokio/discussions/2648#discussioncomment-34885
            mesa::hsm::hw_inventory::hw_component::http_client::get_hw_inventory(
                &shasta_token_string,
                &shasta_base_url_string,
                &shasta_root_cert_vec,
                &hsm_member_string,
            )
            .await
            .unwrap()
        });
    }

    while let Some(message) = tasks.join_next().await {
        if let Ok(mut node_hw_inventory) = message {
            node_hw_inventory = node_hw_inventory.pointer("/Nodes/0").unwrap().clone();
            let node_summary = NodeSummary::from_csm_value(node_hw_inventory.clone());
            hsm_summary.push(node_summary);
        } else {
            tracing::error!("Failed procesing/fetching node hw information");
        }
    }

    println!("DEBUG - result:\n{:?}", hsm_summary);

    Ok(Json(serde_json::to_value(hsm_summary).unwrap()))
}

async fn get_cfs_session() -> Json<serde_json::Value> {
    Json(json!({}))
}

async fn power_off_node(Path(node): Path<String>, headers: HeaderMap) -> Result<(), StatusCode> {
    tracing::info!("Power OFF node {}", node);

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

    let shasta_token = if let Some(usercredentials) = headers.get("authorization") {
        usercredentials.to_str().unwrap().split(" ").nth(1).unwrap()
    } else {
        return Err(StatusCode::UNAUTHORIZED);
    };

    let response_rslt = mesa::capmc::http_client::node_power_off::post_sync(
        shasta_token,
        &shasta_base_url,
        &shasta_root_cert,
        vec![node.clone()],
        Some("Web shutdown".to_string()),
        true,
    )
    .await;

    match response_rslt {
        Ok(_) => Ok(()),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

#[debug_handler]
async fn power_on_node(Path(node): Path<String>, headers: HeaderMap) -> Result<(), StatusCode> {
    tracing::info!("Power ON node {}", node);

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

    let shasta_token = if let Some(usercredentials) = headers.get("authorization") {
        usercredentials.to_str().unwrap().split(" ").nth(1).unwrap()
    } else {
        return Err(StatusCode::UNAUTHORIZED);
    };

    let response_rslt = mesa::capmc::http_client::node_power_on::post_sync(
        shasta_token,
        &shasta_base_url,
        &shasta_root_cert,
        vec![node.clone()],
        Some("Web shutdown".to_string()),
    )
    .await;

    match response_rslt {
        Ok(_) => Ok(()),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }

    /* // Wait for node's power state to be ON
    let i = 0;
    while i < 60 {
        tokio::time::sleep(Duration::from_secs(2)).await;

        let power_status_rslt = mesa::capmc::http_client::node_power_status::post(
            shasta_token,
            &shasta_base_url,
            &shasta_root_cert,
            &vec![node.clone()],
        )
        .await;

        match power_status_rslt {
            Ok(power_status) => {
                tracing::debug!("NODE {} POWER STATUS:\n{:#?}", node, power_status);

                if power_status["on"]
                    .as_array()
                    .is_some_and(|node_string| node_string.contains(&json!(node)))
                {
                    return Ok(());
                } else {
                    tracing::debug!("node {} not ON yet", node);
                    tracing::debug!("NODE {} POWER STATUS:\n{:#?}", node, power_status);
                }
            }
            Err(_) => {}
        }

        mesa::capmc::http_client::node_power_on::post(
            shasta_token,
            &shasta_base_url,
            &shasta_root_cert,
            vec![node.clone()],
            Some("Web shutdown".to_string()),
            false,
        )
        .await;
    }

    Err(StatusCode::INTERNAL_SERVER_ERROR) */
}

async fn power_reset_node(Path(node): Path<String>, headers: HeaderMap) -> Result<(), StatusCode> {
    tracing::debug!("Power RESET node {}", node);

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

    let shasta_token = if let Some(usercredentials) = headers.get("authorization") {
        usercredentials.to_str().unwrap().split(" ").nth(1).unwrap()
    } else {
        return Err(StatusCode::UNAUTHORIZED);
    };

    let response_rslt = mesa::capmc::http_client::node_power_off::post_sync(
        shasta_token,
        &shasta_base_url,
        &shasta_root_cert,
        vec![node.clone()],
        Some("Web shutdown".to_string()),
        true,
    )
    .await;

    match response_rslt {
        Ok(_) => Ok(()),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }

    /* // Wait for node's power state to be ON
    let i = 0;
    while i < 60 {
        tokio::time::sleep(Duration::from_secs(2)).await;

        let power_status = mesa::capmc::http_client::node_power_status::post(
            shasta_token,
            &shasta_base_url,
            &shasta_root_cert,
            &vec![node.clone()],
        )
        .await;

        if power_status.is_ok_and(|power_status_value| {
            power_status_value["on"]
                .as_array()
                .unwrap()
                .contains(&json!(node))
        }) {
            return Ok(());
        }

        mesa::capmc::http_client::node_power_off::post(
            shasta_token,
            &shasta_base_url,
            &shasta_root_cert,
            vec![node.clone()],
            Some("Web shutdown".to_string()),
            true,
        )
        .await;
    }

    Err(StatusCode::INTERNAL_SERVER_ERROR) */
}
