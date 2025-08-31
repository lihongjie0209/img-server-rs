use thiserror::Error;

#[derive(Error, Debug)]
pub enum ImageServerError {
    #[error("Unsupported image format")]
    UnsupportedFormat,
    
    #[error("Image processing error: {0}")]
    ProcessingError(String),
    
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    
    #[error("Image decoding error: {0}")]
    ImageError(#[from] image::ImageError),
    
    #[error("Compression error: {0}")]
    CompressionError(String),
    
    #[error("Invalid parameters: {0}")]
    InvalidParameters(String),
    
    #[error("File too large: maximum size is {max_size} bytes")]
    FileTooLarge { max_size: usize },
}

impl actix_web::ResponseError for ImageServerError {
    fn error_response(&self) -> actix_web::HttpResponse {
        use actix_web::HttpResponse;
        
        match self {
            ImageServerError::UnsupportedFormat => {
                HttpResponse::UnsupportedMediaType().json(serde_json::json!({
                    "error": "unsupported_format",
                    "message": self.to_string()
                }))
            }
            ImageServerError::ProcessingError(_) => {
                HttpResponse::InternalServerError().json(serde_json::json!({
                    "error": "processing_error",
                    "message": self.to_string()
                }))
            }
            ImageServerError::InvalidParameters(_) => {
                HttpResponse::BadRequest().json(serde_json::json!({
                    "error": "invalid_parameters",
                    "message": self.to_string()
                }))
            }
            ImageServerError::FileTooLarge { max_size } => {
                HttpResponse::PayloadTooLarge().json(serde_json::json!({
                    "error": "file_too_large",
                    "message": self.to_string(),
                    "max_size_bytes": max_size
                }))
            }
            _ => {
                HttpResponse::InternalServerError().json(serde_json::json!({
                    "error": "internal_error",
                    "message": "An internal error occurred"
                }))
            }
        }
    }
}
