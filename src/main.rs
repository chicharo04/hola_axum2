use axum::{
    extract::{Form, State, Multipart},
    routing::{get, post},
    Json, Router,
};
use axum::response::{Html, IntoResponse, Redirect};
use serde::Deserialize;
use sqlx::{PgPool, Row};
use std::{env, net::SocketAddr};
use tower_http::{cors::CorsLayer, services::ServeDir};
use tokio::io::AsyncWriteExt;
use uuid::Uuid;
use regex::Regex;

const MAX_IMAGE_SIZE: usize = 5 * 1024 * 1024; // 5MB
const ALLOWED_MIME: [&str; 4] = ["image/jpeg", "image/png", "image/webp", "image/jpg"];

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

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL no encontrada");
    let pool = PgPool::connect(&database_url)
        .await
        .expect("No se pudo conectar a la BD");

    let app = Router::new()
        .route("/enviar", post(enviar))
        .route("/upload-image", post(upload_image))
        .route("/images", get(list_images))
        .nest_service("/uploads", ServeDir::new("uploads"))
        .fallback_service(ServeDir::new("static"))
        .with_state(pool)
        .layer(CorsLayer::permissive());

    let port: u16 = env::var("PORT").unwrap_or("3000".into()).parse().unwrap();
    let addr = SocketAddr::from(([0, 0, 0, 0], port));

    println!("🚀 Servidor en {}", addr);

    axum::serve(tokio::net::TcpListener::bind(addr).await.unwrap(), app)
        .await
        .unwrap();
}

/* ---------------- MENSAJES ---------------- */

async fn enviar(
    State(pool): State<PgPool>,
    Form(mut data): Form<FormData>,
) -> impl IntoResponse {

    sanitize_text(&mut data.nombre);
    sanitize_text(&mut data.mensaje);

    let name_re = Regex::new(r"^[a-zA-ZáéíóúÁÉÍÓÚñÑ\s]{3,50}$").unwrap();

    if !name_re.is_match(&data.nombre) {
        return Html("❌ Nombre inválido").into_response();
    }

    if data.mensaje.len() < 10 || data.mensaje.len() > 500 {
        return Html("❌ Mensaje inválido").into_response();
    }

    if data.recaptcha.is_empty() {
        return Html("❌ Debes completar el reCAPTCHA").into_response();
    }

    let result = sqlx::query(
        "INSERT INTO mensajes (nombre, mensaje) VALUES ($1, $2)"
    )
    .bind(&data.nombre)
    .bind(&data.mensaje)
    .execute(&pool)
    .await;

    match result {
        Ok(_) => Redirect::to("/gracias.html").into_response(),
        Err(_) => Html("❌ Error guardando mensaje").into_response(),
    }
}

/* ---------------- SUBIR IMÁGENES ---------------- */

async fn upload_image(
    State(pool): State<PgPool>,
    mut multipart: Multipart,
) -> impl IntoResponse {

    tokio::fs::create_dir_all("uploads").await.unwrap();

    while let Some(field) = multipart.next_field().await.unwrap() {

        if field.name() != Some("image") {
            continue;
        }

        let content_type = field.content_type().unwrap_or("").to_string();

        if !ALLOWED_MIME.contains(&content_type.as_str()) {
            return Html("❌ Tipo de archivo no permitido").into_response();
        }

        let bytes = field.bytes().await.unwrap();

        if bytes.len() > MAX_IMAGE_SIZE {
            return Html("❌ Imagen demasiado grande (máx 5MB)").into_response();
        }

        let extension = match content_type.as_str() {
            "image/jpeg" | "image/jpg" => "jpg",
            "image/png" => "png",
            "image/webp" => "webp",
            _ => return Html("❌ Formato inválido").into_response(),
        };

        let filename = format!("{}.{}", Uuid::new_v4(), extension);
        let path = format!("uploads/{}", filename);

        let mut file = tokio::fs::File::create(&path).await.unwrap();
        file.write_all(&bytes).await.unwrap();

        sqlx::query("INSERT INTO images (filename) VALUES ($1)")
            .bind(&filename)
            .execute(&pool)
            .await
            .unwrap();
    }

    Redirect::to("/").into_response()
}

/* ---------------- LISTAR IMÁGENES ---------------- */

async fn list_images(
    State(pool): State<PgPool>,
) -> Json<Vec<String>> {

    let rows = sqlx::query("SELECT filename FROM images ORDER BY created_at DESC")
        .fetch_all(&pool)
        .await
        .unwrap();

    let images = rows
        .into_iter()
        .map(|row| format!("/uploads/{}", row.get::<String, _>("filename")))
        .collect();

    Json(images)
}

/* ---------------- UTIL ---------------- */

fn sanitize_text(text: &mut String) {
    let forbidden = ["<", ">", "\"", "'", ";", "--"];
    for f in forbidden {
        *text = text.replace(f, "");
    }
}
