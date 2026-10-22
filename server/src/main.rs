use axum::{
    extract::{DefaultBodyLimit, Multipart},
    http::StatusCode,
    response::{IntoResponse, Json, Response},
    routing::{get, post},
    Router,
};

use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use core_tools::forensics::analysis::analyze_block_file;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::net::SocketAddr;
use tower_http::cors::{Any, CorsLayer};

use core_tools::visualiser::analysis::tx_report::analyze_fixture;
use core_tools::{psbt::builder, visualiser::analysis::block_report::analyze_block_files};

#[derive(Debug, Deserialize)]
struct AnalyzeRequest {
    /// Base64-encoded fixture JSON for tx mode, OR raw JSON object
    #[serde(default)]
    fixture: Option<String>,
    /// Alternative: raw fixture JSON object
    #[serde(default)]
    fixture_json: Option<Value>,
    /// For block mode: base64-encoded blk*.dat bytes
    #[serde(default)]
    blk_data: Option<String>,
    /// For block mode: base64-encoded rev*.dat bytes
    #[serde(default)]
    rev_data: Option<String>,
    /// For block mode: base64-encoded xor.dat bytes
    #[serde(default)]
    xor_data: Option<String>,
}

fn error_response(code: &str, message: &str) -> (StatusCode, Json<Value>) {
    (
        StatusCode::BAD_REQUEST,
        Json(json!({
            "ok": false,
            "error": {
                "code": code,
                "message": message
            }
        })),
    )
}

async fn health() -> Json<Value> {
    Json(json!({ "ok": true }))
}

async fn analyze(Json(req): Json<AnalyzeRequest>) -> Response {
    // Block mode
    if req.blk_data.is_some() || req.rev_data.is_some() {
        let blk_b64 = match &req.blk_data {
            Some(s) => s,
            None => {
                return error_response("INVALID_ARGS", "blk_data is required for block mode")
                    .into_response()
            }
        };
        let rev_b64 = match &req.rev_data {
            Some(s) => s,
            None => {
                return error_response("INVALID_ARGS", "rev_data is required for block mode")
                    .into_response()
            }
        };

        let blk_bytes = match BASE64.decode(blk_b64) {
            Ok(b) => b,
            Err(e) => {
                return error_response("INVALID_ARGS", &format!("Invalid blk_data base64: {}", e))
                    .into_response()
            }
        };
        let rev_bytes = match BASE64.decode(rev_b64) {
            Ok(b) => b,
            Err(e) => {
                return error_response("INVALID_ARGS", &format!("Invalid rev_data base64: {}", e))
                    .into_response()
            }
        };

        let xor_bytes = if let Some(xor_b64) = &req.xor_data {
            match BASE64.decode(xor_b64) {
                Ok(b) => b,
                Err(e) => {
                    return error_response(
                        "INVALID_ARGS",
                        &format!("Invalid xor_data base64: {}", e),
                    )
                    .into_response()
                }
            }
        } else {
            vec![0u8; 8]
        };

        let mut xor_key = [0u8; 8];
        let copy_len = xor_bytes.len().min(8);
        xor_key[..copy_len].copy_from_slice(&xor_bytes[..copy_len]);

        match analyze_block_files(&blk_bytes, &rev_bytes, &xor_key) {
            Ok(reports) => {
                return Json(json!({ "ok": true, "blocks": reports })).into_response();
            }
            Err(e) => {
                return error_response(e.code.as_str(), &e.message).into_response();
            }
        }
    }

    // Transaction mode
    let fixture_json_str = if let Some(raw) = &req.fixture {
        // Could be base64 or direct JSON string
        raw.clone()
    } else if let Some(obj) = &req.fixture_json {
        match serde_json::to_string(obj) {
            Ok(s) => s,
            Err(e) => {
                return error_response("INVALID_ARGS", &format!("Invalid fixture_json: {}", e))
                    .into_response()
            }
        }
    } else {
        return error_response(
            "INVALID_ARGS",
            "Either 'fixture' or 'fixture_json' is required",
        )
        .into_response();
    };

    match analyze_fixture(&fixture_json_str) {
        Ok(report) => Json(json!(report)).into_response(),
        Err(e) => error_response(e.code.as_str(), &e.message).into_response(),
    }
}

