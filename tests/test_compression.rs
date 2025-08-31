use std::io::Cursor;
use log::{info, warn, debug};

#[derive(Debug, Clone)]
pub enum ImageType {
    PNG,
    JPEG,
}

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

#[derive(Debug, Clone)]
pub struct CompressionStats {
    pub original_size: usize,
    pub compressed_size: usize,
    pub compression_ratio: f64,
    pub processing_time_ms: u128,
    pub image_width: u32,
    pub image_height: u32,
    pub algorithm_used: String,
}

impl CompressionStats {
    pub fn new(
        original_size: usize,
        compressed_size: usize,
        processing_time_ms: u128,
        width: u32,
        height: u32,
        algorithm: &CompressionAlgorithm,
    ) -> Self {
        // 计算真正的压缩率：节省的空间百分比
        let compression_ratio = if original_size > 0 {
            ((original_size - compressed_size) as f64 / original_size as f64) * 100.0
        } else {
            0.0
        };

        Self {
            original_size,
            compressed_size,
            compression_ratio,
            processing_time_ms,
            image_width: width,
            image_height: height,
            algorithm_used: match algorithm {
                CompressionAlgorithm::MozJpeg => "mozjpeg".to_string(),
                CompressionAlgorithm::JpegEncoder => "jpeg-encoder".to_string(),
                CompressionAlgorithm::PngQuantized => "png-quantized".to_string(),
            },
        }
    }
}

/// Main compression function
pub fn compress_image(data: &[u8], options: &CompressionOptions) -> Result<(Vec<u8>, CompressionStats), String> {
    let start_time = std::time::Instant::now();
    let original_size = data.len();

    let (compressed_data, width, height) = match options.algorithm {
        CompressionAlgorithm::MozJpeg => {
            let (data, w, h) = do_mozjpeg_compression(data, options.quality)?;
            (data, w, h)
        }
        CompressionAlgorithm::JpegEncoder => {
            let image_type = detect_image_type(data)?;
            match image_type {
                ImageType::PNG => do_jpeg_encoder_compression(data, options.quality)?,
                ImageType::JPEG => do_jpeg_encoder_compression(data, options.quality)?,
            }
        }
        CompressionAlgorithm::PngQuantized => {
            let image_type = detect_image_type(data)?;
            match image_type {
                ImageType::PNG => do_png_compression(data, options.quality)?,
                ImageType::JPEG => do_png_compression(data, options.quality)?,
            }
        }
    };

    let processing_time = start_time.elapsed().as_millis();
    let stats = CompressionStats::new(
        original_size,
        compressed_data.len(),
        processing_time,
        width,
        height,
        &options.algorithm,
    );

    info!(
        "Compressed image: {}x{}, {:.2}% compression ratio, {}ms",
        width, height, stats.compression_ratio, processing_time
    );

    Ok((compressed_data, stats))
}

pub fn detect_image_type(data: &[u8]) -> Result<ImageType, String> {
    if data.len() < 8 {
        return Err("Data too short to determine image type".to_string());
    }

    // Check PNG signature
    if data.starts_with(&[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]) {
        return Ok(ImageType::PNG);
    }

    // Check JPEG signature
    if data.starts_with(&[0xFF, 0xD8, 0xFF]) {
        return Ok(ImageType::JPEG);
    }

    Err("Unsupported image format".to_string())
}

