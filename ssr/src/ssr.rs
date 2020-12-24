use std::{fs, net::Shutdown, path::PathBuf, sync::Arc};

use http::Uri;
use serde::Serialize;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};
use uuid::Uuid;

use crate::{
    error::{InitializationError, RenderingError},
    worker::{Port, Worker},
};

pub enum JsRenderer {
    Global,
    PerRequest { path: PathBuf },
}

pub struct SsrConfig {
    pub port: u16,
    pub js_worker: PathBuf,
    pub global_js_renderer: Option<PathBuf>,
}

#[derive(Clone)]
pub struct Ssr {
    worker: Arc<Worker>,
    js_worker: PathBuf,
    global_js_renderer: Option<PathBuf>,
}

impl Ssr {
    pub async fn new(cfg: SsrConfig) -> Result<Self, InitializationError> {
        let port = Port::new(cfg.port);
        let js_worker = match fs::canonicalize(cfg.js_worker) {
            Ok(path) => path,
            Err(err) => return Err(InitializationError::InvalidJsWorkerPath(err)),
        };
        let global_js_renderer = match cfg.global_js_renderer {
            Some(path) => match fs::canonicalize(path) {
                Ok(path) => Some(path),
                Err(err) => return Err(InitializationError::InvalidGlobalJsRendererPath(err)),
            },
            None => None,
        };
        let worker = Worker::new(&port, &js_worker, &global_js_renderer).await?;
        Ok(Self {
            worker: Arc::new(worker),
            js_worker,
            global_js_renderer,
        })
    }

    pub async fn render<D: Serialize>(
        &self,
        uri: &Uri,
        data: &D,
        js_renderer: JsRenderer,
    ) -> Result<String, RenderingError> {
        let request_id = Uuid::new_v4();

        trace!("Starting request {}", request_id);

        let worker = &self.worker;

        let mut stream = match worker.connect().await {
            Ok(stream) => stream,
            Err(err) => {
                error!(
                    "{worker}: Failed to restart: {err}",
                    worker = worker.display_with_request_id(&request_id),
                    err = err
                );
                return Err(RenderingError::ConnectionError(err));
            }
        };

        let url = match uri.path_and_query() {
            Some(url) => url,
            None => {
                Self::finalize_rendering_session(&worker, &stream, &request_id);
                return Err(RenderingError::InvalidUri);
            }
        };

        let request_renderer = match (&self.global_js_renderer, js_renderer) {
            (Some(_), JsRenderer::Global) => None,
            (_, JsRenderer::PerRequest { path }) => Some(path),
            (None, JsRenderer::Global) => {
                Self::finalize_rendering_session(&worker, &stream, &request_id);
                return Err(RenderingError::GlobalRendererNotProvided);
            }
        };

        let meta = json!({
          "requestId": request_id,
          "requestRenderer": request_renderer,
          "url": json!({"path": url.path(), "query": url.query()}),
        });
        let meta_bytes = match serde_json::to_vec(&meta) {
            Ok(bytes) => bytes,
            Err(err) => {
                Self::finalize_rendering_session(&worker, &stream, &request_id);
                return Err(RenderingError::UrlSerializationError(err));
            }
        };
        let data = match serde_json::to_string(&data) {
            Ok(data) => data,
            Err(err) => {
                Self::finalize_rendering_session(&worker, &stream, &request_id);
                return Err(RenderingError::DataSerializationError(err));
            }
        };
        let data_bytes = match crate::json::to_vec(&data) {
            Ok(bytes) => bytes,
            Err(err) => {
                Self::finalize_rendering_session(&worker, &stream, &request_id);
                return Err(RenderingError::DataSerializationError(err));
            }
        };
        let meta_len = meta_bytes.len() as u32;
        let data_len = data_bytes.len() as u32;
        let meta_len_bytes = meta_len.to_be_bytes();
        let data_len_bytes = data_len.to_be_bytes();
        let mut input = meta_len_bytes.to_vec();
        input.extend_from_slice(&data_len_bytes);
        input.extend(meta_bytes);
        input.extend(data_bytes);

        let mut res = String::new();

        if let Err(err) = stream.write(input.as_slice()).await {
            Self::finalize_rendering_session(&worker, &stream, &request_id);
            return Err(RenderingError::RenderRequestError(err));
        };
        if let Err(err) = stream.read_to_string(&mut res).await {
            Self::finalize_rendering_session(&worker, &stream, &request_id);
            return Err(RenderingError::RenderResponseError(err));
        };

        // No need to shutdown connection as it's already closed by the js worker
        if res.starts_with("ERROR:") {
            match res.splitn(2, ':').collect::<Vec<_>>().as_slice() {
                ["ERROR", stack] => Err(RenderingError::JsExceptionDuringRendering(
                    stack.to_string(),
                )),
                _ => unreachable!(),
            }
        } else {
            Ok(res)
        }
    }

    fn finalize_rendering_session(worker: &Worker, connection: &TcpStream, request_id: &Uuid) {
        if let Err(err) = connection.shutdown(Shutdown::Both) {
            warn!(
                "{worker}: Failed to shutdown connection to the js worker: {err}",
                worker = worker.display_with_request_id(&request_id),
                err = err
            );
        };
    }
}
