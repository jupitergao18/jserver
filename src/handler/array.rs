use std::cmp::Ordering;

use axum::{
    extract::{Path, Query, State},
    http::{StatusCode, Uri},
    Json,
};
use serde::Deserialize;
use serde_json::{json, Value};

use super::{get_name, AppState};

pub async fn list(
    uri: Uri,
    paginate: Option<Query<Paginate>>,
    sort: Option<Query<Sort>>,
    slice: Option<Query<Slice>>,
    State(app_state): State<AppState>,
) -> Result<String, (StatusCode, String)> {
    let name = get_name(uri);
    let db_value = app_state.db_value.read().await;
    let values = db_value.get(&name).unwrap();
    if !values.is_array() {
        return Err((StatusCode::BAD_REQUEST, "key is not array".to_string()));
    }
    let (sorts, orders) = if let Some(sort) = sort {
        let mut sorts = sort
            .sort
            .split(',')
            .map(|i| i.to_string())
            .collect::<Vec<String>>();
        let mut orders = sort
            .order
            .split(',')
            .map(|i| i.to_string())
            .collect::<Vec<String>>();
        if sorts.len() != orders.len() {
            return Err((
                StatusCode::BAD_REQUEST,
                "sort and order length not match".to_string(),
            ));
        }
        sorts.reverse();
        orders.reverse();
        (sorts, orders)
    } else {
        (Vec::new(), Vec::new())
    };

    if paginate.is_some() && slice.is_some() {
        return Err((
            StatusCode::BAD_REQUEST,
            "paginate and slice can not use together".to_string(),
        ));
    }

    if let Some(slice) = slice.clone() {
        if slice.end.is_none() && slice.limit.is_none() {
            return Err((
                StatusCode::BAD_REQUEST,
                "slice end and limit can not both be none".to_string(),
            ));
        }
        if let Some(end) = slice.end {
            if slice.start > end {
                return Err((
                    StatusCode::BAD_REQUEST,
                    "slice start must less than end".to_string(),
                ));
            }
        }
        if let Some(limit) = slice.limit {
            if limit == 0 {
                return Err((
                    StatusCode::BAD_REQUEST,
                    "slice limit can not be zero".to_string(),
                ));
            }
        }
    }

    if let Some(paginate) = paginate.clone() {
        if paginate.size == 0 {
            return Err((
                StatusCode::BAD_REQUEST,
                "paginate size can not be zero".to_string(),
            ));
        }
    }

    let mut values_clone = values.clone();
    let values = values_clone.as_array_mut().unwrap();

    //1、filter
    //2、sort
    for (sort, order) in sorts.iter().zip(orders.iter()) {
        values.sort_by(|a, b| {
            let a = a.get(sort).unwrap();
            let b = b.get(sort).unwrap();
            if a.is_number() && b.is_number() {
                let a = a.as_f64().unwrap();
                let b = b.as_f64().unwrap();
                if order == "asc" {
                    a.partial_cmp(&b).unwrap()
                } else {
                    b.partial_cmp(&a).unwrap()
                }
            } else if a.is_string() && b.is_string() {
                let a = a.as_str().unwrap();
                let b = b.as_str().unwrap();
                if order == "asc" {
                    a.partial_cmp(b).unwrap()
                } else {
                    b.partial_cmp(a).unwrap()
                }
            } else {
                Ordering::Equal
            }
        });
    }
    //3、page or slice
    let (start, end) = if let Some(paginate) = paginate {
        (
            paginate.page * paginate.size,
            paginate.page * paginate.size + paginate.size,
        )
    } else if let Some(slice) = slice {
        if let Some(end) = slice.end {
            (slice.start, end)
        } else {
            (slice.start, slice.start + slice.limit.unwrap())
        }
    } else {
        (0, 20)
    };

    if start >= values.len() {
        return Ok(json!(Vec::<Value>::new()).to_string());
    }
    if end > values.len() {
        return Ok(json!(values[start..].to_vec()).to_string());
    }

    Ok(json!(values[start..end].to_vec()).to_string())
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

#[derive(Deserialize, Clone)]
pub struct Paginate {
    #[serde(rename = "_page")]
    pub page: usize,
    #[serde(rename = "_size")]
    pub size: usize,
}

#[derive(Deserialize)]
pub struct Sort {
    #[serde(rename = "_sort")]
    pub sort: String,
    #[serde(rename = "_order")]
    pub order: String,
}

#[derive(Deserialize, Clone)]
pub struct Slice {
    #[serde(rename = "_start")]
    pub start: usize,
    #[serde(rename = "_end")]
    pub end: Option<usize>,
    #[serde(rename = "_limit")]
    pub limit: Option<usize>,
}
