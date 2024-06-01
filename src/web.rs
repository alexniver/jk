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
    sync::{mpsc::Receiver, oneshot, RwLock},
};
use tokio_util::io::ReaderStream;

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
    let mut rx = state.rx.write().await;

    let list_string = get_list_string(state.clone()).await;
    let _ = sender.send(Message::Text(list_string.to_string())).await;

    while let Some(_) = rx.recv().await {
        let list_string = get_list_string(state.clone()).await;
        if sender.send(Message::Text(list_string)).await.is_err() {
            break;
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

async fn download(Query(p): Query<DownloadParam>) -> impl IntoResponse {
    // 调用上面定义的函数来处理下载
    match stream_file(Path::new(&p.path)).await {
        Ok(response_body) => response_body.into_response(),
        Err(e) => {
            eprintln!("Error streaming file: {}", e);
            // 返回一个错误响应，实际应用中可能需要更详细的错误处理
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to stream file",
            )
                .into_response()
        }
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
