use axum::{extract::{Multipart, State}, http::StatusCode, Json};
use serde::Serialize;

use crate::{auth::AuthUser, state::AppState};

const MAX_FILE: usize = 5 * 1024 * 1024;
const MAX_FILES: usize = 5;
const ALLOWED: &[(&str, &str)] = &[
    ("image/jpeg", "jpg"),
    ("image/png", "png"),
    ("image/webp", "webp"),
    ("image/gif", "gif"),
];

/// 上傳目錄：UPLOAD_DIR env，預設 ./uploads（test.sh 會指到 tmp 隔離）
pub(crate) fn upload_dir() -> String {
    std::env::var("UPLOAD_DIR").unwrap_or_else(|_| "uploads".to_string())
}

#[derive(Debug, Serialize)]
pub(crate) struct Uploaded {
    url: String,
}

// POST /uploads（multipart，需登入）：回 [{url}]，url 形如 /uploads/<uuid>.<ext>
pub(crate) async fn create(
    State(_s): State<AppState>,
    _auth: AuthUser,
    mut mp: Multipart,
) -> Result<Json<Vec<Uploaded>>, (StatusCode, String)> {
    tokio::fs::create_dir_all(upload_dir())
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let mut out = vec![];
    while let Some(field) = mp
        .next_field()
        .await
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?
    {
        if out.len() >= MAX_FILES {
            return Err((StatusCode::BAD_REQUEST, "too many files".into()));
        }
        let ctype = field.content_type().unwrap_or("").to_string();
        let ext = ALLOWED
            .iter()
            .find(|(m, _)| *m == ctype)
            .map(|(_, e)| *e)
            .ok_or((StatusCode::BAD_REQUEST, "bad file type".into()))?;
        let data = field
            .bytes()
            .await
            .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
        if data.len() > MAX_FILE {
            return Err((StatusCode::BAD_REQUEST, "file too large".into()));
        }
        let name = format!("{}.{}", uuid::Uuid::new_v4(), ext);
        tokio::fs::write(format!("{}/{}", upload_dir(), name), &data)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        out.push(Uploaded {
            url: format!("/uploads/{name}"),
        });
    }
    if out.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "no file".into()));
    }
    Ok(Json(out))
}
