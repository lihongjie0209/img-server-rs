use std::io::Cursor;
use log::info;
use image::{ImageBuffer, Rgb, Rgba, ImageFormat, DynamicImage};
use exif::{In, Tag, Reader as ExifReader};

// 压缩图片的主要函数
pub fn compress_image(
    data: &[u8],
    max_width: u32,
    max_height: u32,
    format: &str,
    quality: u8
) -> Result<(Vec<u8>, u32, u32, String), String> {
    info!("开始压缩图片 - 目标格式: {}, 质量: {}, 最大尺寸: {}x{}", 
         format, quality, max_width, max_height);
    
    // 如果目标格式是 JPEG，移除 EXIF 信息以避免自动旋转
    let (processed_data, exif_info) = if format.to_lowercase() == "jpeg" || format.to_lowercase() == "jpg" {
        info!("检测到JPEG格式，开始移除EXIF信息");
        match remove_exif_data(data) {
            Ok(result) => {
                info!("EXIF信息移除成功");
                result
            },
            Err(e) => {
                info!("移除EXIF信息失败: {}, 使用原始数据", e);
                (data.to_vec(), "EXIF removal failed".to_string())
            }
        }
    } else {
        (data.to_vec(), "No EXIF processing for non-JPEG".to_string())
    };
    
    info!("EXIF处理结果: {}", exif_info);
    
    // 加载图片（无EXIF的数据）
    let img = image::load_from_memory(&processed_data)
        .map_err(|e| format!("Failed to load image: {}", e))?;
    
    let (original_width, original_height) = (img.width(), img.height());
    info!("原始图片尺寸: {}x{}", original_width, original_height);
    
    // 计算目标尺寸
    let (target_width, target_height) = calculate_target_size(
        original_width, original_height, max_width, max_height
    );
    info!("目标图片尺寸: {}x{}", target_width, target_height);
    
    // 调整图片尺寸
    let resized_img = if target_width != original_width || target_height != original_height {
        img.resize(target_width, target_height, image::imageops::FilterType::Lanczos3)
    } else {
        img
    };
    
    // 根据格式进行压缩
    let compressed_data = match format.to_lowercase().as_str() {
        "jpeg" | "jpg" => {
            info!("使用 mozjpeg 进行 JPEG 压缩");
            do_mozjpeg_compression(resized_img, quality)?
        },
        "png" => {
            info!("进行 PNG 压缩");
            let result = do_png_compression(&resized_img.to_rgba8().into_raw(), target_width, target_height)?;
            result.0
        },
        "webp" => {
            info!("进行 WebP 压缩");
            do_webp_compression(&resized_img.to_rgba8().into_raw(), target_width, target_height, quality)?
        },
        _ => return Err(format!("Unsupported format: {}", format))
    };
    
    let final_size = compressed_data.len();
    info!("压缩完成 - 最终大小: {} bytes", final_size);
    
    Ok((compressed_data, target_width, target_height, exif_info))
}

// 移除 EXIF 数据的函数
fn remove_exif_data(data: &[u8]) -> Result<(Vec<u8>, String), String> {
    // 首先检查并提取 EXIF 信息
    let exif_info = match extract_exif_data(data) {
        Ok(info) => info,
        Err(e) => format!("EXIF extraction failed: {}", e)
    };
    
    // 使用 image::io::Reader 直接解码，不保留 EXIF
    let reader = image::io::Reader::new(Cursor::new(data))
        .with_guessed_format()
        .map_err(|e| format!("Failed to create image reader: {}", e))?;
        
    let img = reader.decode()
        .map_err(|e| format!("Failed to decode image: {}", e))?;
    
    // 重新编码为 JPEG，不包含 EXIF
    let mut output = Vec::new();
    img.write_to(&mut Cursor::new(&mut output), ImageFormat::Jpeg)
        .map_err(|e| format!("Failed to re-encode image: {}", e))?;
    
    Ok((output, exif_info))
}

