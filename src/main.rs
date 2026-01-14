use axum::{
    extract::Form,
    response::Html,
    routing::post,
    Router,
};
use serde::Deserialize;
use std::{env, net::SocketAddr};
use tiberius::{AuthMethod, Client, Config};
use tokio_util::compat::TokioAsyncWriteCompatExt;
use tower_http::services::ServeDir;

#[derive(Deserialize)]
struct CaptchaForm {
    #[serde(rename = "g-recaptcha-response")]
    token: String,
}

#[tokio::main]
async fn main() {
    // Rutas
    let app = Router::new()
        .route("/submit", post(handle_submit))
        .nest_service("/", ServeDir::new("static"));

    // Puerto asignado por Railway
    let port: u16 = env::var("PORT")
        .unwrap_or_else(|_| "3000".to_string())
        .parse()
        .expect("PORT debe ser un número");

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    println!("🚀 Servidor corriendo en http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("No se pudo bindear el puerto");

    axum::serve(listener, app)
        .await
        .expect("Error al iniciar el servidor");
}

async fn handle_submit(Form(data): Form<CaptchaForm>) -> Html<String> {
    match save_token(&data.token).await {
        Ok(_) => Html("<h1>Captcha guardado en la base de datos ✅</h1>".to_string()),
        Err(e) => Html(format!("<h1>Error al guardar: {}</h1>", e)),
    }
}

async fn save_token(token: &str) -> Result<(), Box<dyn std::error::Error>> {
    // Variables de entorno (Railway)
    let db_host = env::var("DB_HOST")?;
    let db_name = env::var("DB_NAME")?;
    let db_user = env::var("DB_USER")?;
    let db_pass = env::var("DB_PASS")?;

    let mut config = Config::new();
    config.host(&db_host);
    config.port(1433);
    config.database(&db_name);
    config.authentication(AuthMethod::sql_server(&db_user, &db_pass));
    config.trust_cert(); // NECESARIO para SOMEe

    let tcp = tokio::net::TcpStream::connect(format!("{}:1433", db_host)).await?;
    let mut client = Client::connect(config, tcp.compat_write()).await?;

    client
        .execute(
            "INSERT INTO captcha_logs (token) VALUES (@P1)",
            &[&token],
        )
        .await?;

    Ok(())
}
