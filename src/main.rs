use axum::{
    extract::Form,
    response::Html,
    routing::post,
    Router,
};
use serde::Deserialize;
use std::{env, net::SocketAddr};
use tiberius::{AuthMethod, Client, Config};
use tokio::net::TcpListener;
use tokio_util::compat::TokioAsyncWriteCompatExt;
use tower_http::services::ServeDir;

#[derive(Deserialize)]
struct CaptchaForm {
    #[serde(rename = "g-recaptcha-response")]
    token: String,
}

#[tokio::main]
async fn main() {
    // ───────────── ROUTER ─────────────
    let app = Router::new()
        .route("/submit", post(handle_submit))
        .nest_service("/", ServeDir::new("static"));

    // ───────────── RAILWAY PORT ─────────────
    let port: u16 = env::var("PORT")
        .unwrap_or_else(|_| "8080".to_string())
        .parse()
        .expect("PORT inválido");

    let addr = SocketAddr::from(([0, 0, 0, 0], port));

    println!("🚀 Servidor corriendo en http://0.0.0.0:{port}");

    // ───────────── AXUM 0.7 SERVER ─────────────
    let listener = TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

// ───────────── HANDLER ─────────────
async fn handle_submit(Form(data): Form<CaptchaForm>) -> Html<String> {
    match save_token(&data.token).await {
        Ok(_) => Html("<h1>Captcha guardado correctamente ✅</h1>".to_string()),
        Err(e) => Html(format!("<h1>Error: {}</h1>", e)),
    }
}

// ───────────── SQL SERVER (SOMEE) ─────────────
async fn save_token(token: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut config = Config::new();

    config.host("captcha_db.mssql.somee.com");
    config.port(1433);
    config.authentication(AuthMethod::sql_server(
        "CesarLT04_SQLLogin_1",
        "ecm87pr46l",
    ));
    config.database("captcha_db");
    config.trust_cert(); // IMPORTANTE para SOMEe

    let tcp = tokio::net::TcpStream::connect("captcha_db.mssql.somee.com:1433").await?;
    let mut client = Client::connect(config, tcp.compat_write()).await?;

    cl
