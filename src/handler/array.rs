use std::{cmp::Ordering, collections::HashMap};

use axum::{
    extract::{Path, Query, State},
    http::{StatusCode, Uri},
    response::{IntoResponse, Response},
    Json,
};
use rayon::prelude::*;
use serde::Deserialize;
use serde_json::{json, Value};

use super::{get_name, AppState};

const DEFAULT_PAGE_SIZE: usize = 20;

pub async fn list(
    uri: Uri,
    paginate: Option<Query<Paginate>>,
    sort: Option<Query<Sort>>,
    slice: Option<Query<Slice>>,
    Query(params): Query<HashMap<String, String>>,
    State(app_state): State<AppState>,
) -> impl IntoResponse {
    let name = get_name(uri);
    let db_value = app_state.db_value.read().await;
    let values = db_value.get(&name).expect("key not found");
    if !values.is_array() {
        return Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .body::<String>("key is not array".into())
            .expect("failed to render response");
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
            return Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .body("sort and order length not match".into())
                .expect("failed to render response");
        }
        sorts.reverse();
        orders.reverse();
        (sorts, orders)
    } else {
        (Vec::new(), Vec::new())
    };

    if paginate.is_some() && slice.is_some() {
        return Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .body("paginate and slice can not use together".into())
            .expect("failed to render response");
    }

    if let Some(slice) = slice.clone() {
        if let Some(end) = slice.end {
            if slice.start > end {
                return Response::builder()
                    .status(StatusCode::BAD_REQUEST)
                    .body("slice start must less than end".into())
                    .expect("failed to render response");
            }
        }
        if let Some(limit) = slice.limit {
            if limit == 0 {
                return Response::builder()
                    .status(StatusCode::BAD_REQUEST)
                    .body("slice limit can not be zero".into())
                    .expect("failed to render response");
            }
        }
    }

    let mut values_clone = values.clone();
    let values = values_clone.as_array_mut().unwrap();

    //1、filter
    let filters = params
        .par_iter()
        .filter(|(k, _)| !k.starts_with('_'))
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect::<Vec<(String, String)>>();
    values.retain(|item| {
        for (k, v) in filters.iter() {
            if k.ends_with("_lte") && v.parse::<f64>().is_ok() {
                let k = k.trim_end_matches("_lte");
                if let Some(value) = item.get(k) {
                    if value.is_number() && value.as_f64().unwrap() > v.parse::<f64>().unwrap() {
                        return false;
                    }
                } else {
                    return false;
                }
            } else if k.ends_with("_gte") && v.parse::<f64>().is_ok() {
                let k = k.trim_end_matches("_gte");
                if let Some(value) = item.get(k) {
                    if value.is_number() && value.as_f64().unwrap() < v.parse::<f64>().unwrap() {
                        return false;
                    }
                } else {
                    return false;
                }
            } else if k.ends_with("_lt") && v.parse::<f64>().is_ok() {
                let k = k.trim_end_matches("_lt");
                if let Some(value) = item.get(k) {
                    if value.is_number() && value.as_f64().unwrap() >= v.parse::<f64>().unwrap() {
                        return false;
                    }
                } else {
                    return false;
                }
            } else if k.ends_with("_gt") && v.parse::<f64>().is_ok() {
                let k = k.trim_end_matches("_gt");
                if let Some(value) = item.get(k) {
                    if value.is_number() && value.as_f64().unwrap() <= v.parse::<f64>().unwrap() {
                        return false;
                    }
                } else {
                    return false;
                }
            } else if k.ends_with("_ne") {
                let k = k.trim_end_matches("_ne");
                if let Some(value) = item.get(k) {
                    if (value.is_string() && value.as_str().unwrap() == v)
                        || (value.is_number()
                            && v.parse::<f64>().is_ok()
                            && value.as_f64().unwrap() == v.parse::<f64>().unwrap())
                        || (value.is_boolean()
                            && value.as_bool().unwrap() == v.parse::<bool>().unwrap())
                    {
                        return false;
                    }
                } else {
                    return false;
                }
            } else if k.ends_with("_like") {
                let k = k.trim_end_matches("_like");
                if let Some(value) = item.get(k) {
                    if !value.is_string() || !value.as_str().unwrap().contains(v) {
                        return false;
                    }
                } else {
                    return false;
                }
            } else if k.ends_with("_nlike") {
                let k = k.trim_end_matches("_nlike");
                if let Some(value) = item.get(k) {
                    if !value.is_string() || value.as_str().unwrap().contains(v) {
                        return false;
                    }
                } else {
                    return false;
                }
            } else if k.ends_with("_contains") {
                let k = k.trim_end_matches("_contains");
                if let Some(value) = item.get(k) {
                    if !value.is_array()
                        || !value
                            .as_array()
                            .unwrap()
                            .contains(&Value::String(v.to_string()))
                    {
                        return false;
                    }
                } else {
                    return false;
                }
            } else if k.ends_with("_ncontains") {
                let k = k.trim_end_matches("_ncontains");
                if let Some(value) = item.get(k) {
                    if !value.is_array()
                        || value
                            .as_array()
                            .unwrap()
                            .contains(&Value::String(v.to_string()))
                    {
                        return false;
                    }
                } else {
                    return false;
                }
            } else if k.ends_with("_exists") {
                let k = k.trim_end_matches("_exists");
                if item.get(k).is_none() {
                    return false;
                }
            } else if k.ends_with("_nexists") {
                let k = k.trim_end_matches("_nexists");
                if item.get(k).is_some() {
                    return false;
                }
            } else if let Some(value) = item.get(k) {
                if (value.is_string() && value.as_str().unwrap() != v)
                    || (value.is_number()
                        && v.parse::<f64>().is_ok()
                        && value.as_f64().unwrap() != v.parse::<f64>().unwrap())
                    || (value.is_boolean()
                        && v.parse::<bool>().is_ok()
                        && value.as_bool().unwrap() != v.parse::<bool>().unwrap())
                {
                    return false;
                }
            } else {
                return false;
            }
        }
        true
    });
    //2、sort
    for (sort, order) in sorts.iter().zip(orders.iter()) {
        values.sort_by(|a, b| {
            let a = a.get(sort);
            let b = b.get(sort);
            if a.is_none() && b.is_none() {
                return Ordering::Equal;
            } else if a.is_none() && b.is_some() {
                if order == "asc" {
                    return Ordering::Less;
                } else {
                    return Ordering::Greater;
                }
            } else if a.is_some() && b.is_none() {
                if order == "asc" {
                    return Ordering::Greater;
                } else {
                    return Ordering::Less;
                }
            }
            let a = a.unwrap();
            let b = b.unwrap();
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
            } else if a.is_boolean() && b.is_boolean() {
                let a = a.as_bool().unwrap();
                let b = b.as_bool().unwrap();
                if order == "asc" {
                    a.cmp(&b)
                } else {
                    b.cmp(&a)
                }
            } else {
                Ordering::Equal
            }
        });
    }
    //3、page or slice
    let (start, end) = if let Some(paginate) = paginate {
        (
            (if paginate.page.unwrap_or(1) > 0 {
                paginate.page.unwrap_or(1) - 1
            } else {
                0
            }) * paginate.size.unwrap_or(DEFAULT_PAGE_SIZE),
            (if paginate.page.unwrap_or(1) > 0 {
                paginate.page.unwrap_or(1) - 1
            } else {
                0
            }) * paginate.size.unwrap_or(DEFAULT_PAGE_SIZE)
                + paginate.size.unwrap_or(DEFAULT_PAGE_SIZE),
        )
    } else if let Some(slice) = slice {
        if let Some(end) = slice.end {
            (slice.start, end)
        } else {
            (
                slice.start,
                slice.start + slice.limit.unwrap_or(DEFAULT_PAGE_SIZE),
            )
        }
    } else {
        (0, DEFAULT_PAGE_SIZE)
    };

    let body = if start >= values.len() {
        json!(Vec::<Value>::new()).to_string()
    } else if end > values.len() {
        json!(values[start..].to_vec()).to_string()
    } else {
        json!(values[start..end].to_vec()).to_string()
    };

    Response::builder()
        .status(StatusCode::OK)
        .header("X-Total-Count", values.len().to_string())
        .header("Content-Type", "application/json")
        .body(body)
        .expect("failed to render response")
}

