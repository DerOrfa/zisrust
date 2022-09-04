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
use db::{DB, ImageInfo, RegisterSuccess};

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
		.route("/images/:uuid", get(get_image))
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

async fn get_images(Extension(db): Extension<Arc<Mutex<DB>>>) -> Result<Json<Vec<ImageInfo>>,StatusCode> {
	match db.lock().unwrap().query_images(None){
		Ok(images) => Ok(Json(images)),
		Err(e) => Err(StatusCode::INTERNAL_SERVER_ERROR)
	}
}

async fn get_image(Path(id):Path<Uuid>, Extension(db): Extension<Arc<Mutex<DB>>>) -> Result<Json<ImageInfo>,StatusCode> {
	let image = db.lock().unwrap().get_image(id)
		.or(Err(StatusCode::INTERNAL_SERVER_ERROR))?
		.ok_or(StatusCode::NOT_FOUND)?;
	Ok(Json(image))
}

#[derive(Deserialize)]
struct RegisterImagePayload{filename:PathBuf}
async fn register_image(Json(payload):Json<RegisterImagePayload>, Extension(db): Extension<Arc<Mutex<DB>>>) -> Result<Response,StatusCode> {
	if ! payload.filename.exists(){
		return Err(StatusCode::NOT_FOUND);
	}
	if ! payload.filename.is_file(){
		return Err(StatusCode::NOT_ACCEPTABLE)
	}
	// todo handle missing read access
	match db.lock().unwrap().register_file(&payload.filename){
		Ok(r) => {
			match r {
				RegisterSuccess::Inserted => Ok(StatusCode::CREATED.into_response()),
				RegisterSuccess::ImageExists(e) => Ok((StatusCode::ACCEPTED,Json(e)).into_response()),
				RegisterSuccess::FileExists => Ok(StatusCode::ALREADY_REPORTED.into_response())
			}

		}
		Err(e) => Err(StatusCode::INTERNAL_SERVER_ERROR)
	}
}


async fn get_image_xml(Path(id):Path<Uuid>,Extension(db): Extension<Arc<Mutex<DB>>>) -> Result<Response,StatusCode> {
	let xml=db.lock().unwrap()
		.get_image_xml(id).or(Err(StatusCode::INTERNAL_SERVER_ERROR))?
		.ok_or(StatusCode::NOT_FOUND)?;
	Ok((axum::TypedHeader(axum::headers::ContentType::xml()),xml).into_response())
}

async fn get_image_thumbnail(Path(id):Path<Uuid>,Extension(db): Extension<Arc<Mutex<DB>>>) -> Result<Response,StatusCode> {
	let image=db.lock().unwrap()
		.get_image_thumbnail(id).or(Err(StatusCode::INTERNAL_SERVER_ERROR))?
		.ok_or(StatusCode::NOT_FOUND)?;
	Ok((axum::TypedHeader(axum::headers::ContentType::jpeg()),image).into_response())
}