// 提取 EXIF 数据的函数
fn extract_exif_data(data: &[u8]) -> Result<String, String> {
    let mut cursor = Cursor::new(data);
    let exif_reader = ExifReader::new();
    
    match exif_reader.read_from_container(&mut cursor) {
        Ok(exif) => {
            let mut info = String::new();
            
            // 检查方向信息
            if let Some(field) = exif.get_field(Tag::Orientation, In::PRIMARY) {
                info.push_str(&format!("Orientation: {}", field.display_value().to_string()));
            } else {
                info.push_str("No orientation tag found");
            }
            
            // 添加其他有用的 EXIF 信息
            if let Some(field) = exif.get_field(Tag::Make, In::PRIMARY) {
                info.push_str(&format!(", Make: {}", field.display_value().to_string()));
            }
            
            if let Some(field) = exif.get_field(Tag::Model, In::PRIMARY) {
                info.push_str(&format!(", Model: {}", field.display_value().to_string()));
            }
            
            Ok(info)
        },
        Err(_) => Ok("No EXIF data found".to_string())
    }
}

// 计算目标尺寸
fn calculate_target_size(
    original_width: u32,
    original_height: u32,
    max_width: u32,
    max_height: u32
) -> (u32, u32) {
    if original_width <= max_width && original_height <= max_height {
        return (original_width, original_height);
    }
    
    let width_ratio = max_width as f64 / original_width as f64;
    let height_ratio = max_height as f64 / original_height as f64;
    let scale = width_ratio.min(height_ratio);
    
    let target_width = (original_width as f64 * scale) as u32;
    let target_height = (original_height as f64 * scale) as u32;
    
    (target_width, target_height)
}

// mozjpeg 压缩函数
fn do_mozjpeg_compression(img: DynamicImage, quality: u8) -> Result<Vec<u8>, String> {
    info!("开始 mozjpeg 压缩");
    
    // 转换为 RGB
    let rgb_img = img.to_rgb8();
    let (width, height) = (rgb_img.width(), rgb_img.height());
    let raw_data = rgb_img.into_raw();
    
    info!("图片信息 - 宽: {}, 高: {}, 数据长度: {}", width, height, raw_data.len());
    
    // 使用 mozjpeg 的压缩接口
    match mozjpeg::compress::compress_to_vec(&raw_data, width as usize, height as usize, quality) {
        Ok(compressed) => {
            info!("mozjpeg 压缩成功，输出大小: {} bytes", compressed.len());
            Ok(compressed)
        },
        Err(e) => {
            let error_msg = format!("mozjpeg compression failed: {:?}", e);
            info!("{}", error_msg);
            
            // 如果 mozjpeg 失败，回退到标准 JPEG 编码
            info!("回退到标准 JPEG 编码");
            do_standard_jpeg_compression(&raw_data, width, height, quality)
        }
    }
}

// 标准 JPEG 压缩（作为回退）
fn do_standard_jpeg_compression(data: &[u8], width: u32, height: u32, quality: u8) -> Result<Vec<u8>, String> {
    use jpeg_encoder::{Encoder, ColorType};
    
    let mut encoder = Encoder::new(quality);
    encoder.encode(data, width as u16, height as u16, ColorType::Rgb)
        .map_err(|e| format!("Standard JPEG encoding failed: {:?}", e))
}

// PNG 压缩函数
pub fn do_png_compression(rgba_data: &[u8], width: u32, height: u32) -> Result<(Vec<u8>, u32, u32), String> {
    // 简化的 PNG 压缩，直接使用 PNG 编码器
    let mut output = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut output, width, height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        encoder.set_compression(png::Compression::Best);
        
        let mut writer = encoder.write_header()
            .map_err(|e| format!("PNG encoder error: {}", e))?;
        writer.write_image_data(rgba_data)
            .map_err(|e| format!("PNG encoder error: {}", e))?;
    }
    
    Ok((output, width, height))
}

// WebP 压缩函数
pub fn do_webp_compression(data: &[u8], width: u32, height: u32, quality: u8) -> Result<Vec<u8>, String> {
    // 简单的 WebP 压缩实现
    // 这里可以使用 webp 库，但为了简化，我们先返回错误
    Err("WebP compression not yet implemented".to_string())
}
