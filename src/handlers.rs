use actix_multipart::{Field, Multipart};
use actix_web::{web, HttpResponse, Result};
use futures::TryStreamExt;
use log::{error, info};
use serde::Deserialize;
use std::collections::HashMap;

// Import the compression module
use crate::compression;
use crate::errors::ImageServerError;
use crate::config::Config;

#[derive(Debug, Deserialize)]
pub struct CompressionQuery {
    pub quality: Option<u8>,
    pub format: Option<String>,
    pub algorithm: Option<String>,
}

pub struct FileUpload {
    pub data: Vec<u8>,
    pub filename: Option<String>,
    pub content_type: Option<String>,
}

impl FileUpload {
    pub fn new() -> Self {
        Self {
            data: Vec::new(),
            filename: None,
            content_type: None,
        }
    }
}

pub async fn compress_endpoint(
    mut payload: Multipart,
    query: web::Query<CompressionQuery>,
    config: web::Data<Config>,
) -> Result<HttpResponse> {
    let mut file_upload: Option<FileUpload> = None;
    let mut form_params = HashMap::new();

    // Process multipart data
    while let Some(field) = payload.try_next().await? {
        let field_name = field.name().to_string();
        
        if field_name == "file" {
            file_upload = Some(process_file_field(field, config.max_file_size_bytes()).await?);
        } else {
            // Process other form fields (quality, algorithm, etc.)
            let value = process_text_field(field).await?;
            form_params.insert(field_name, value);
        }
    }

    let file_upload = match file_upload {
        Some(upload) => upload,
        None => {
            return Ok(HttpResponse::BadRequest().json(serde_json::json!({
                "error": "No file provided in 'file' field"
            })));
        }
    };

    // 根据目标格式确定输出格式
    let target_format = match query.format.as_deref() {
        Some(f) => f,
        None => if file_upload.filename.as_deref().unwrap_or("").to_lowercase().ends_with(".png") {
            "png"
        } else {
            "jpeg"
        }
    };
    
    // 设置质量
    let quality = query.quality
        .or_else(|| {
            form_params.get("quality")
                .and_then(|s| s.parse::<u8>().ok())
        })
        .unwrap_or(85)
        .clamp(1, 100);

    // 设置算法
    let algorithm = query.algorithm.clone()
        .or_else(|| form_params.get("algorithm").cloned())
        .unwrap_or_else(|| config.compression.default_algorithm.clone());

    info!(
        "Processing file: {} ({} bytes) with quality: {}, format: {} (保持原始尺寸), algorithm: {}",
        file_upload.filename.as_deref().unwrap_or("unknown"),
        file_upload.data.len(),
        quality,
        target_format,
        algorithm
    );
    
    // Perform compression
    match compression::compress_image(
        &file_upload.data, 
        target_format, 
        quality,
        &algorithm
    ) {
        Ok((compressed_data, width, height, exif_info)) => {
            let output_size = compressed_data.len();
            
            info!("Compression successful, size: {} bytes, dimensions: {}x{}, EXIF: {}", 
                  output_size, width, height, exif_info);
            
            let response = HttpResponse::Ok()
                .insert_header(("Content-Type", determine_output_content_type(target_format)))
                .insert_header(("Content-Length", output_size.to_string()))
                // Add compression statistics to response headers
                .insert_header(("X-Original-Size", file_upload.data.len().to_string()))
                .insert_header(("X-Compressed-Size", output_size.to_string()))
                .insert_header(("X-Image-Width", width.to_string()))
                .insert_header(("X-Image-Height", height.to_string()))
                .insert_header(("X-EXIF-Info", exif_info.clone()))
                .insert_header((
                    "Content-Disposition",
                    format!(
                        "attachment; filename=\"{}\"",
                        generate_output_filename(&file_upload.filename, target_format)
                    ),
                ))
                .body(compressed_data);

            info!(
                "Successfully compressed file: {} -> {} bytes ({}x{}), EXIF: {}",
                file_upload.data.len(), output_size, width, height, exif_info
            );

            Ok(response)
        }
        Err(err) => {
            error!("Compression failed: {}", err);
            Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                "error": format!("Compression failed: {}", err)
            })))
        }
    }
}

