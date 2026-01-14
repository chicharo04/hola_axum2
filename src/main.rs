use axum::{
    routing::{get, post},
    response::Html,
    Form, Router,
};
use serde::Deserialize;
use std::net::SocketAddr;
use tokio::net::TcpListener;

#[derive(Deserialize)]
struct CaptchaForm {
    nombre: String,
    mensaje: String,
}

#[tokio::main]
async fn main() {
    // 🚨 Railway usa la variable PORT
    let port: u16 = std::env::var("PORT")
        .unwrap_or("8080".to_string())
        .parse()
        .expect("PORT must be a number");

    let app = Router::new()
        .route("/", get(index))
        .route("/submit", post(handle_submit));

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    println!("Servidor corriendo en http://{}", addr);

    let listener = TcpListener::bind(addr)
        .await
        .expect("No se pudo bindear el puerto");

    axum::serve(listener, app)
        .await
        .expect("Error al iniciar el servidor");
}

async fn index() -> Html<&'static str> {
    Html(
        r#"
        <!DOCTYPE html>
        <html lang="es">
        <head>
            <meta charset="UTF-8">
            <title>Axum Railway OK</title>
        </head>
        <body>
            <h1>🚀 Axum funcionando en Railway</h1>

            <form method="post" action="/submit">
                <input type="text" name="nombre" placeholder="Nombre" required />
                <br><br>
                <textarea name="mensaje" placeholder="Mensaje" required></textarea>
                <br><br>
                <button type="submit">Enviar</button>
            </form>
        </body>
        </html>
        "#,
    )
}

async fn handle_submit(Form(data): Form<CaptchaForm>) -> Html<String> {
    Html(format!(
        "<h1>Formulario recibido ✅</h1><p>Nombre: {}</p><p>Mensaje: {}</p>",
        data.nombre, data.mensaje
    ))
}
