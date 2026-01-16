use axum::{
    extract::{Form, State},
    response::Html,
    routing::post,
    Router,
};
use serde::Deserialize;
use sqlx::PgPool;
use std::{env, net::SocketAddr};
use tower_http::{cors::CorsLayer, services::ServeDir};

#[derive(Deserialize)]
struct FormData {
    nombre: String,
    mensaje: String,
    #[serde(rename = "g-recaptcha-response")]
    recaptcha: String,
}

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    let database_url =
        env::var("DATABASE_URL").expect("DATABASE_URL no encontrada");

    let pool = PgPool::connect(&database_url)
        .await
        .expect("Error conectando a Postgres");

    let app = Router::new()
        .nest_service("/", ServeDir::new("static"))
        .route("/enviar", post(enviar))
        .with_state(pool)
        .layer(CorsLayer::permissive());

    let port = env::var("PORT")
        .unwrap_or_else(|_| "3000".to_string())
        .parse::<u16>()
        .unwrap();

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();

    println!("Servidor corriendo en {}", addr);

    axum::serve(listener, app).await.unwrap();
}

async fn enviar(
    State(pool): State<PgPool>,
    Form(form): Form<FormData>,
) -> Html<&'static str> {
    if form.recaptcha.is_empty() {
        return Html("❌ Debes completar el reCAPTCHA");
    }

    if !verify_recaptcha(&form.recaptcha).await {
        return Html("❌ reCAPTCHA inválido");
    }

    let _ = sqlx::query(
        "INSERT INTO mensajes (nombre, mensaje) VALUES ($1, $2)",
    )
    .bind(&form.nombre)
    .bind(&form.mensaje)
    .execute(&pool)
    .await;

    Html("✅ Mensaje enviado correctamente")
}

async fn verify_recaptcha(token: &str) -> bool {
    let secret = env::var("RECAPTCHA_SECRET")
        .expect("RECAPTCHA_SECRET no encontrada");

    let client = reqwest::Client::new();
    let res = client
        .post("https://www.google.com/recaptcha/api/siteverify")
        .form(&[
            ("secret", secret),
            ("response", token.to_string()),
        ])
        .send()
        .await;

    if let Ok(resp) = res {
        if let Ok(json) = resp.json::<serde_json::Value>().await {
            return json["success"].as_bool().unwrap_or(false);
        }
    }

    false
}
