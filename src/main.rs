use axum::{
    extract::{Form, State},
    response::{Html, IntoResponse},
    routing::{get, post},
    Router,
};
use serde::Deserialize;
use sqlx::{PgPool, Row};
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

    /* -------- DATABASE -------- */
    let database_url =
        env::var("DATABASE_URL").expect("DATABASE_URL no encontrada");
    let pool = PgPool::connect(&database_url)
        .await
        .expect("Error conectando a Postgres");

    /* -------- ROUTER -------- */
    let app = Router::new()
        .nest_service("/", ServeDir::new("static"))
        .nest_service("/uploads", ServeDir::new("uploads"))
        .route("/enviar", post(enviar))
        .route("/images", get(list_images))
        .with_state(pool)
        .layer(CorsLayer::permissive());

    /* -------- PORT (RAILWAY FIX) -------- */
    let port: u16 = env::var("PORT")
        .expect("PORT no encontrada")
        .parse()
        .expect("PORT inválido");

    let addr = SocketAddr::from(([0, 0, 0, 0], port));

    println!("Servidor escuchando en {}", addr);

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("Error al bindear el puerto");

    axum::serve(listener, app)
        .await
        .unwrap();
}

/* ---------------- MENSAJES ---------------- */

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

    let result = sqlx::query(
        "INSERT INTO messages (name, message) VALUES ($1, $2)",
    )
    .bind(&form.nombre)
    .bind(&form.mensaje)
    .execute(&pool)
    .await;

    match result {
        Ok(_) => Html("✅ Mensaje enviado correctamente"),
        Err(e) => {
            eprintln!("Error insertando mensaje: {:?}", e);
            Html("❌ Error guardando el mensaje")
        }
    }
}

/* ---------------- IMÁGENES (SOLO LECTURA) ---------------- */

async fn list_images(
    State(pool): State<PgPool>,
) -> impl IntoResponse {
    let rows = sqlx::query("SELECT filename FROM images ORDER BY created_at DESC")
        .fetch_all(&pool)
        .await
        .unwrap_or_default();

    let images: Vec<String> = rows
        .into_iter()
        .map(|row| {
            let filename: String = row.get("filename");
            format!("/uploads/{}", filename)
        })
        .collect();

    axum::Json(images)
}

/* ---------------- reCAPTCHA ---------------- */

async fn verify_recaptcha(token: &str) -> bool {
    let secret =
        env::var("RECAPTCHA_SECRET_KEY")
            .expect("RECAPTCHA_SECRET_KEY no encontrada");

    let res = reqwest::Client::new()
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
