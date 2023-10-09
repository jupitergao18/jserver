use axum::{
    extract::{DefaultBodyLimit, State},
    http::Uri,
    routing::{delete, get, patch, post, put},
    Json, Router,
};
use serde_json::Value;
use tower_http::{
    cors::{Any, CorsLayer},
    limit::RequestBodyLimitLayer,
    services::ServeDir,
};

use crate::{AppState, Args};

mod array;
mod upload;
mod value;

pub async fn build_router(app_state: AppState, args: Args) -> Router {
    use rayon::prelude::*;
    let mut api_routers = Router::new();
    let db_value = app_state.db_value.read().await;
    let id = &app_state.id;
    for (key, value) in db_value.as_object().expect("Invalid json object").iter() {
        if value.is_array() {
            let value_id_check = value.as_array().unwrap().par_iter().all(|item| {
                item.is_object() && item.get(id).is_some() && item.get(id).unwrap().is_u64()
            });
            if !value_id_check {
                log::error!("Array[{}] item object must have field [{}] and its value must be an unsigned integer", key, id);
                panic!();
            }
            api_routers = api_routers.route(&format!("/{}", key), get(array::list));
            api_routers = api_routers.route(&format!("/{}/:id", key), get(array::get_item_by_id));
            api_routers = api_routers.route(&format!("/{}", key), post(array::post_item));
            api_routers =
                api_routers.route(&format!("/{}/:id", key), put(array::update_item_by_id));
            api_routers =
                api_routers.route(&format!("/{}/:id", key), patch(array::update_item_by_id));
            api_routers =
                api_routers.route(&format!("/{}/:id", key), delete(array::delete_item_by_id));
        } else if !value.is_null() {
            api_routers = api_routers.route(&format!("/{}", key), get(value::get_value));
            api_routers = api_routers.route(&format!("/{}", key), post(value::update_value));
            api_routers = api_routers.route(&format!("/{}", key), put(value::update_value));
            api_routers = api_routers.route(&format!("/{}", key), patch(value::update_value));
        }
    }

    Router::new()
        .route("/db", get(db))
        .route("/upload", post(upload::upload))
        .nest("/api", api_routers)
        .fallback_service(ServeDir::new(args.public_path))
        .layer(
            CorsLayer::new()
                .allow_methods(Any)
                .allow_origin(Any)
                .allow_headers(Any),
        )
        .layer(DefaultBodyLimit::disable())
        .layer(RequestBodyLimitLayer::new(
            args.max_body_limit_m * 1024 * 1024,
        ))
        .with_state(app_state.clone())
}

async fn db(State(app_state): State<AppState>) -> Json<Value> {
    let db_value = app_state.db_value.read().await;
    db_value.clone().into()
}

pub fn get_name(uri: Uri) -> String {
    uri.path().split('/').nth(1).unwrap().to_string()
}
