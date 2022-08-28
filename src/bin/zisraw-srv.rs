//! Run with
//!
//! ```not_rust
//! cd examples && cargo run -p example-readme
//! ```

use axum::{
	http::StatusCode,
	response::IntoResponse,
	routing::{get, post},
	Json, Router,
	extract::Extension
};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use argh::FromArgs;
use zisrust::db::DB;
use zisrust::ImageInfo;

#[derive(FromArgs, PartialEq, Debug)]
#[argh(description = "sqlite backed registry for czi files")]
struct Cli {
	#[argh(option, short='d', default = "PathBuf::from(\"czi_registry.db\")")]
	/// path to the database file
	dbfile: PathBuf,
}

#[tokio::main]
async fn main() {
	// initialize tracing
	// tracing_subscriber::fmt::init();

	let cli: Cli = argh::from_env();
	let db=DB::new(&cli.dbfile).unwrap();
	let state = Arc::new(Mutex::new(db));

	// build our application with a route
	let app = Router::new()
		// `GET /` goes to `root`
		.route("/", get(root))
		// `POST /users` goes to `create_user`
		.route("/images", get(get_images))
		.layer(Extension(state))
		;

	// run our app with hyper
	// `axum::Server` is a re-export of `hyper::Server`
	let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
	// tracing::debug!("listening on {}", addr);
	axum::Server::bind(&addr)
		.serve(app.into_make_service())
		.await
		.unwrap();
}

#[derive(Serialize)]
struct ServerError{
	error:String
}

// basic handler that responds with a static string
async fn root() -> &'static str {
	"Hello, World!"
}

#[axum_macros::debug_handler]
async fn get_images(
	Extension(db): Extension<Arc<Mutex<DB>>>
) -> Json<Vec<ImageInfo>> {
	// match db.query_images(None){
	// 	Ok(images) =>
	// 		return (StatusCode::OK, Json(images)),
	// 	Err(e) =>
	// 		return (StatusCode::INTERNAL_SERVER_ERROR,Json(ServerError{error:e.to_string()}))
	// }
	let images=db.lock().unwrap().query_images(None).unwrap();
	Json(images)
}
