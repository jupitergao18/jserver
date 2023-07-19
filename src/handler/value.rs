use axum::{
    extract::{Json, State},
    http::{StatusCode, Uri},
};
use serde_json::Value;

use crate::handler::get_name;

use super::AppState;

pub async fn get_value(uri: Uri, State(app_state): State<AppState>) -> Json<Value> {
    let name = get_name(uri);
    app_state
        .db_value
        .read()
        .await
        .get(&name)
        .unwrap()
        .clone()
        .into()
}

pub async fn update_value(
    uri: Uri,
    State(app_state): State<AppState>,
    Json(value): Json<Value>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let name = get_name(uri);
    if value.is_array() || value.is_null() {
        return Err((
            StatusCode::BAD_REQUEST,
            "value must be object or plain value, not array nor null".to_string(),
        ));
    }
    if let Some(db_value) = app_state.db_value.write().await.as_object_mut() {
        let old_value = db_value.get(&name).unwrap();
        if (old_value.is_boolean() && !value.is_boolean())
            || (old_value.is_number() && !value.is_number())
            || (old_value.is_string() && !value.is_string())
            || (old_value.is_number() && !value.is_number())
            || (old_value.is_object() && !value.is_object())
        {
            return Err((StatusCode::BAD_REQUEST, "value type mismatch".to_string()));
        }
        let mut dirty = app_state.dirty.write().await;
        db_value.insert(name, value.clone());
        *dirty = true;
        drop(dirty);
        Ok(value.into())
    } else {
        Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            "unknown error".to_string(),
        ))
    }
}
