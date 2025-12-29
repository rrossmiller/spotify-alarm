use axum::response::Html;

pub const FRONTEND_HTML: &str = include_str!("frontend.html");

pub async fn serve_frontend() -> Html<&'static str> {
    Html(FRONTEND_HTML)
}