/// Extract EXIF data from JPEG and log orientation info
fn extract_exif_data(data: &[u8]) -> Option<Vec<u8>> {
    let exif_reader = exif::Reader::new();
    let mut cursor = std::io::Cursor::new(data);
    
    match exif_reader.read_from_container(&mut cursor) {
        Ok(exif) => {
            info!("Found EXIF data with {} fields", exif.fields().count());
            
            // 检查方向信息
            if let Some(orientation_field) = exif.get_field(exif::Tag::Orientation, exif::In::PRIMARY) {
                if let exif::Value::Short(ref vec) = orientation_field.value {
                    if !vec.is_empty() {
                        let orientation = vec[0];
                        info!("EXIF Orientation: {}", orientation);
                        
                        match orientation {
                            1 => info!("  → 正常方向 (0°)"),
                            3 => warn!("  → 需要旋转180°"),
                            6 => warn!("  → 需要顺时针旋转90°"),
                            8 => warn!("  → 需要逆时针旋转90°"),
                            _ => warn!("  → 其他方向: {}", orientation),
                        }
                        
                        if orientation != 1 {
                            warn!("图片包含方向信息，压缩后可能显示方向异常");
                        }
                    }
                }
            } else {
                info!("No orientation info found in EXIF");
            }
            
            None // 暂时不返回EXIF数据
        }
        Err(e) => {
            debug!("No EXIF data found: {}", e);
            None
        }
    }
}

/// MozJPEG compression with EXIF awareness
pub fn do_mozjpeg_compression(data: &[u8], quality: u8) -> Result<(Vec<u8>, u32, u32), String> {
    info!("Starting MozJPEG compression with quality: {}", quality);
    
    // 读取并记录EXIF信息
    let _original_exif = extract_exif_data(data);
    
    // 为什么不直接使用mozjpeg？
    // 1. 理论上可以，但mozjpeg crate的API设计主要面向从RGB数据压缩
    // 2. 直接JPEG到JPEG需要处理各种颜色空间、采样因子等复杂情况  
    // 3. image crate提供了统一的接口处理多种格式
    
    // 当前方案：使用image crate但注意EXIF问题
    let img = image::load_from_memory(data)
        .map_err(|e| format!("Failed to load image: {}", e))?;
        
    info!("Image loaded through image crate, dimensions: {}x{}", img.width(), img.height());
    info!("注意：image crate可能已应用EXIF旋转，这会改变像素布局");
    
    let rgb_img = img.to_rgb8();
    let width = rgb_img.width();
    let height = rgb_img.height();
    let rgb_data = rgb_img.as_raw();
    info!("Converted to RGB8, dimensions: {}x{}, raw data size: {} bytes", width, height, rgb_data.len());

    // Create MozJPEG compressor
    let mut comp = mozjpeg::Compress::new(mozjpeg::ColorSpace::JCS_RGB);
    comp.set_size(width as usize, height as usize);
    comp.set_quality(quality as f32);
    comp.set_mem_dest();
    info!("MozJPEG compressor configured");
    
    comp.start_compress();
    info!("Started compression");

    // Write scanlines row by row
    let row_stride = (width * 3) as usize;
    info!("Writing {} scanlines with stride {}", height, row_stride);
    
    for (i, row) in rgb_data.chunks(row_stride).enumerate() {
        if i % 100 == 0 {
            debug!("Writing scanline {}/{}", i, height);
        }
        comp.write_scanlines(row);
    }
    info!("All scanlines written");

    // Finish compression and get JPEG bytes
    info!("Finishing compression and extracting data");
    let jpeg_data = comp.data_to_vec().map_err(|e| format!("MozJPEG compression error: {:?}", e))?;
    info!("MozJPEG compression completed, output size: {} bytes", jpeg_data.len());
    
    Ok((jpeg_data, width, height))
}

