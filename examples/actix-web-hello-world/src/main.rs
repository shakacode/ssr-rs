#[macro_use]
extern crate log;

use std::path::PathBuf;

use actix_web::{web, web::Data, App, HttpRequest, HttpResponse, HttpServer};
use ssr::{JsRenderer, JsWorkerLog, Ssr, SsrConfig};

#[actix_web::main]
pub async fn main() -> std::io::Result<()> {
    env_logger::init();

    let ssr = Ssr::new(SsrConfig {
        port: 9000,
        js_worker: PathBuf::from("./ssr/js/worker.js"),
        js_worker_log: JsWorkerLog::Verbose,
        global_js_renderer: Some(PathBuf::from(
            "./examples/actix-web-hello-world/src/renderer.js",
        )),
    })
    .await
    .unwrap();

    HttpServer::new(move || {
        App::new()
            .app_data(Data::new(ssr.clone()))
            .route("/", web::get().to(hello_world))
    })
    .bind("127.0.0.1:3000")?
    .run()
    .await
}

pub async fn hello_world(ssr: Data<Ssr>, req: HttpRequest) -> HttpResponse {
    let uri = req.uri();
    match ssr.render(uri, &"Hello, world!", JsRenderer::Global).await {
        Ok(html) => HttpResponse::Ok().body(html),
        Err(error) => {
            error!("Error: {}", error);
            HttpResponse::InternalServerError().finish()
        }
    }
}
