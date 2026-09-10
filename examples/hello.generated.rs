use axum::{ serve, Router, routing::get, response::{ Html, IntoResponse}, extract::{ Query, Path}};
use tokio::net::TcpListener;
use serde::Deserialize;
#[derive(Debug, Deserialize)]
struct HelloParams {
    name: Option<String>,
}
#[tokio::main]
async fn main() {
    let routes_all = Router::new().merge(routes_hello());
    let listener = TcpListener::bind("127.0.0.1:8089").await.unwrap();
    println!("Listening on {} ...", listener.local_addr().unwrap());
    serve(listener, routes_all).await.unwrap()
}
fn routes_hello() -> Router {
    Router::new().route("/hello", get(handler_hello)).route("/hello2/:name", get(handler_hello2))
}
async fn handler_hello(Query(params): Query<HelloParams>) -> impl IntoResponse {
    println!("> {:<8} handler_hello - {params:?}", "HANDLER");
    let name = params.name.as_deref().unwrap_or("World");
    Html(format!("Hello, <strong>{name}</strong>"))
}
async fn handler_hello2(Path(name): Path<String>) -> impl IntoResponse {
    println!("> {:<8} handler_hello2 - {name:?}", "HANDLER");
    Html(format!("Hello 2, {name}"))
}