/// PNG compression using imagequant
pub fn do_png_compression(data: &[u8], quality: u8) -> Result<(Vec<u8>, u32, u32), String> {
    // Load image data
    let img = image::load_from_memory(data)
        .map_err(|e| format!("Failed to load PNG image: {}", e))?;

    // Convert to RGBA8 format for imagequant
    let rgba_img = img.to_rgba8();
    let width = rgba_img.width();
    let height = rgba_img.height();
    let image_data = rgba_img.as_raw();

    // Memory optimization info
    info!(
        "PNG compression: {}x{}, using {} optimization",
        width,
        height,
        if can_use_zero_copy() {
            "zero-copy"
        } else {
            "memory-copy"
        }
    );

    // Create imagequant attributes
    let mut liq = imagequant::new();
    liq.set_quality(0, quality).map_err(|e| format!("Failed to set quality: {:?}", e))?;

    // Create image - use zero-copy if safe
    let mut img = if can_use_zero_copy() {
        unsafe {
            let rgba_slice = bytes_to_rgba_slice(image_data);
            liq.new_image(rgba_slice, width as usize, height as usize, 0.0)
        }
    } else {
        // Fallback: copy data
        let rgba_pixels: Vec<imagequant::RGBA> = image_data
            .chunks_exact(4)
            .map(|chunk| imagequant::RGBA::new(chunk[0], chunk[1], chunk[2], chunk[3]))
            .collect();
        liq.new_image_borrowed(&rgba_pixels, width as usize, height as usize, 0.0)
    }
    .map_err(|e| format!("Failed to create imagequant image: {:?}", e))?;

    // Quantize
    let mut res = match liq.quantize(&mut img) {
        Ok(res) => res,
        Err(imagequant::Error::QualityTooLow) => {
            warn!("Quality too low, retrying with higher minimum quality");
            liq.set_quality(10, quality).map_err(|e| format!("Failed to set fallback quality: {:?}", e))?;
            liq.quantize(&mut img).map_err(|e| format!("Failed to quantize with fallback: {:?}", e))?
        }
        Err(e) => return Err(format!("Failed to quantize: {:?}", e)),
    };

    // Write PNG
    let (palette, pixels) = res.remapped(&mut img).map_err(|e| format!("Failed to remap: {:?}", e))?;

    let mut png_data = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut png_data, width, height);
        encoder.set_color(png::ColorType::Indexed);
        encoder.set_depth(png::BitDepth::Eight);
        
        // Convert RGBA palette to RGB bytes for PNG encoder
        let palette_bytes: Vec<u8> = palette.into_iter()
            .flat_map(|rgba| [rgba.r, rgba.g, rgba.b])
            .collect();
        encoder.set_palette(palette_bytes);

        let mut writer = encoder.write_header().map_err(|e| format!("Failed to write PNG header: {:?}", e))?;
        writer.write_image_data(&pixels).map_err(|e| format!("Failed to write PNG data: {:?}", e))?;
    }

    Ok((png_data, width, height))
}

/// JPEG compression using fast jpeg-encoder
pub fn do_jpeg_encoder_compression(data: &[u8], quality: u8) -> Result<(Vec<u8>, u32, u32), String> {
    // Load image data
    let img = image::load_from_memory(data)
        .map_err(|e| format!("Failed to load image: {}", e))?;

    // Convert to RGB format
    let rgb_img = img.to_rgb8();
    let width = rgb_img.width() as u16;
    let height = rgb_img.height() as u16;
    let raw_data = rgb_img.as_raw();

    // Pre-allocate output buffer
    let mut jpeg_data = Vec::with_capacity(data.len() / 2);

    // Use jpeg-encoder for fast compression
    {
        let encoder = jpeg_encoder::Encoder::new(&mut jpeg_data, quality);
        encoder
            .encode(raw_data, width, height, jpeg_encoder::ColorType::Rgb)
            .map_err(|e| format!("Failed to encode JPEG with fast encoder: {}", e))?;
    }

    Ok((jpeg_data, width as u32, height as u32))
}

/// Check if zero-copy conversion is safe for imagequant::RGBA
pub fn can_use_zero_copy() -> bool {
    std::mem::size_of::<imagequant::RGBA>() == 4
        && std::mem::align_of::<imagequant::RGBA>() == 1
}

/// Convert &[u8] to &[imagequant::RGBA] safely
pub unsafe fn bytes_to_rgba_slice(bytes: &[u8]) -> &[imagequant::RGBA] {
    std::slice::from_raw_parts(
        bytes.as_ptr() as *const imagequant::RGBA,
        bytes.len() / 4,
    )
}
