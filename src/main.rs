mod jwt_utils;

use axum::{
    extract::{
        ws::{CloseFrame, Message, WebSocket},
        ConnectInfo, WebSocketUpgrade, Path,
    },
    headers,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router, TypedHeader,
};
use bytes::Bytes;
use cojin::{
    cli::commands::log::get_cfs_session_logs_stream,
    common::vault::http_client::fetch_shasta_k8s_secrets,
    manta::{self, console::get_container_attachment},
    shasta::kubernetes::get_k8s_client_programmatically,
};
use hyper::HeaderMap;
use serde::{Deserialize, Serialize};
use std::{borrow::Cow, net::SocketAddr, ops::ControlFlow, path::PathBuf};
use tokio::{io::AsyncWriteExt, runtime::Runtime};
use tower_http::{
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
        .route("/console/:xname", get(ws_console))
        .route("/cfssession/:cfssession/logs", get(ws_cfs_session_logs))
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(DefaultMakeSpan::default().include_headers(true)),
        );

    // run our app with hyper
    // `axum::Server` is a re-export of `hyper::Server`
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
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

    let shasta_base_url = "https://api.cmn.alps.cscs.ch/apis";
    let shasta_token = "eyJhbGciOiJSUzI1NiIsInR5cCIgOiAiSldUIiwia2lkIiA6ICJNSW5BOEFfUUd4RTJ3REI5RlNkTzRKelVYSE9wVWFqZXVVb3JXemx1QlQwIn0.eyJqdGkiOiJhNDJiYTljYS1mODE3LTQ4YjgtOTFjYy0xMjBiMTAxMzQxNmMiLCJleHAiOjE2ODI0OTc4NzIsIm5iZiI6MCwiaWF0IjoxNjgyNDExNDcyLCJpc3MiOiJodHRwczovL2FwaS1ndy1zZXJ2aWNlLW5tbi5sb2NhbC9rZXljbG9hay9yZWFsbXMvc2hhc3RhIiwiYXVkIjpbImdhdGVrZWVwZXIiLCJzaGFzdGEiLCJhY2NvdW50Il0sInN1YiI6ImU5MWNjNGIzLTJlNWItNDExMC05N2Y1LWQ2YjAzYmJkMDRkYSIsInR5cCI6IkJlYXJlciIsImF6cCI6ImFkbWluLWNsaWVudCIsImF1dGhfdGltZSI6MCwic2Vzc2lvbl9zdGF0ZSI6IjQ5NDhlYjBlLTc1MTUtNGRiNS05NTM4LWFiYTI3NDY0YzI3OSIsImFjciI6IjEiLCJyZWFsbV9hY2Nlc3MiOnsicm9sZXMiOlsib2ZmbGluZV9hY2Nlc3MiLCJ1bWFfYXV0aG9yaXphdGlvbiJdfSwicmVzb3VyY2VfYWNjZXNzIjp7InNoYXN0YSI6eyJyb2xlcyI6WyJhZG1pbiJdfSwiYWNjb3VudCI6eyJyb2xlcyI6WyJtYW5hZ2UtYWNjb3VudCIsIm1hbmFnZS1hY2NvdW50LWxpbmtzIiwidmlldy1wcm9maWxlIl19fSwic2NvcGUiOiJwcm9maWxlIGVtYWlsIiwiY2xpZW50SWQiOiJhZG1pbi1jbGllbnQiLCJjbGllbnRIb3N0IjoiMTAuNDcuMTI4LjAiLCJlbWFpbF92ZXJpZmllZCI6ZmFsc2UsInByZWZlcnJlZF91c2VybmFtZSI6InNlcnZpY2UtYWNjb3VudC1hZG1pbi1jbGllbnQiLCJjbGllbnRBZGRyZXNzIjoiMTAuNDcuMTI4LjAifQ.Z-TvRIDwpJVRqkRTjKLzY4qOqFvrQQVxOHpL2dPM584eiD_nQHLVXxWbI37w2xd8Kmtw0jKsLu2xrgXRRtHn6cChDnlls6RhPX1xrt-LSc7A2VVRpIrOcY4AEG3G2ZluCdTIaxk3bKlxqNCDKBO1YRSc901Pmys1g26pLYjCN_RWF_RsVnmW_XR4v0xoBu5eJEXrM8ZQXCQFuxCJ7qLqMv5b8Bt3ga-uCIteRUZ83KZsuZGJEn0wSNorFxhBOORyaG82XfSyNJuGF-d-aACZIEh3yDmTQPPEdTYbQaLvNwZkHKbShC_DPVXPn8l-GN1cAUHByHFdIAyxI3vE2RblnA";

    // let gitea_base_url = "https://api.cmn.alps.cscs.ch/vcs";
    let gitea_token: &str;
    let vault_base_url = "https://hashicorp-vault.cscs.ch:8200";
    let vault_role_id = "b15517de-cabb-06ba-af98-633d216c6d99";
    // let cfs_session_name: &str;
    let k8s_api_url = "https://10.252.1.12:6442";

    let configuration_name: Option<String> = None;
    let most_recent: Option<bool> = None;
    let layer_id: Option<&u8> = None;
    // let xname: &str = "x1000c4s0b0n0";

    // GET CFS CONFIGURATION

    /* get_configuration::exec(
    gitea_token,
    shasta_token,
    shasta_base_url,
    configuration_name.as_ref(),
    hsm_group_name.as_ref(),
    most_recent,
    limit.as_ref(),
    )
    .await; */

    // GET K8S CLIENT

    let shasta_k8s_secrets = fetch_shasta_k8s_secrets(vault_base_url, vault_role_id).await;

    let client = get_k8s_client_programmatically(k8s_api_url, shasta_k8s_secrets)
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

    let mut logs_stream = get_cfs_session_logs_stream(client, &cfs_session_name, layer_id)
        .await
        .unwrap();

    while let Some(line) = logs_stream.try_next().await.unwrap() {
        socket
            .send(Message::Text(format!(
                "{}",
                std::str::from_utf8(&line).unwrap()
            )))
            .await;
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
) -> impl IntoResponse {
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
async fn handle_socket(mut socket: WebSocket, who: SocketAddr, xname: String) {
    /* //send a ping (unsupported by some browsers) just to kick things off and get a response
    if socket.send(Message::Ping(vec![1, 2, 3])).await.is_ok() {
        println!("Pinged {}...", who);
    } else {
        println!("Could not send ping {}!", who);
        // no Error here since the only thing we can do is to close the connection.
        // If we can not send messages, there is no way to salvage the statemachine anyway.
        return;
    }

    // receive single message from a client (we can either receive or send with socket).
    // this will likely be the Pong for our Ping or a hello message from client.
    // waiting for message from a client will block this task, but will not block other client's
    // connections.
    if let Some(msg) = socket.recv().await {
        if let Ok(msg) = msg {
            if process_message(msg, who).is_break() {
                return;
            }
        } else {
            println!("client {who} abruptly disconnected");
            return;
        }
    }

    // Since each client gets individual statemachine, we can pause handling
    // when necessary to wait for some external event (in this case illustrated by sleeping).
    // Waiting for this client to finish getting its greetings does not prevent other clients from
    // connecting to server and receiving their greetings.
    for i in 1..5 {
        if socket
            .send(Message::Text(format!("Hi {i} times!")))
            .await
            .is_err()
        {
            println!("client {who} abruptly disconnected");
            return;
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    } */

    /*
    // By splitting socket we can send and receive at the same time. In this example we will send
    // unsolicited messages to client based on some sort of server's internal event (i.e .timer).
    let (mut sender, mut receiver) = socket.split();

    // Spawn a task that will push several messages to the client (does not matter what client does)
    let mut send_task = tokio::spawn(async move {
        let n_msg = 20;
        for i in 0..n_msg {
            // In case of any websocket error, we exit.
            if sender
                .send(Message::Text(format!("Server message {i} ...")))
                .await
                .is_err()
            {
                return i;
            }

            tokio::time::sleep(std::time::Duration::from_millis(300)).await;
        }

        println!("Sending close to {who}...");
        if let Err(e) = sender
            .send(Message::Close(Some(CloseFrame {
                code: axum::extract::ws::close_code::NORMAL,
                reason: Cow::from("Goodbye"),
            })))
            .await
        {
            println!("Could not send Close due to {}, probably it is ok?", e);
        }
        n_msg
    });

    // If any one of the tasks exit, abort the other.
    tokio::select! {
        rv_a = (&mut send_task) => {
            match rv_a {
                Ok(a) => println!("{} messages sent to {}", a, who),
                Err(a) => println!("Error sending messages {:?}", a)
            }
            recv_task.abort();
        },
        rv_b = (&mut recv_task) => {
            match rv_b {
                Ok(b) => println!("Received {} messages", b),
                Err(b) => println!("Error receiving messages {:?}", b)
            }
            send_task.abort();
        }
    }

    // This second task will receive messages from client and print them on server console
    let mut recv_task = tokio::spawn(async move {
        let mut cnt = 0;
        while let Some(Ok(msg)) = receiver.next().await {
            cnt += 1;
            // print message and break if instructed to do so
            if process_message(msg, who).is_break() {
                break;
            }
        }
        cnt
    });

    // returning from the handler closes the websocket connection
    println!("Websocket context {} destroyed", who);
    */

    /* let welcome_msg = socket
        .send(Message::Text(
            "Hello from Cama backend!\r\nI am going to fetch data from Shasta using 'cojin'.\r\n"
                .to_string(),
        ))
        .await;

    if welcome_msg.is_err() {
        eprintln!("ERROR: {:#?}", welcome_msg);
    } */

    let shasta_base_url = "https://api.cmn.alps.cscs.ch/apis";
    let shasta_token = "eyJhbGciOiJSUzI1NiIsInR5cCIgOiAiSldUIiwia2lkIiA6ICJNSW5BOEFfUUd4RTJ3REI5RlNkTzRKelVYSE9wVWFqZXVVb3JXemx1QlQwIn0.eyJqdGkiOiJhNDJiYTljYS1mODE3LTQ4YjgtOTFjYy0xMjBiMTAxMzQxNmMiLCJleHAiOjE2ODI0OTc4NzIsIm5iZiI6MCwiaWF0IjoxNjgyNDExNDcyLCJpc3MiOiJodHRwczovL2FwaS1ndy1zZXJ2aWNlLW5tbi5sb2NhbC9rZXljbG9hay9yZWFsbXMvc2hhc3RhIiwiYXVkIjpbImdhdGVrZWVwZXIiLCJzaGFzdGEiLCJhY2NvdW50Il0sInN1YiI6ImU5MWNjNGIzLTJlNWItNDExMC05N2Y1LWQ2YjAzYmJkMDRkYSIsInR5cCI6IkJlYXJlciIsImF6cCI6ImFkbWluLWNsaWVudCIsImF1dGhfdGltZSI6MCwic2Vzc2lvbl9zdGF0ZSI6IjQ5NDhlYjBlLTc1MTUtNGRiNS05NTM4LWFiYTI3NDY0YzI3OSIsImFjciI6IjEiLCJyZWFsbV9hY2Nlc3MiOnsicm9sZXMiOlsib2ZmbGluZV9hY2Nlc3MiLCJ1bWFfYXV0aG9yaXphdGlvbiJdfSwicmVzb3VyY2VfYWNjZXNzIjp7InNoYXN0YSI6eyJyb2xlcyI6WyJhZG1pbiJdfSwiYWNjb3VudCI6eyJyb2xlcyI6WyJtYW5hZ2UtYWNjb3VudCIsIm1hbmFnZS1hY2NvdW50LWxpbmtzIiwidmlldy1wcm9maWxlIl19fSwic2NvcGUiOiJwcm9maWxlIGVtYWlsIiwiY2xpZW50SWQiOiJhZG1pbi1jbGllbnQiLCJjbGllbnRIb3N0IjoiMTAuNDcuMTI4LjAiLCJlbWFpbF92ZXJpZmllZCI6ZmFsc2UsInByZWZlcnJlZF91c2VybmFtZSI6InNlcnZpY2UtYWNjb3VudC1hZG1pbi1jbGllbnQiLCJjbGllbnRBZGRyZXNzIjoiMTAuNDcuMTI4LjAifQ.Z-TvRIDwpJVRqkRTjKLzY4qOqFvrQQVxOHpL2dPM584eiD_nQHLVXxWbI37w2xd8Kmtw0jKsLu2xrgXRRtHn6cChDnlls6RhPX1xrt-LSc7A2VVRpIrOcY4AEG3G2ZluCdTIaxk3bKlxqNCDKBO1YRSc901Pmys1g26pLYjCN_RWF_RsVnmW_XR4v0xoBu5eJEXrM8ZQXCQFuxCJ7qLqMv5b8Bt3ga-uCIteRUZ83KZsuZGJEn0wSNorFxhBOORyaG82XfSyNJuGF-d-aACZIEh3yDmTQPPEdTYbQaLvNwZkHKbShC_DPVXPn8l-GN1cAUHByHFdIAyxI3vE2RblnA";

    // let gitea_base_url = "https://api.cmn.alps.cscs.ch/vcs";
    let gitea_token: &str;
    let vault_base_url = "https://hashicorp-vault.cscs.ch:8200";
    let vault_role_id = "b15517de-cabb-06ba-af98-633d216c6d99";
    let cfs_session_name: &str;
    let k8s_api_url = "https://10.252.1.12:6442";

    let configuration_name: Option<String> = None;
    let most_recent: Option<bool> = None;
    let layer_id: Option<&u8> = None;
    // let xname: &str = "x1000c4s0b0n0";

    // GET CFS CONFIGURATION

    /* get_configuration::exec(
    gitea_token,
    shasta_token,
    shasta_base_url,
    configuration_name.as_ref(),
    hsm_group_name.as_ref(),
    most_recent,
    limit.as_ref(),
    )
    .await; */

    // GET K8S CLIENT

    let shasta_k8s_secrets = fetch_shasta_k8s_secrets(vault_base_url, vault_role_id).await;

    let client = get_k8s_client_programmatically(k8s_api_url, shasta_k8s_secrets)
        .await
        .unwrap();

    // By splitting socket we can send and receive at the same time. In this example we will send
    // unsolicited messages to client based on some sort of server's internal event (i.e .timer).
    let (mut sender, mut receiver) = socket.split();

    // CONSOLE

    let mut attached = get_container_attachment(
        &xname.to_string(),
        vault_base_url,
        vault_role_id,
        k8s_api_url,
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
