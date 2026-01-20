use axum::{
    extract::{Form, State, Multipart},
    response::{Html, IntoResponse},
    routing::post,
    Router,
};
use std::fs;
use uuid::Uuid;
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
    .nest_service("/uploads", ServeDir::new("uploads")) // 👈 NUEVO
    .route("/enviar", post(enviar))
    .route("/upload-image", post(upload_image)) // 👈 NUEVO
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


async fn verify_recaptcha(token: &str) -> bool {
    let secret = env::var("RECAPTCHA_SECRET_KEY")
        .expect("RECAPTCHA_SECRET_KEY no encontrada");

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
async fn upload_image(
    mut multipart: Multipart,
) -> impl IntoResponse {
    while let Ok(Some(field)) = multipart.next_field().await {
        if let Some(name) = field.name() {
            if name == "image" {
                let data = field.bytes().await.unwrap();

                fs::create_dir_all("uploads").ok();

                let filename = format!("{}.jpg", Uuid::new_v4());
                let path = format!("uploads/{}", filename);

                fs::write(&path, data).unwrap();

                return Html(format!(
                    "<img src=\"/uploads/{}\" width=\"200\">",
                    filename
                ));
            }
        }
    }

    Html("❌ Error subiendo imagen")
}


