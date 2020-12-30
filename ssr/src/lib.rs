//! ## Installation
//! Add to your `Cargo.toml`:
//!
//! ```toml
//! ssr = "0.0.2"
//! ```
//!
//! And install node worker from `npm`:
//!
//! ```sh
//! // using npm
//! npm install --save ssr-rs
//!
//! // or yarn
//! yarn add ssr-rs
//! ```
//!
//! ## How it works
//! On application start, you create an `Ssr` instance. Under the hood, it spins up a Node.js
//! worker ready to accept rendering requests. `Ssr` instance should be stored in a web server's
//! state, so handlers can access it during a handling of incoming requests.
//!
//! `Ssr` instance exposes a single method `render`, which accepts `Uri` and serializable data as
//! an input. If everything went smooth, it returns a rendered `String`. This string can be a plain
//! HTML or an app-specific encoded object with additional metadata—whatever returned from a JS
//! renderer, supplied by the app.
//!
//! ## Initialization
//!
//! ```rust
//! let ssr =
//!   Ssr::new(
//!     SsrConfig {
//!       port: 9000,
//!       js_worker: PathBuf::from("./node_modules/ssr-rs/worker.js"),
//!       global_js_renderer: Some(PathBuf::from("./js/ssr.js")),
//!     }
//!   );
//! ```
//!
//! ### `port`
//! A port that Node.js worker will be listening on.
//!
//! ### `js_worker`
//! Path to Node.js worker installed from `npm`. It should be relative to the
//! [`std::env::current_dir`](std::env::current_dir).
//!
//! ### `global_js_renderer`
//! If your web app is a SPA (Single Page Application), then you should have a single entry point
//! for all rendering requests. If it's the case, provide a path to this file here and it will be
//! used by the worker to render all responses. Another option is to provide a JS renderer per
//! request but keep in mind that it would introduce additional runtime overhead since JS module
//! has to be required during a request as opposed to requiring it once on application startup.
//!
//! ## Rendering
//! In request handlers, you need to get [`Ssr`](Ssr) instance from your server's state. Once you
//! have it (as well as all the required data to handle the current request), call
//! [`ssr.render`](Ssr::render) function with the following input:
//! - [`Uri`](http::Uri): uri of the current request
//! - `Data: impl Serialize`: anything that implements [`Serialize`](serde::Serialize)
//! - [`JsRenderer`](JsRenderer): an enum that tells to use either a global JS renderer or a
//! renderer specific to this request.
//!
//! ```rust
//! let uri = req.uri();
//! let data = db::get_data();
//! match ssr.render(uri, &data, JsRenderer::Global).await {
//!     Ok(html) => HttpResponse::Ok().body(html),
//!     Err(error) => {
//!         error!("Error: {}", error);
//!         HttpResponse::InternalServerError().finish()
//!     }
//! }
//! ```

#[macro_use]
extern crate log;
#[macro_use]
extern crate serde_json;

mod error;
mod json;
mod ssr;
mod worker;

pub use ssr::{JsRenderer, Ssr, SsrConfig};
