mod backend_api;
mod backend_dispatcher;
mod common;
mod handlers;
mod http_response;
mod jwt_utils;
mod log;

use ::backend_dispatcher::{
    contracts::BackendTrait,
    interfaces::{cfs::CfsTrait, hsm::group::GroupTrait},
    types::{K8sAuth, K8sDetails, cfs::CfsSessionGetResponse},
};
use axum::{
    Json, Router, debug_handler,
    extract::{
        ConnectInfo, Path, Query, WebSocketUpgrade,
        ws::{Message, WebSocket, Utf8Bytes},
    },
    http::{ StatusCode, HeaderMap},
    response::IntoResponse,
    routing::{get, post, put},
};
use axum_extra::{ TypedHeader, headers };
use bytes::Bytes;
use common::config::types::MantaConfiguration;
use config::Config;
use directories::ProjectDirs;
use futures::{AsyncBufReadExt, SinkExt, StreamExt, TryStreamExt};
//use hyper::HeaderMap;
use mesa::{
    common::vault::http_client::fetch_shasta_k8s_secrets_from_vault,
    hsm::hw_inventory::hw_component::types::NodeSummary,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{fs::File, io::Read, net::SocketAddr, ops::ControlFlow, path::PathBuf, sync::Arc};
use tokio::{io::AsyncWriteExt, sync::Semaphore};
use tower_http::{
    cors::CorsLayer,
    services::ServeDir,
    trace::{DefaultMakeSpan, TraceLayer},
};
use tracing_subscriber::{prelude::__tracing_subscriber_SubscriberExt, util::SubscriberInitExt};

use crate::jwt_utils::get_claims_from_jwt_token;

use tokio_util::io::ReaderStream;

use anyhow::{Error, Result, bail};

use crate::handlers::*;

use backend_dispatcher::StaticBackendDispatcher;

use utoipa::{OpenApi, ToSchema, openapi::OpenApi as OpenApiDoc, path};
//use utoipa::OpenApi;
//use openapi_doc::ApiDoc;

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
        .fallback_service(ServeDir::new(assets_dir).append_index_html_on_directories(true))
        // `GET /` goes to `root`
        .route("/", get(root))
        .route("/test/whoami", get(test_whoami))
        .route("/test/ping", get(test_ping))
        .route("/openapi", get(get_openapi))
        .route("/version", get(get_version))
        // `POST /users` goes to `create_user`
        .route("/users", post(create_user))
        .route("/kernel-parameters", get(get_kernel_parameters))
        .route("/cfs/health", get(get_cfs_health_check))
        .route("/bos/health", get(get_bos_health_check))
        .route("/authenticate", get(authenticate))
        .route("/console/{xname}", get(ws_console))
        .route("/cfssession/{cfssession}", get(get_cfs_session))
        .route("/cfssession/{cfssession}/logs", get(ws_cfs_session_logs))
        .route("/hsm", get(get_hsm))

        .route("/hsm/{group}", get(get_hsm_details))
        .route("/hsm/{group}/hardware", get(get_hsm_hardware))
        .route("/node/{node}/power-off", get(power_off_node))
        .route("/node/{node}/power-on", get(power_on_node))
        .route("/node/{node}/power-reset", get(power_reset_node))

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
        (status = 200, description = "Test current user", body = String)
    )
)]
async fn test_whoami(headers: HeaderMap) -> String {
    let token = headers.get("authorization").unwrap().to_str().unwrap();

    let claims_json = get_claims_from_jwt_token(token).unwrap();

    format!("Hello {}!!!", claims_json["name"].as_str().unwrap())
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

async fn get_cfs_session(Path(cfs_session_name): Path<String>) -> Json<Value> {
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

    let backend_tech = &site.backend;
    let shasta_base_url = &site.shasta_base_url;
    let shasta_barebone_url = shasta_base_url // HACK to not break compatibility with
        // old configuration file. TODO: remove this when needed in the future and all users are
        // using the right configuration file
        .strip_suffix("/apis")
        .unwrap_or(&shasta_base_url);

    let shasta_api_url = match backend_tech.as_str() {
        "csm" => shasta_barebone_url.to_owned() + "/apis",
        "ochami" => shasta_barebone_url.to_owned(),
        _ => {
            eprintln!("Invalid backend technology");
            std::process::exit(1);
        }
    };

    let root_ca_cert_file = &site.root_ca_cert_file;

    let shasta_root_cert = common::config::get_csm_root_cert_content(&root_ca_cert_file).unwrap();

    // Backend
    let backend = StaticBackendDispatcher::new(&backend_tech, &shasta_base_url, &shasta_root_cert);

    // Get auth token
    let shasta_token = backend.get_api_token(&site_name).await.unwrap();

    let hsm_group_available_vec: Vec<String> = backend
        .get_group_name_available(&shasta_token)
        .await
        .unwrap();

    let cfs_session_vec = backend
        .get_and_filter_sessions(
            &shasta_token,
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

    dbg!(&cfs_session_vec);

    Json(serde_json::to_value(cfs_session_vec).unwrap())
}

async fn ws_cfs_session_logs(
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
    let shasta_barebone_url = shasta_base_url // HACK to not break compatibility with
        // old configuration file. TODO: remove this when needed in the future and all users are
        // using the right configuration file
        .strip_suffix("/apis")
        .unwrap_or(&shasta_base_url);

    let shasta_api_url = match backend_tech.as_str() {
        "csm" => shasta_barebone_url.to_owned() + "/apis",
        "ochami" => shasta_barebone_url.to_owned(),
        _ => {
            // FIXME: Do not exit like this
            eprintln!("Invalid backend technology");
            std::process::exit(1);
        }
    };

    let root_ca_cert_file = &site.root_ca_cert_file;

    let shasta_root_cert = common::config::get_csm_root_cert_content(&root_ca_cert_file).unwrap();

    // Backend
    let backend = StaticBackendDispatcher::new(&backend_tech, &shasta_base_url, &shasta_root_cert);

    // Get auth token
    let shasta_token = backend.get_api_token(&site_name).await.unwrap();

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
        } => fetch_shasta_k8s_secrets_from_vault(&vault_base_url, &shasta_token, &site_name)
            .await
            .unwrap(),
    };

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
            shasta_token,
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
    /* let settings = get_configuration();

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
    .await
    .expect("ERROR - Unable to fetch K8s secrets");

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

    let _ = socket
        .send(Message::Text(format!(
            "Fetching CFS session logs for {} ...",
            cfs_session_name
        )))
        .await;

    let (ansible_container, cfs_session_pod, pods_api) =
        kubernetes::get_cfs_session_container_ansible_logs_details(client, &cfs_session_name)
            .await
            .unwrap();

    let mut container_status =
        kubernetes::get_container_status(&cfs_session_pod, &ansible_container.name);

    let mut attempt = 0;
    let max_attempts = 3;

    if container_status.as_ref().unwrap().terminated.is_some() {
        // Print CFS session logs already terminated on screen
        let logs_stream_rslt =
            kubernetes::get_container_logs_stream(&ansible_container, &cfs_session_pod, &pods_api)
                .await;

        if let Ok(mut logs_stream) = logs_stream_rslt {
            while let Some(line) = logs_stream.try_next().await.unwrap() {
                if line.is_empty() {
                    // FIXME: This is a hack to make sure that the logs are displayed properly
                    // because for some reason websocat stops displaying logs if an empty line is
                    // sent
                    let _ = socket.send(Message::Text(" ".to_string())).await;
                } else {
                    let _ = socket.send(Message::Text(line)).await;
                }
            }
        }
    } else {
        // Print current CFS session logs on screen
        while container_status.as_ref().unwrap().running.is_some() && attempt < max_attempts {
            let logs_stream_rslt = kubernetes::get_container_logs_stream(
                &ansible_container,
                &cfs_session_pod,
                &pods_api,
            )
            .await;

            if let Ok(mut logs_stream) = logs_stream_rslt {
                while let Ok(line_opt) = logs_stream.try_next().await {
                    if let Some(line) = line_opt {
                        println!("{}", line);
                        let _ = socket.send(Message::Text(line)).await;
                    } else {
                        attempt += 1;
                    }
                }
            } else {
                attempt += 1;
            }

            container_status =
                kubernetes::get_container_status(&cfs_session_pod, &ansible_container.name);
        }
    } */

    dbg!(&shasta_token);
    dbg!(&site_name);
    dbg!(&cfs_session_name);
    dbg!(&k8s_details);

    let logs_stream = backend
        .get_session_logs_stream(&shasta_token, &site_name, &cfs_session_name, &k8s_details)
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
    _headers: HeaderMap,
) -> impl IntoResponse {
    // let cookie_header = headers.get("cookie").unwrap().to_str().unwrap();

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
async fn handle_socket(socket: WebSocket, _who: SocketAddr, xname: String) {
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

    let backend_tech = &site.backend;
    let shasta_base_url = &site.shasta_base_url;
    let shasta_barebone_url = shasta_base_url // HACK to not break compatibility with
        // old configuration file. TODO: remove this when needed in the future and all users are
        // using the right configuration file
        .strip_suffix("/apis")
        .unwrap_or(&shasta_base_url);

    let shasta_api_url = match backend_tech.as_str() {
        "csm" => shasta_barebone_url.to_owned() + "/apis",
        "ochami" => shasta_barebone_url.to_owned(),
        _ => {
            eprintln!("Invalid backend technology");
            std::process::exit(1);
        }
    };

    let root_ca_cert_file = &site.root_ca_cert_file;

    let shasta_root_cert = common::config::get_csm_root_cert_content(&root_ca_cert_file).unwrap();

    // Backend
    let backend = StaticBackendDispatcher::new(&backend_tech, &shasta_base_url, &shasta_root_cert);

    // Get auth token
    let shasta_token = backend.get_api_token(&site_name).await.unwrap();

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
        } => fetch_shasta_k8s_secrets_from_vault(&vault_base_url, &shasta_token, &site_name)
            .await
            .unwrap(),
    };

    // By splitting socket we can send and receive at the same time. In this example we will send
    // unsolicited messages to client based on some sort of server's internal event (i.e .timer).
    let (mut sender, mut receiver) = socket.split();

    // CONSOLE

    let mut attached = mesa::node::console::get_container_attachment_to_conman(
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
            .send(Message::Text(Utf8Bytes::from(format!("Connected to {}\n\r", xname))))
            .await;

        let _ = sender
            .send(Message::Text(
                Utf8Bytes::from(
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
            match message.as_ref().unwrap() {
                Message::Close(_) => {
                    println!("Client sent CLOSE message:\n{:#?}", message.unwrap());
                    break;
                }
                _ => {
                    let msg = message.unwrap();
                    let value = msg.to_text().unwrap();
                    println!("Message from xterm web client:\n{:#?}", value);
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

async fn get_service_health(headers: HeaderMap, service: &str) -> Result<Json<serde_json::Value>> {
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
    let shasta_barebone_url = shasta_base_url // HACK to not break compatibility with
        // old configuration file. TODO: remove this when needed in the future and all users are
        // using the right configuration file
        .strip_suffix("/apis")
        .unwrap_or(&shasta_base_url);

    let shasta_api_url = match backend_tech.as_str() {
        "csm" => shasta_barebone_url.to_owned() + "/apis",
        "ochami" => shasta_barebone_url.to_owned(),
        _ => bail!("Invalid backend technology".to_string()),
    };

    let root_ca_cert_file = &site.root_ca_cert_file;

    let shasta_root_cert = common::config::get_csm_root_cert_content(&root_ca_cert_file)?;

    // Backend
    let backend = StaticBackendDispatcher::new(&backend_tech, &shasta_base_url, &shasta_root_cert);

    // Get auth token
    let shasta_token = backend.get_api_token(&site_name).await?;

    let response: Value = match service {
        // NOTE: sending always 500 error is a BAD practice, we
        // should do proper error handling by making mesa to return the right error code,
        // then create the right HTTP status code based on it
        "cfs" => {
            mesa::cfs::common::health_check(&shasta_token, &shasta_base_url, &shasta_root_cert)
                .await?
        }
        "bos" => {
            mesa::bos::health_check::get(&shasta_token, &shasta_base_url, &shasta_root_cert).await?
        }
        _ => bail!("Invalid service name"),
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

async fn get_all_groups(headers: HeaderMap) -> Json<serde_json::Value> {
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

    let shasta_root_cert = common::config::get_csm_root_cert_content(&root_ca_cert_file).unwrap();

    // Backend
    let backend = StaticBackendDispatcher::new(&backend_tech, &shasta_base_url, &shasta_root_cert);

    // Get auth token
    let shasta_token = headers.get("authorization").unwrap().to_str().unwrap();
    // let shasta_token = backend.get_api_token(&site_name).await.unwrap();

    let hsm_group_available_name_vec = backend
        .get_group_available(shasta_token)
        .await
        .unwrap()
        .iter()
        .map(|hsm_group| hsm_group.label.clone())
        .collect::<Vec<String>>();

    let response_rslt = backend.get_all_groups(&shasta_token).await;

    match response_rslt {
        Ok(mut response) => {
            response.retain(|hsm_group| hsm_group_available_name_vec.contains(&hsm_group.label));
            Json(serde_json::to_value(response).unwrap())
        }
        Err(e) => {
            eprintln!("{}", e);
            std::process::exit(1);
        }
    }
}

async fn get_group_details(
    Path(group): Path<String>,
    headers: HeaderMap,
) -> Json<serde_json::Value> {
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

    let shasta_root_cert = common::config::get_csm_root_cert_content(&root_ca_cert_file).unwrap();

    // Backend
    let backend = StaticBackendDispatcher::new(&backend_tech, &shasta_base_url, &shasta_root_cert);

    // Get auth token
    let shasta_token = headers.get("authorization").unwrap().to_str().unwrap();
    // let shasta_token = backend.get_api_token(&site_name).await.unwrap();

    let group = backend.get_group(&shasta_token, &group).await.unwrap();

    let hsm_groups_node_list = group.get_members();

    let response = mesa::node::utils::get_node_details(
        &shasta_token,
        &shasta_base_url,
        &shasta_root_cert,
        hsm_groups_node_list,
    )
    .await
    .expect("ERROR - Unable to get node details");

    Json(serde_json::to_value(response).unwrap())
}

async fn get_hsm_hardware(
    Path(group): Path<String>,
    headers: HeaderMap,
) -> Json<serde_json::Value> {
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
    let shasta_barebone_url = shasta_base_url // HACK to not break compatibility with
        // old configuration file. TODO: remove this when needed in the future and all users are
        // using the right configuration file
        .strip_suffix("/apis")
        .unwrap_or(&shasta_base_url);

    let shasta_api_url = match backend_tech.as_str() {
        "csm" => shasta_barebone_url.to_owned() + "/apis",
        "ochami" => shasta_barebone_url.to_owned(),
        _ => {
            eprintln!("Invalid backend technology");
            std::process::exit(1);
        }
    };

    let root_ca_cert_file = &site.root_ca_cert_file;

    let shasta_root_cert = common::config::get_csm_root_cert_content(&root_ca_cert_file).unwrap();

    // Backend
    let backend = StaticBackendDispatcher::new(&backend_tech, &shasta_base_url, &shasta_root_cert);

    // Get auth token
    let shasta_token = backend.get_api_token(&site_name).await.unwrap();

    let hsm_group = mesa::hsm::group::http_client::get(
        &shasta_token,
        &shasta_base_url,
        &shasta_root_cert,
        Some(&[&group]),
        None,
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
            mesa::hsm::hw_inventory::hw_component::http_client::get(
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
        if let Ok(node_summary) = message {
            hsm_summary.push(node_summary);
        } else {
            tracing::error!("Failed procesing/fetching node hw information");
        }
    }

    println!("DEBUG - result:\n{:?}", hsm_summary);

    Json(serde_json::to_value(hsm_summary).unwrap())
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
) -> Result<(), StatusCode> {
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

    let shasta_token = if let Some(usercredentials) = headers.get("authorization") {
        usercredentials.to_str().unwrap().split(" ").nth(1).unwrap()
    } else {
        return Err(StatusCode::UNAUTHORIZED);
    };

    let new_target_hsm_members = ids
        .split(',')
        .map(|xname| xname.trim())
        .collect::<Vec<&str>>();

    if mesa::hsm::group::http_client::get(
        shasta_token,
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
            mesa::hsm::group::http_client::create_new_group(
                shasta_token,
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
            return Err(StatusCode::UNPROCESSABLE_ENTITY);
        }
    }

    let _ = mesa::hsm::group::utils::migrate_hsm_members(
        shasta_token,
        &shasta_base_url,
        &shasta_root_cert,
        &target,
        &parent,
        new_target_hsm_members,
        true,
    )
    .await;

    Ok(())
}
