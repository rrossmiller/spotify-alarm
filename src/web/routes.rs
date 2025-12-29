use crate::alarm::{Alarm, AlarmConfig};
use crate::state::SharedState;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::Serialize;

#[derive(Serialize)]
pub struct SystemStatus {
    alarm_count: usize,
    enabled_count: usize,
    last_trigger: Option<(String, String)>,
}

#[derive(Serialize)]
pub struct ErrorResponse {
    error: String,
}

/// GET /api/alarms - List all alarms
pub async fn list_alarms(State(state): State<SharedState>) -> Json<AlarmConfig> {
    let config = state.read().await.config.clone();
    Json(config)
}

/// GET /api/alarms/:index - Get specific alarm
pub async fn get_alarm(
    State(state): State<SharedState>,
    Path(index): Path<usize>,
) -> Result<Json<Alarm>, (StatusCode, Json<ErrorResponse>)> {
    let alarm = state.read().await.get_alarm(index);

    match alarm {
        Some(alarm) => Ok(Json(alarm)),
        None => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("Alarm at index {} not found", index),
            }),
        )),
    }
}

/// POST /api/alarms - Create new alarm
pub async fn create_alarm(
    State(state): State<SharedState>,
    Json(alarm): Json<Alarm>,
) -> Result<(StatusCode, Json<Alarm>), (StatusCode, Json<ErrorResponse>)> {
    // Validate alarm time format
    if let Err(e) = alarm.parse_time() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse { error: e }),
        ));
    }

    let mut state_guard = state.write().await;
    state_guard.add_alarm(alarm.clone());

    if let Err(e) = state_guard.save_config() {
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("Failed to save config: {}", e),
            }),
        ));
    }

    Ok((StatusCode::CREATED, Json(alarm)))
}

/// PUT /api/alarms/:index - Update existing alarm
pub async fn update_alarm(
    State(state): State<SharedState>,
    Path(index): Path<usize>,
    Json(alarm): Json<Alarm>,
) -> Result<Json<Alarm>, (StatusCode, Json<ErrorResponse>)> {
    // Validate alarm time format
    if let Err(e) = alarm.parse_time() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse { error: e }),
        ));
    }

    let mut state_guard = state.write().await;

    if let Err(e) = state_guard.update_alarm(index, alarm.clone()) {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse { error: e }),
        ));
    }

    if let Err(e) = state_guard.save_config() {
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("Failed to save config: {}", e),
            }),
        ));
    }

    Ok(Json(alarm))
}

/// DELETE /api/alarms/:index - Delete alarm
pub async fn delete_alarm(
    State(state): State<SharedState>,
    Path(index): Path<usize>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    let mut state_guard = state.write().await;

    if let Err(e) = state_guard.delete_alarm(index) {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse { error: e }),
        ));
    }

    if let Err(e) = state_guard.save_config() {
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("Failed to save config: {}", e),
            }),
        ));
    }

    Ok(StatusCode::NO_CONTENT)
}

/// POST /api/alarms/:index/toggle - Toggle alarm enabled state
pub async fn toggle_alarm(
    State(state): State<SharedState>,
    Path(index): Path<usize>,
) -> Result<Json<Alarm>, (StatusCode, Json<ErrorResponse>)> {
    let mut state_guard = state.write().await;

    let alarm = match state_guard.toggle_alarm(index) {
        Ok(alarm) => alarm,
        Err(e) => {
            return Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse { error: e }),
            ));
        }
    };

    if let Err(e) = state_guard.save_config() {
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("Failed to save config: {}", e),
            }),
        ));
    }

    Ok(Json(alarm))
}

/// GET /api/status - Get system status
pub async fn get_status(State(state): State<SharedState>) -> Json<SystemStatus> {
    let state_guard = state.read().await;
    let alarms = &state_guard.config.alarms;

    let status = SystemStatus {
        alarm_count: alarms.len(),
        enabled_count: alarms.iter().filter(|a| a.enabled).count(),
        last_trigger: state_guard.last_alarm_trigger.as_ref().map(|(name, time)| {
            (name.clone(), time.format("%Y-%m-%d %H:%M:%S").to_string())
        }),
    };

    Json(status)
}

/// POST /api/test-alarm - Trigger test playback
pub async fn test_alarm(State(state): State<SharedState>) -> StatusCode {
    let (session, spirc) = {
        let state_guard = state.read().await;
        (state_guard.session.clone(), state_guard.spirc.clone())
    };

    match crate::spotify::play(session, spirc).await {
        Ok(_) => StatusCode::OK,
        Err(e) => {
            eprintln!("Test alarm playback failed: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        }
    }
}
