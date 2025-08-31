// Simple test module to check basic module functionality

#[derive(Debug, Clone)]
pub enum CompressionAlgorithm {
    MozJpeg,
    JpegEncoder,
    PngQuantized,
}

#[derive(Debug, Clone)]
pub struct CompressionOptions {
    pub quality: u8,
    pub algorithm: CompressionAlgorithm,
}

pub fn compress_image(_data: &[u8], _options: &CompressionOptions) -> Result<Vec<u8>, String> {
    Ok(vec![1, 2, 3, 4]) // dummy implementation
}

pub fn test_compression_module() -> &'static str {
    "simple module is working"
}
