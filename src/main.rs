use axum::{
    routing::{get, post},
    response::Html,
    Form, Router,
};
use serde::Deserialize;
use sqlx::PgPool;
use std::net::SocketAddr;
use tokio::net::TcpListener;

#[derive(Deserialize)]
struct FormData {
    nombre: String,
    mensaje: String,
}

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    let database_url =
        std::env::var("DATABASE_URL").expect("DATABASE_URL no encontrada");

    let pool = PgPool::connect(&database_url)
        .await
        .expect("No se pudo conectar a PostgreSQL");

    let port: u16 = std::env::var("PORT")
        .unwrap_or("8080".to_string())
        .parse()
        .unwrap();

    let app = Router::new()
        .route("/", get(index))
        .route("/submit", post(submit))
        .with_state(pool);

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let listener = TcpListener::bind(addr).await.unwrap();

    axum::serve(listener, app).await.unwrap();
}

async fn index() -> Html<&'static str> {
    Html(
        r#"
        <h1>Axum + PostgreSQL en Railway</h1>
        <form method="post" action="/submit">
            <input name="nombre" placeholder="Nombre" required />
            <br><br>
            <textarea name="mensaje" placeholder="Mensaje" required></textarea>
            <br><br>
            <button>Enviar</button>
        </form>
        "#,
    )
}

async fn submit(
    axum::extract::State(pool): axum::extract::State<PgPool>,
    Form(data): Form<FormData>,
) -> Html<String> {
    sqlx::query(
        "INSERT INTO mensajes (nombre, mensaje) VALUES ($1, $2)",
    )
    .bind(&data.nombre)
    .bind(&data.mensaje)
    .execute(&pool)
    .await
    .unwrap();

    Html("Guardado en PostgreSQL ✅".to_string())
}
