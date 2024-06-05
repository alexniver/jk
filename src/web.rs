use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use askama::Template;
use askama_axum::IntoResponse;
use axum::{
    body::Body,
    extract::{
        ws::{Message, WebSocket},
        Query, State, WebSocketUpgrade,
    },
    routing::get,
    Router,
};
use futures::{SinkExt, StreamExt};
use serde::Deserialize;
use tokio::{
    fs::File,
    runtime::Runtime,
    sync::{
        broadcast::{self},
        mpsc::Receiver,
        oneshot, RwLock,
    },
};
use tokio_util::io::ReaderStream;

use crate::consts::PORT;

#[derive(Debug, Clone)]
struct AppState {
    share_path_arr: Arc<RwLock<Vec<PathBuf>>>,
    broadcast_tx: broadcast::Sender<()>,
}
impl AppState {
    fn new(share_path_arr: Arc<RwLock<Vec<PathBuf>>>, broadcast_tx: broadcast::Sender<()>) -> Self {
        Self {
            share_path_arr,
            broadcast_tx,
        }
    }
}

#[derive(Template)]
#[template(path = "index.html")]
pub struct IndexTemplate;

#[derive(Template)]
#[template(path = "file_list.html")]
pub struct FileListTemplate {
    pub file_arr: Vec<FileInfo>,
    pub is_hx_swap_oob: bool,
}

pub struct FileInfo {
    name: String,
    path: String,
}

pub fn run(
    mut rx: Receiver<()>,
    share_path_arr: Arc<RwLock<Vec<PathBuf>>>,
    shutdown_rx: oneshot::Receiver<()>,
) {
    std::thread::spawn(move || {
        let rt = Runtime::new().unwrap();
        rt.block_on(async {
            let (broadcast_tx, _) = broadcast::channel(16);

            let tx_clone = broadcast_tx.clone();
            // rx 收到信号，广播
            tokio::spawn(async move {
                while (rx.recv().await).is_some() {
                    let _ = tx_clone.send(());
                }
            });

            let app_state = AppState::new(share_path_arr, broadcast_tx);
            let app = Router::new()
                .route("/", get(index))
                .route("/download", get(download))
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

async fn index() -> impl IntoResponse {
    IndexTemplate.into_response()
}

async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| websocket(socket, state))
}

async fn websocket(stream: WebSocket, state: AppState) {
    let (mut sender, _) = stream.split();
    let mut rx = state.broadcast_tx.subscribe();

    let list_string = get_list_string(state.clone()).await;
    let _ = sender.send(Message::Text(list_string.to_string())).await;

    while (rx.recv().await).is_ok() {
        let list_string = get_list_string(state.clone()).await;
        match sender.send(Message::Text(list_string)).await {
            Ok(_) => {}
            Err(e) => {
                tracing::error!("send message error: {e}");
                break;
            }
        }
    }
}

async fn get_list_string(state: AppState) -> String {
    let list = FileListTemplate {
        file_arr: path_arr_2_file_arr(state.share_path_arr).await,
        is_hx_swap_oob: true,
    };
    match list.render() {
        Ok(l) => l,
        Err(e) => {
            tracing::error!("list render fail, e: {}", e);
            panic!("{}", e);
        }
    }
}

#[derive(Deserialize)]
struct DownloadParam {
    path: String,
}

async fn download(
    Query(p): Query<DownloadParam>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let path_to_download = Path::new(&p.path);
    let share_path_arr = state.share_path_arr.read().await;
    if share_path_arr.iter().any(|p| p.eq(path_to_download)) {
        // 调用上面定义的函数来处理下载
        match stream_file(Path::new(&p.path)).await {
            Ok(response_body) => response_body.into_response(),
            Err(e) => {
                tracing::error!("Error streaming file: {}", e);
                // 返回一个错误响应，实际应用中可能需要更详细的错误处理
                (
                    axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                    "Failed to stream file",
                )
                    .into_response()
            }
        }
    } else {
        tracing::error!("Error streaming file, file isn't share");
        // 返回一个错误响应，实际应用中可能需要更详细的错误处理
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to stream file, file isn't share",
        )
            .into_response()
    }
}

async fn stream_file(path: &Path) -> Result<impl IntoResponse, std::io::Error> {
    let file = File::open(path).await?;
    let stream = ReaderStream::new(tokio::io::BufReader::new(file));
    Ok(Body::from_stream(stream))
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