async fn process_file_field(mut field: Field, max_size_bytes: usize) -> Result<FileUpload> {
    let mut upload = FileUpload::new();
    
    // Get metadata
    upload.filename = field
        .content_disposition()
        .get_filename()
        .map(|s| s.to_string());
    
    upload.content_type = field
        .content_type()
        .map(|ct| ct.to_string());

    // Stream file data efficiently to handle large files
    let mut data = Vec::new();
    let mut total_size = 0;
    
    while let Some(chunk) = field.try_next().await? {
        total_size += chunk.len();
        
        // Use configured file size limit
        if total_size > max_size_bytes {
            return Err(ImageServerError::FileTooLarge { max_size: max_size_bytes }.into());
        }
        
        data.extend_from_slice(&chunk);
    }

    upload.data = data;
    Ok(upload)
}

async fn process_text_field(mut field: Field) -> Result<String> {
    let mut data = Vec::new();
    
    while let Some(chunk) = field.try_next().await? {
        data.extend_from_slice(&chunk);
        
        // Prevent excessively long text fields
        if data.len() > 1024 {
            return Err(ImageServerError::InvalidParameters(
                "Text field too long".to_string()
            ).into());
        }
    }
    
    String::from_utf8(data)
        .map_err(|_| ImageServerError::InvalidParameters(
            "Invalid UTF-8 in text field".to_string()
        ).into())
}

fn determine_output_content_type(format: &str) -> &'static str {
    match format.to_lowercase().as_str() {
        "jpeg" | "jpg" => "image/jpeg",
        "png" => "image/png",
        "webp" => "image/webp",
        _ => "application/octet-stream",
    }
}

fn generate_output_filename(
    original_filename: &Option<String>,
    format: &str,
) -> String {
    let base_name = original_filename
        .as_ref()
        .and_then(|name| {
            // Remove extension
            let stem = std::path::Path::new(name)
                .file_stem()
                .and_then(|s| s.to_str())?;
            Some(stem.to_string())
        })
        .unwrap_or_else(|| format!("compressed_{}", uuid::Uuid::new_v4()));

    let extension = match format.to_lowercase().as_str() {
        "jpeg" | "jpg" => "jpg",
        "png" => "png",
        "webp" => "webp",
        _ => "bin",
    };

    format!("{}_compressed.{}", base_name, extension)
}

pub async fn health_check() -> Result<HttpResponse> {
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "status": "ok",
        "service": "image-compression-server",
        "version": env!("CARGO_PKG_VERSION")
    })))
}

pub async fn info_endpoint(config: web::Data<Config>) -> Result<HttpResponse> {
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "service": "Image Compression Server",
        "version": env!("CARGO_PKG_VERSION"),
        "config": {
            "max_file_size_mb": config.server.max_file_size_mb,
            "default_quality": config.compression.default_quality,
            "default_algorithm": config.compression.default_algorithm,
        },
        "supported_algorithms": [
            {
                "name": "mozjpeg",
                "description": "Mozilla JPEG encoder (high quality)",
                "output_format": "image/jpeg"
            },
            {
                "name": "jpeg-encoder",
                "description": "Fast JPEG encoder",
                "output_format": "image/jpeg"
            },
            {
                "name": "png-quantized",
                "description": "PNG with color quantization",
                "output_format": "image/png"
            }
        ],
        "usage": {
            "endpoint": "/compress",
            "method": "POST",
            "content_type": "multipart/form-data",
            "parameters": {
                "file": "Image file to compress (required)",
                "quality": format!("Compression quality 1-100 (optional, default: {})", config.compression.default_quality),
                "algorithm": format!("Compression algorithm (optional, default: {})", config.compression.default_algorithm)
            },
            "query_parameters": {
                "quality": "Alternative way to specify quality",
                "algorithm": "Alternative way to specify algorithm"
            }
        }
    })))
}
