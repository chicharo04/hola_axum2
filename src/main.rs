use axum::{
    extract::Form,
    response::Html,
    routing::post,
    Router,
};
use serde::Deserialize;
use std::env;
use std::net::SocketAddr;
use tiberius::{Client, Config};
use tokio_util::compat::TokioAsyncWriteCompatExt;
use tower_http::services::ServeDir;

#[derive(Deserialize)]
struct CaptchaForm {
    #[serde(rename = "g-recaptcha-response")]
    token: String,
}

#[tokio::main]
async fn main() {
    let app = Router::new()
        .route("/submit", post(handle_submit))
        .nest_service("/", ServeDir::new("static"));

    // Railway usa esta variable
    let port: u16 = env::var("PORT")
        .unwrap_or_else(|_| "3000".to_string())
        .parse()
        .unwrap();

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    println!("Servidor corriendo en http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn handle_submit(Form(data): Form<CaptchaForm>) -> Html<String> {
    if let Err(e) = save_token(&data.token).await {
        return Html(format!("<h1>Error al guardar: {}</h1>", e));
    }

    Html("<h1>Captcha guardado en la base de datos ✅</h1>".to_string())
}

async fn save_token(token: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut config = Config::new();

    config.host("captcha_db.mssql.somee.com");
    config.port(1433);
    config.authentication(tiberius::AuthMethod::sql_server(
        "CesarLT04_SQLLogin_1",
        "ecm87pr46l",
    ));
    config.database("captcha_db");
    config.trust_cert(); // necesario para SOMEe

    let tcp = tokio::net::TcpStream::connect("captcha_db.mssql.somee.com:1433").await?;
    let mut client = Client::connect(config, tcp.compat_write()).await?;

    client
        .execute(
            "INSERT INTO captcha_logs (token) VALUES (@P1)",
            &[&token],
        )
        .await?;

    Ok(())
}
