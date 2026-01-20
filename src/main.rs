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
    // En Railway no hace daño, local sí ayuda
    dotenvy::dotenv().ok();

    let database_url = match env::var("DATABASE_URL") {
        Ok(v) => v,
        Err(_) => {
            eprintln!("❌ DATABASE_URL no encontrada");
            std::process::exit(1);
        }
    };

    let pool = match PgPool::connect(&database_url).await {
        Ok(p) => p,
        Err(e) => {
            eprintln!("❌ Error conectando a Postgres: {:?}", e);
            std::process::exit(1);
        }
    };

    let app = Router::new()
        .nest_service("/", ServeDir::new("static"))
        .route("/enviar", post(enviar))
        .with_state(pool)
        .layer(CorsLayer::permissive());

    let port = env::var("PORT")
        .unwrap_or_else(|_| "3000".to_string())
        .parse::<u16>()
        .unwrap_or(3000);

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("No se pudo bindear el puerto");

    println!("🚀 Servidor corriendo en {}", addr);

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

    if let Err(e) = sqlx::query(
        "INSERT INTO mensajes (nombre, mensaje) VALUES ($1, $2)",
    )
    .bind(&form.nombre)
    .bind(&form.mensaje)
    .execute(&pool)
    .await
    {
        eprintln!("❌ Error insertando mensaje: {:?}", e);
        return Html("❌ Error guardando el mensaje");
    }

    Html("✅ Mensaje enviado correctamente")
}

async fn verify_recaptcha(token: &str) -> bool {
    let secret = match env::var("RECAPTCHA_SECRET_KEY") {
        Ok(v) => v,
        Err(_) => {
            eprintln!("❌ RECAPTCHA_SECRET_KEY no definida");
            return false;
        }
    };

    let client = reqwest::Client::new();

    let res = client
        .post("https://www.google.com/recaptcha/api/siteverify")
        .form(&[
            ("secret", secret),
            ("response", token.to_string()),
        ])
        .send()
        .await;

    let resp = match res {
        Ok(r) => r,
        Err(e) => {
            eprintln!("❌ Error enviando request a reCAPTCHA: {:?}", e);
            return false;
        }
    };

    let json = match resp.json::<serde_json::Value>().await {
        Ok(j) => j,
        Err(e) => {
            eprintln!("❌ Error parseando respuesta reCAPTCHA: {:?}", e);
            return false;
        }
    };

    json["success"].as_bool().unwrap_or(false)
}
