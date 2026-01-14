use axum::{
    extract::Form,
    response::Html,
    routing::post,
    Router,
};
use serde::Deserialize;
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

    let addr = "127.0.0.1:3000";
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
    config.host("localhost");
    config.port(1433);
    config.authentication(tiberius::AuthMethod::sql_server("sa", "1234"));
    config.database("captcha_db");
    config.trust_cert();

    let tcp = tokio::net::TcpStream::connect("localhost:1433").await?;
    let mut client = Client::connect(config, tcp.compat_write()).await?;

    client
        .execute(
            "INSERT INTO captcha_logs (token) VALUES (@P1)",
            &[&token],
        )
        .await?;

    Ok(())
}
