#[macro_use]
extern crate rocket;
#[macro_use]
extern crate log;

use std::path::PathBuf;

use rocket::{
    http::uri::Origin,
    response::{content::Html, status},
    Rocket, State,
};
use ssr::{JsRenderer, JsWorkerLog, Ssr, SsrConfig};

#[launch]
async fn rocket() -> Rocket {
    env_logger::init();

    let ssr = Ssr::new(SsrConfig {
        port: 9000,
        js_worker: PathBuf::from("./ssr/js/worker.js"),
        js_worker_log: JsWorkerLog::Verbose,
        global_js_renderer: Some(PathBuf::from(
            "./examples/rocket-hello-world/src/renderer.js",
        )),
    })
    .await
    .unwrap();

    rocket::ignite()
        .mount("/", routes![hello_world])
        .manage(ssr)
}

#[get("/")]
async fn hello_world(
    ssr: State<'_, Ssr>,
    origin: &Origin<'_>,
) -> Result<Html<String>, status::Custom<()>> {
    // TODO: Prolly, we should expect different uri type depending on specific feature
    let uri = match origin.query() {
        Some(query) => format!("{}?{}", origin.path(), query)
            .parse::<http::Uri>()
            .unwrap(),
        None => origin.path().parse::<http::Uri>().unwrap(),
    };
    match ssr.render(&uri, &"Hello, world!", JsRenderer::Global).await {
        Ok(html) => Ok(Html(html)),
        Err(error) => {
            error!("Error: {}", error);
            Err(status::Custom(
                rocket::http::Status::InternalServerError,
                (),
            ))
        }
    }
}
