use axum::{
    extract::{Path, State},
    http::{StatusCode, Uri},
    Json,
};
use serde_json::Value;

use super::{get_name, AppState};

pub async fn list(uri: Uri, State(app_state): State<AppState>) -> String {
    let name = get_name(uri);
    app_state
        .db_value
        .read()
        .await
        .get(&name)
        .unwrap()
        .to_string()
}

pub async fn get_item_by_id(
    uri: Uri,
    Path(id): Path<u64>,
    State(app_state): State<AppState>,
) -> Result<String, (StatusCode, String)> {
    let name = get_name(uri);
    match app_state
        .db_value
        .read()
        .await
        .get(&name)
        .unwrap()
        .as_array()
        .unwrap()
        .iter()
        .find(|item| item[&app_state.id] == id)
    {
        Some(item) => Ok(item.to_string()),
        None => Err((StatusCode::NOT_FOUND, "not found".to_string())),
    }
}

pub async fn post_item(
    uri: Uri,
    State(app_state): State<AppState>,
    Json(value): Json<Value>,
) -> Result<String, (StatusCode, String)> {
    let name = get_name(uri);
    if !value.is_object() {
        return Err((StatusCode::BAD_REQUEST, "value is not object".to_string()));
    }
    if let Some(id_value) = value.get(&app_state.id) {
        if !id_value.is_number() {
            return Err((
                StatusCode::BAD_REQUEST,
                "id must be an unsigned integer".to_string(),
            ));
        }
    }
    if let Some(db_value) = app_state.db_value.write().await.as_object_mut() {
        let old_value = db_value.get_mut(&name).unwrap();
        if !old_value.is_array() {
            return Err((StatusCode::BAD_REQUEST, "key is not array".to_string()));
        }
        let value = match value.get(&app_state.id) {
            Some(id) => {
                //check id
                let id_exists = old_value.as_array().unwrap().iter().any(|item| {
                    item.get(&app_state.id).unwrap().as_u64().unwrap() == id.as_u64().unwrap()
                });
                if id_exists {
                    return Err((StatusCode::BAD_REQUEST, "id exists".to_string()));
                }
                value
            }
            None => {
                //gen id
                let max_id = old_value
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|item| item.get(&app_state.id).unwrap().as_u64().unwrap())
                    .reduce(u64::max)
                    .unwrap();
                let mut value_clone = value.clone();
                let value_with_id = value_clone.as_object_mut().unwrap();
                value_with_id.insert(app_state.id.clone(), (max_id + 1).into());
                Value::Object(value_with_id.clone())
            }
        };
        let mut dirty = app_state.dirty.write().await;
        old_value.as_array_mut().unwrap().push(value.clone());
        *dirty = true;
        drop(dirty);
        Ok(value.to_string())
    } else {
        Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            "unknown error".to_string(),
        ))
    }
}

pub async fn update_item_by_id(
    uri: Uri,
    Path(id): Path<u64>,
    State(app_state): State<AppState>,
    Json(value): Json<Value>,
) -> Result<String, (StatusCode, String)> {
    let name = get_name(uri);
    if !value.is_object() {
        return Err((StatusCode::BAD_REQUEST, "value is not object".to_string()));
    }
    if let Some(db_value) = app_state.db_value.write().await.as_object_mut() {
        let old_value = db_value.get_mut(&name).unwrap();
        if !old_value.is_array() {
            return Err((StatusCode::BAD_REQUEST, "key is not array".to_string()));
        }
        let mut value_clone = value.clone();
        let value_replace_id = value_clone.as_object_mut().unwrap();
        value_replace_id.insert(app_state.id.clone(), id.into());
        let arr = old_value.as_array_mut().unwrap().iter_mut();
        for item in arr {
            if item[&app_state.id] == id {
                let mut dirty = app_state.dirty.write().await;
                *item = value_clone.clone();
                *dirty = true;
                drop(dirty);
            }
        }
        Ok(value_clone.to_string())
    } else {
        Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            "unknown error".to_string(),
        ))
    }
}

pub async fn delete_item_by_id(
    uri: Uri,
    Path(id): Path<u64>,
    State(app_state): State<AppState>,
) -> Result<String, (StatusCode, String)> {
    let name = get_name(uri);
    if let Some(db_value) = app_state.db_value.write().await.as_object_mut() {
        let old_value = db_value.get_mut(&name).unwrap();
        if !old_value.is_array() {
            return Err((StatusCode::BAD_REQUEST, "key is not array".to_string()));
        }
        match old_value
            .as_array()
            .unwrap()
            .iter()
            .position(|item| item[&app_state.id] == id)
        {
            Some(index) => {
                let mut dirty = app_state.dirty.write().await;
                let value = old_value.as_array_mut().unwrap().remove(index);
                *dirty = true;
                drop(dirty);
                Ok(value.to_string())
            }
            None => Err((StatusCode::NOT_FOUND, "not found".to_string())),
        }
    } else {
        Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            "unknown error".to_string(),
        ))
    }
}
