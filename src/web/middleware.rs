use crate::auth::verify_password;
use crate::state::SharedState;
use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::Response,
};

pub async fn auth_middleware(
    State(state): State<SharedState>,
    req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    // Skip authentication for the root path (frontend HTML)
    if req.uri().path() == "/" {
        return Ok(next.run(req).await);
    }

    // Get password hash from config
    let password_hash = {
        let state_guard = state.read().await;
        match &state_guard.config.web.password_hash {
            Some(hash) => hash.clone(),
            None => {
                // No password configured - allow access (development mode)
                return Ok(next.run(req).await);
            }
        }
    };

    // Check for X-Password header
    let password = req
        .headers()
        .get("X-Password")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    // Verify password
    if verify_password(password, &password_hash) {
        Ok(next.run(req).await)
    } else {
        Err(StatusCode::UNAUTHORIZED)
    }
}