/// Multipart endpoint for uploading block files
async fn analyze_multipart(mut multipart: Multipart) -> Response {
    let mut blk_bytes: Option<Vec<u8>> = None;
    let mut rev_bytes: Option<Vec<u8>> = None;
    let mut xor_bytes: Option<Vec<u8>> = None;
    let mut fixture_json_str: Option<String> = None;

    while let Ok(Some(field)) = multipart.next_field().await {
        let name = field.name().unwrap_or("").to_string();
        let data = match field.bytes().await {
            Ok(d) => d.to_vec(),
            Err(e) => {
                return error_response(
                    "INVALID_ARGS",
                    &format!("Error reading field {}: {}", name, e),
                )
                .into_response()
            }
        };

        match name.as_str() {
            "blk" => blk_bytes = Some(data),
            "rev" => rev_bytes = Some(data),
            "xor" => xor_bytes = Some(data),
            "fixture" => {
                fixture_json_str = Some(String::from_utf8_lossy(&data).to_string());
            }
            _ => {}
        }
    }

    // Block mode
    if blk_bytes.is_some() || rev_bytes.is_some() {
        let blk = match blk_bytes {
            Some(b) => b,
            None => {
                return error_response("INVALID_ARGS", "blk file is required for block mode")
                    .into_response()
            }
        };
        let rev = match rev_bytes {
            Some(b) => b,
            None => {
                return error_response("INVALID_ARGS", "rev file is required for block mode")
                    .into_response()
            }
        };
        let xor_raw = xor_bytes.unwrap_or_else(|| vec![0u8; 8]);

        let mut xor_key = [0u8; 8];
        let copy_len = xor_raw.len().min(8);
        xor_key[..copy_len].copy_from_slice(&xor_raw[..copy_len]);

        return match analyze_block_files(&blk, &rev, &xor_key) {
            Ok(reports) => Json(json!({ "ok": true, "blocks": reports })).into_response(),
            Err(e) => error_response(e.code.as_str(), &e.message).into_response(),
        };
    }

    // Tx mode
    let fixture_str = match fixture_json_str {
        Some(s) => s,
        None => return error_response("INVALID_ARGS", "fixture field is required").into_response(),
    };

    match analyze_fixture(&fixture_str) {
        Ok(report) => Json(json!(report)).into_response(),
        Err(e) => error_response(e.code.as_str(), &e.message).into_response(),
    }
}

async fn build_handler(Json(body): Json<Value>) -> impl IntoResponse {
    let json_str = serde_json::to_string(&body).unwrap();

    match builder::build_from_fixture(&json_str) {
        Ok(report) => {
            let json_val = serde_json::to_value(&report).unwrap();
            (StatusCode::OK, Json(json_val))
        }
        Err(err) => {
            let error_report = err.to_error_report();
            let json_val = serde_json::to_value(&error_report).unwrap();
            (StatusCode::BAD_REQUEST, Json(json_val))
        }
    }
}

async fn forensics(mut multipart: Multipart) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let mut blk_data: Option<Vec<u8>> = None;
    let mut rev_data: Option<Vec<u8>> = None;
    let mut xor_data: Option<Vec<u8>> = None;
    let mut blk_filename = String::from("upload.dat");

    while let Some(field) = multipart.next_field().await.map_err(|e| {
        make_error(
            "MULTIPART_ERROR",
            &format!("Failed to read multipart field: {}", e),
        )
    })? {
        let name = field.name().unwrap_or("").to_string();
        let file_name = field.file_name().unwrap_or("").to_string();
        let data = field
            .bytes()
            .await
            .map_err(|e| make_error("READ_ERROR", &format!("Failed to read field bytes: {}", e)))?;

        match name.as_str() {
            "blk" => {
                if !file_name.is_empty() {
                    blk_filename = file_name;
                }
                blk_data = Some(data.to_vec());
            }
            "rev" => {
                rev_data = Some(data.to_vec());
            }
            "xor" => {
                xor_data = Some(data.to_vec());
            }
            _ => {}
        }
    }

    let blk = blk_data.ok_or_else(|| make_error("MISSING_FILE", "blk.dat file is required"))?;
    let rev = rev_data.ok_or_else(|| make_error("MISSING_FILE", "rev.dat file is required"))?;
    let xor = xor_data.ok_or_else(|| make_error("MISSING_FILE", "xor.dat file is required"))?;

    // Parse XOR key (8 bytes)
    let mut xor_key = [0u8; 8];
    if xor.len() >= 8 {
        xor_key.copy_from_slice(&xor[..8]);
    } else {
        xor_key[..xor.len()].copy_from_slice(&xor);
    }

    // Run the analysis (potentially slow, so we spawn a blocking task)
    let filename = blk_filename.clone();
    let result =
        tokio::task::spawn_blocking(move || analyze_block_file(&blk, &rev, &xor_key, &filename))
            .await
            .map_err(|e| make_error("INTERNAL_ERROR", &format!("Task panicked: {}", e)))?;

    match result {
        Ok(output) => {
            let json_value = serde_json::to_value(&output).map_err(|e| {
                make_error("SERIALIZE_ERROR", &format!("Failed to serialize: {}", e))
            })?;
            Ok(Json(json_value))
        }
        Err(e) => Err(make_error(&e.code, &e.message)),
    }
}

fn make_error(code: &str, message: &str) -> (StatusCode, Json<Value>) {
    (
        StatusCode::BAD_REQUEST,
        Json(json!({
            "ok": false,
            "error": {
                "code": code,
                "message": message
            }
        })),
    )
}

#[tokio::main]
async fn main() {
    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(3000);

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any)
        .expose_headers(Any)
        .max_age(std::time::Duration::from_secs(86400));

    let app = Router::new()
        .route("/", get(health))
        .route("/health", get(health))
        .route("/api/health", get(health))
        .route(
            "/api/analyze",
            post(analyze).layer(DefaultBodyLimit::max(512 * 1024 * 1024)),
        )
        .route(
            "/api/analyze/upload",
            post(analyze_multipart).layer(DefaultBodyLimit::max(512 * 1024 * 1024)),
        )
        .route(
            "/api/build",
            post(build_handler).layer(DefaultBodyLimit::max(512 * 1024 * 1024)),
        )
        .route(
            "/api/forensics",
            post(forensics).layer(DefaultBodyLimit::max(512 * 1024 * 1024)),
        )
        .layer(cors);

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    println!("Listening on http://0.0.0.0:{}", port);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
