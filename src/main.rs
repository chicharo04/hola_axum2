use axum::{
    extract::{Form, Multipart, State},
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

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL no encontrada");
    let pool = PgPool::connect(&database_url).await.unwrap();

    let app = Router::new()
        .nest_service("/", ServeDir::new("static"))
        .nest_service("/uploads", ServeDir::new("uploads"))
        .route("/enviar", post(enviar))
        .route("/upload-image", post(upload_image))
        .route("/images", get(list_images))
        .with_state(pool)
        .layer(CorsLayer::permissive());

    let port = env::var("PORT").unwrap_or("3000".into()).parse().unwrap();
    let addr = SocketAddr::from(([0, 0, 0, 0], port));

    println!("Servidor en {}", addr);
    axum::serve(tokio::net::TcpListener::bind(addr).await.unwrap(), app)
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

    let res = sqlx::query(
        "INSERT INTO messages (name, message) VALUES ($1, $2)",
    )
    .bind(&form.nombre)
    .bind(&form.mensaje)
    .execute(&pool)
    .await;

    match res {
        Ok(_) => Html("✅ Mensaje enviado correctamente"),
        Err(_) => Html("❌ Error guardando mensaje"),
    }
}

/* ---------------- IMÁGENES ---------------- */

async fn upload_image(
    State(pool): State<PgPool>,
    mut multipart: Multipart,
) -> Html<&'static str> {
    use tokio::io::AsyncWriteExt;

    tokio::fs::create_dir_all("uploads").await.unwrap();

    while let Some(field) = multipart.next_field().await.unwrap() {
        if field.name() == Some("image") {
            let filename = field.file_name().unwrap().to_string();
            let data = field.bytes().await.unwrap();

            let path = format!("uploads/{}", filename);
            let mut file = tokio::fs::File::create(&path).await.unwrap();
            file.write_all(&data).await.unwrap();

            sqlx::query("INSERT INTO images (filename) VALUES ($1)")
                .bind(&filename)
                .execute(&pool)
                .await
                .unwrap();
        }
    }

    Html("✅ Imagen subida correctamente")
}


async fn list_images(
    State(pool): State<PgPool>,
) -> impl IntoResponse {
    let rows = sqlx::query("SELECT filename FROM images ORDER BY created_at DESC")
        .fetch_all(&pool)
        .await
        .unwrap();

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
