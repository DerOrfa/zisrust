use axum::{
	http::StatusCode,
	response::IntoResponse,
	routing::{get, post},
	Json, Router,
	extract::Extension
};
use serde::Deserialize;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use argh::FromArgs;
use axum::extract::Path;
use axum::response::Response;
use uuid::Uuid;
use zisrust::db::{DB, RegisterSuccess};
use zisrust::ImageInfo;

#[derive(FromArgs, PartialEq, Debug)]
#[argh(description = "sqlite backed registry for czi files")]
struct Cli {
	#[argh(option, short='d', default = "PathBuf::from(\"czi_registry.db\")")]
	/// path to the database file
	dbfile: PathBuf,

	#[argh(option, short='i', default = "SocketAddr::from(([127,0,0,1],3000))")]
	/// ip adress to listen at
	address:SocketAddr
}

#[tokio::main]
async fn main() {
	// initialize tracing
	// tracing_subscriber::fmt::init();

	let cli: Cli = argh::from_env();
	println!("opening database {}",cli.dbfile.to_string_lossy());
	let db=DB::new(&cli.dbfile).unwrap();
	let state = Arc::new(Mutex::new(db));

	// build our application with a route
	let app = Router::new()
		// `GET /` goes to `root`
		.route("/", get(root))
		// `POST /users` goes to `create_user`
		.route("/images", get(get_images))
		.route("/images", post(register_image))
		.route("/images/:uuid/xml", get(get_image_xml))
		.route("/images/:uuid/thumbnail", get(get_image_thumbnail))
		.layer(Extension(state))
		;

	// run our app with hyper
	// `axum::Server` is a re-export of `hyper::Server`
	// tracing::debug!("listening on {}", addr);
	println!("starting server at {}",cli.address);
	axum::Server::bind(&cli.address)
		.serve(app.into_make_service())
		.await
		.unwrap();
}

// basic handler that responds with a static string
async fn root() -> &'static str {
	"Hello, World!"
}

async fn get_images(Extension(db): Extension<Arc<Mutex<DB>>>) -> Response {
	match db.lock().unwrap().query_images(None){
		Ok(images) => (StatusCode::OK,Json(images)).into_response(),
		Err(e) => (StatusCode::INTERNAL_SERVER_ERROR,Json(e)).into_response()
	}
}

async fn get_image(Path(id):Path<Uuid>, Extension(db): Extension<Arc<Mutex<DB>>>) -> Response {
	match db.lock().unwrap().query_images(Some(format!("guid==\"{id}\""))){
		Ok(images) => {
			match images.first() {
				None => (StatusCode::NOT_FOUND).into_response(),
				Some(image) => (StatusCode::INTERNAL_SERVER_ERROR,Json(image)).into_response()
			}
		},
		Err(e) => (StatusCode::INTERNAL_SERVER_ERROR,Json(e)).into_response()
	}
}

#[derive(Deserialize)]
struct RegisterImagePayload{filename:PathBuf}
async fn register_image(Json(payload):Json<RegisterImagePayload>, Extension(db): Extension<Arc<Mutex<DB>>>) -> Response {
	if ! payload.filename.exists(){
		return StatusCode::NOT_FOUND.into_response();
	}
	if ! payload.filename.is_file(){
		return StatusCode::NOT_ACCEPTABLE.into_response()
	}
	// todo handle missing read access
	match db.lock().unwrap().register_file(&payload.filename){
		Ok(r) => {
			match r {
				RegisterSuccess::Inserted => StatusCode::CREATED.into_response(),
				RegisterSuccess::ImageExists(e) => (StatusCode::ACCEPTED,Json(e)).into_response(),
				RegisterSuccess::FileExists => StatusCode::ALREADY_REPORTED.into_response()
			}

		}
		Err(e) => (StatusCode::INTERNAL_SERVER_ERROR,Json(e)).into_response()
	}
}


async fn get_image_xml(Path(id):Path<Uuid>,Extension(db): Extension<Arc<Mutex<DB>>>) -> Response {
	todo!()
}

async fn get_image_thumbnail(Path(id):Path<Uuid>,Extension(db): Extension<Arc<Mutex<DB>>>) -> Response {
	todo!()
}
