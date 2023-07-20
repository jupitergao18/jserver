use axum::{
    extract::{Multipart, State},
    http::StatusCode,
    Json,
};
use serde::Serialize;
use tokio::io::AsyncWriteExt;

use crate::AppState;

#[derive(Debug, Serialize, Clone)]
pub struct FileInfo {
    pub name: String,
    pub path: String,
    pub size: usize,
}

pub async fn upload(
    State(app_state): State<AppState>,
    mut multipart: Multipart,
) -> Result<Json<Vec<FileInfo>>, (StatusCode, String)> {
    let public_path = &app_state.public_path;
    let mut result = Vec::<FileInfo>::new();
    while let Ok(opt_field) = multipart.next_field().await {
        if let Some(mut field) = opt_field {
            if let Some(field_name) = field.name() {
                if field_name != "file" {
                    continue;
                }
                let uuid = uuid::Uuid::new_v4().to_string();
                let name = field
                    .file_name()
                    .unwrap_or(uuid.clone().as_str())
                    .to_string();
                let ext_name = name.split('.').last().unwrap_or("").to_string();
                log::debug!("found file [{}.{}] = [{}]", uuid, ext_name, name);

                if let Ok(mut file) =
                    tokio::fs::File::create(format!("{}/{}.{}", public_path, uuid, ext_name)).await
                {
                    let mut size = 0usize;
                    while let Ok(chunk) = field.chunk().await {
                        if let Some(chunk) = chunk {
                            match file.write_all(&chunk).await {
                                Ok(_) => {
                                    log::debug!(
                                        "upload file [{}.{}] = [{}] ( {} bytes)",
                                        uuid,
                                        ext_name,
                                        name,
                                        chunk.len()
                                    );
                                    size += chunk.len();
                                }
                                Err(e) => {
                                    return Err((
                                        StatusCode::INTERNAL_SERVER_ERROR,
                                        format!("write file error: {}", e),
                                    ))
                                }
                            }
                        } else {
                            break;
                        }
                    }
                    result.push(FileInfo {
                        name,
                        path: format!("/{}.{}", uuid, ext_name),
                        size,
                    });
                } else {
                    return Err((
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "create file error".to_string(),
                    ));
                }
            }
        } else {
            break;
        }
    }
    Ok(result.into())
}
