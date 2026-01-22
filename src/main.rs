use axum::{
    extract::{Form, State},
    response::Redirect,
    routing::{get, post},
    Json, Router,
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

    let database_url =
        env::var("DATABASE_URL").expect("DATABASE_URL no encontrada");
    let pool = PgPool::connect(&database_url)
        .await
        .expect("No se pudo conectar a la BD");

    let app = Router::new()
        .route("/enviar", post(enviar))
        .route("/images", get(list_images))
        .nest_service("/uploads", ServeDir::new("uploads"))
        .fallback_service(ServeDir::new("static"))
        .with_state(pool)
        .layer(CorsLayer::permissive());

    let port: u16 = env::var("PORT")
        .unwrap_or("3000".into())
        .parse()
        .unwrap();

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    println!("🚀 Servidor en {}", addr);

    axum::serve(
        tokio::net::TcpListener::bind(addr).await.unwrap(),
        app,
    )
    .await
    .unwrap();
}

/* ---------------- MENSAJES ---------------- */

async fn enviar(
    State(pool): State<PgPool>,
    Form(form): Form<FormData>,
) -> impl IntoResponse {
    let nombre = form.nombre.trim();
    let mensaje = form.mensaje.trim();

    /* VALIDACIONES */

    if nombre.is_empty() {
        return Html("❌ El nombre es obligatorio".into());
    }

    if nombre.len() < 3 || nombre.len() > 50 {
        return Html("❌ El nombre debe tener entre 3 y 50 caracteres".into());
    }

    if mensaje.is_empty() {
        return Html("❌ El mensaje es obligatorio".into());
    }

    if mensaje.len() < 10 || mensaje.len() > 500 {
        return Html("❌ El mensaje debe tener entre 10 y 500 caracteres".into());
    }

    if form.recaptcha.is_empty() {
        return Html("❌ Debes completar el reCAPTCHA".into());
    }

    if !verify_recaptcha(&form.recaptcha).await {
        return Html("❌ reCAPTCHA inválido".into());
    }

    /* INSERT */

    let res = sqlx::query(
        "INSERT INTO messages (name, message) VALUES ($1, $2)",
    )
    .bind(nombre)
    .bind(mensaje)
    .execute(&pool)
    .await;

    match res {
    Ok(_) => Redirect::to("/gracias.html").into_response(),
    Err(_) => Html("❌ Error guardando mensaje").into_response(),
}

}

/* ---------------- IMÁGENES ---------------- */

async fn list_images(
    State(pool): State<PgPool>,
) -> Json<Vec<String>> {
    let rows = sqlx::query(
        "SELECT filename FROM images ORDER BY created_at DESC",
    )
    .fetch_all(&pool)
    .await
    .unwrap();

    let images = rows
        .into_iter()
        .map(|row| {
            let filename: String = row.get("filename");
            format!("/uploads/{}", filename)
        })
        .collect();

    Json(images)
}

/* ---------------- reCAPTCHA ---------------- */

async fn verify_recaptcha(token: &str) -> bool {
    let secret = env::var("RECAPTCHA_SECRET_KEY").unwrap();

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
