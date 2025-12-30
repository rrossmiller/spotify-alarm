mod frontend;
mod middleware;
mod routes;

use crate::state::SharedState;
use axum::{
    routing::{get, post},
    Router,
};
use std::net::SocketAddr;

pub async fn run_server(
    state: SharedState,
    bind_addr: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let app = Router::new()
        // Frontend
        .route("/", get(frontend::serve_frontend))
        // API routes
        .route(
            "/api/alarms",
            get(routes::list_alarms).post(routes::create_alarm),
        )
        .route(
            "/api/alarms/:index",
            get(routes::get_alarm)
                .put(routes::update_alarm)
                .delete(routes::delete_alarm),
        )
        .route("/api/alarms/:index/toggle", post(routes::toggle_alarm))
        .route("/api/status", get(routes::get_status))
        // .route("/api/test-alarm", post(routes::test_alarm))
        // Add authentication middleware to all routes except the root
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            middleware::auth_middleware,
        ))
        .with_state(state);

    let addr: SocketAddr = bind_addr.parse()?;
    println!("🌐 Web interface available at http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