pub async fn get_item_by_id(
    uri: Uri,
    Path(id): Path<u64>,
    State(app_state): State<AppState>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let name = get_name(uri);
    match app_state
        .db_value
        .read()
        .await
        .get(&name)
        .unwrap()
        .as_array()
        .unwrap()
        .par_iter()
        .find_any(|item| item[&app_state.id] == id)
    {
        Some(item) => Ok(item.clone().into()),
        None => Err((StatusCode::NOT_FOUND, "not found".to_string())),
    }
}

pub async fn post_item(
    uri: Uri,
    State(app_state): State<AppState>,
    Json(value): Json<Value>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let name = get_name(uri);
    if !value.is_object() {
        return Err((StatusCode::BAD_REQUEST, "value is not object".to_string()));
    }
    if let Some(id_value) = value.get(&app_state.id) {
        if !id_value.is_number() || id_value.as_u64().is_none() {
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
                let id_exists = old_value.as_array().unwrap().par_iter().any(|item| {
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
                    .par_iter()
                    .map(|item| item.get(&app_state.id).unwrap().as_u64().unwrap())
                    .max()
                    .unwrap_or(0);
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
        Ok(value.into())
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
) -> Result<Json<Value>, (StatusCode, String)> {
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
                break;
            }
        }
        Ok(value_clone.into())
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
) -> Result<Json<Value>, (StatusCode, String)> {
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
                Ok(value.into())
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
    pub page: Option<usize>,
    #[serde(rename = "_size")]
    pub size: Option<usize>,
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
