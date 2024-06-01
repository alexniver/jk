use std::{path::PathBuf, sync::Arc};

use askama::Template;
use askama_axum::IntoResponse;
use axum::{
    extract::{
        ws::{Message, WebSocket},
        State, WebSocketUpgrade,
    },
    routing::get,
    Router,
};
use futures::{SinkExt, StreamExt};
use tokio::{
    runtime::Runtime,
    sync::{mpsc::Receiver, oneshot, RwLock},
};

use crate::consts::PORT;

#[derive(Debug, Clone)]
struct AppState {
    // 接收shares change event
    rx: Arc<RwLock<Receiver<()>>>,
    share_path_arr: Arc<RwLock<Vec<PathBuf>>>,
}
impl AppState {
    fn new(rx: Arc<RwLock<Receiver<()>>>, share_path_arr: Arc<RwLock<Vec<PathBuf>>>) -> Self {
        Self { rx, share_path_arr }
    }
}

#[derive(Template)]
#[template(path = "index.html")]
pub struct IndexTemplate {
    pub file_arr: Vec<FileInfo>,
}

#[derive(Template)]
#[template(path = "file_list.html")]
pub struct FileListTemplate {
    pub file_arr: Vec<FileInfo>,
}

pub struct FileInfo {
    name: String,
    path: String,
}

pub fn run(
    rx: Arc<RwLock<Receiver<()>>>,
    share_path_arr: Arc<RwLock<Vec<PathBuf>>>,
    shutdown_rx: oneshot::Receiver<()>,
) {
    // 创建并启动一个新线程来运行Tokio事件循环
    std::thread::spawn(move || {
        let rt = Runtime::new().unwrap();
        rt.block_on(async {
            let app_state = AppState::new(rx, share_path_arr);
            let app = Router::new()
                .route("/", get(index))
                .route("/websocket", get(websocket_handler))
                .with_state(app_state);

            let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", PORT))
                .await
                .unwrap();
            tracing::debug!("listening on {}", listener.local_addr().unwrap());
            axum::serve(listener, app)
                .with_graceful_shutdown(async {
                    shutdown_rx.await.ok();
                })
                .await
                .unwrap();
        });
    });
}

async fn index(State(state): State<AppState>) -> impl IntoResponse {
    IndexTemplate {
        file_arr: path_arr_2_file_arr(state.share_path_arr).await,
    }
}

async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| websocket(socket, state))
}

async fn websocket(stream: WebSocket, state: AppState) {
    let (mut sender, _) = stream.split();
    let mut rx = state.rx.write().await;
    while let Some(_) = rx.recv().await {
        if sender.send(Message::Text("".to_string())).await.is_err() {
            break;
        }
    }
}

async fn path_arr_2_file_arr(path_arr: Arc<RwLock<Vec<PathBuf>>>) -> Vec<FileInfo> {
    let path_arr = path_arr.read().await;
    path_arr
        .iter()
        .map(|p| {
            let name = p
                .file_name()
                .map(|os_str| os_str.to_string_lossy().into_owned())
                .unwrap_or("".to_string());
            FileInfo {
                name,
                path: p.to_string_lossy().to_string(),
            }
        })
        .collect::<Vec<_>>()
}
