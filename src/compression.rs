use log::info;
use image::DynamicImage;
use std::time::Instant;
use std::io::Cursor;
use exif::{Reader, In, Tag, Value};

// 压缩图片的主要函数
pub fn compress_image(
    data: &[u8],
    format: &str,
    quality: u8,
    algorithm: &str
) -> Result<(Vec<u8>, u32, u32, String), String> {
    let total_start = Instant::now();
    info!("开始压缩图片 - 目标格式: {}, 质量: {} (保持原始尺寸), 算法: {}", 
         format, quality, algorithm);
    
    // 读取EXIF信息（仅针对JPEG）
    let exif_orientation = if format.to_lowercase() == "jpeg" || format.to_lowercase() == "jpg" {
        read_exif_orientation(data)
    } else {
        None
    };
    
    if let Some(orientation) = exif_orientation {
        info!("检测到EXIF方向信息: {}", orientation);
    }
    
    // 优化的图片加载
    let load_start = Instant::now();
    
    // 使用通用解码器加载图片
    let mut img = image::load_from_memory(data)
        .map_err(|e| format!("Failed to decode image: {}", e))?;
    
    // 应用EXIF方向校正（仅在JPEG压缩时）
    let exif_info = if format.to_lowercase() == "jpeg" || format.to_lowercase() == "jpg" {
        if let Some(orientation) = exif_orientation {
            img = apply_exif_orientation(img, orientation);
            format!("Applied EXIF orientation: {}", orientation)
        } else {
            "No EXIF orientation found".to_string()
        }
    } else {
        "No EXIF processing".to_string()
    };
    
    let original_width = img.width();
    let original_height = img.height();
    let load_duration = load_start.elapsed();
    
    info!("图片加载完成 - 尺寸: {}x{}, 加载时间: {:.2}ms, EXIF处理: {}", 
          original_width, original_height, load_duration.as_secs_f64() * 1000.0, exif_info);
    
    // 不修改尺寸，直接压缩原始尺寸的图片
    let compression_start = Instant::now();
    let compressed_data = match format.to_lowercase().as_str() {
        "jpeg" | "jpg" => {
            info!("进行 JPEG 压缩，保持原始尺寸 {}x{}，使用算法: {}", original_width, original_height, algorithm);
            match algorithm.to_lowercase().as_str() {
                "mozjpeg" => {
                    info!("使用 mozjpeg 进行 JPEG 压缩");
                    do_mozjpeg_compression(img, quality)?
                },
                "jpeg-encoder" => {
                    info!("使用 jpeg-encoder 进行 JPEG 压缩");
                    do_jpeg_encoder_compression(img, quality)?
                },
                _ => {
                    info!("未知算法 '{}', 默认使用 mozjpeg", algorithm);
                    do_mozjpeg_compression(img, quality)?
                }
            }
        },
        "png" => {
            info!("进行 PNG 压缩，保持原始尺寸 {}x{}", original_width, original_height);
            let result = do_png_compression(&img.to_rgba8().into_raw(), original_width, original_height)?;
            result.0
        },
        "webp" => {
            info!("进行 WebP 压缩，保持原始尺寸 {}x{}", original_width, original_height);
            do_webp_compression(&img.to_rgba8().into_raw(), original_width, original_height, quality)?
        },
        _ => return Err(format!("Unsupported format: {}", format))
    };
    let compression_duration = compression_start.elapsed();
    
    let final_size = compressed_data.len();
    let total_duration = total_start.elapsed();
    
    info!("压缩完成 - 保持原始尺寸 {}x{}, 最终大小: {} bytes", original_width, original_height, final_size);
    info!("性能统计 - 加载时间: {:.2}ms, 压缩时间: {:.2}ms, 总时间: {:.2}ms", 
         load_duration.as_secs_f64() * 1000.0, 
         compression_duration.as_secs_f64() * 1000.0,
         total_duration.as_secs_f64() * 1000.0);
    
    Ok((compressed_data, original_width, original_height, exif_info))
}

// 读取EXIF方向信息
fn read_exif_orientation(data: &[u8]) -> Option<u16> {
    let mut cursor = Cursor::new(data);
    
    match Reader::new().read_from_container(&mut cursor) {
        Ok(exif) => {
            if let Some(orientation_field) = exif.get_field(Tag::Orientation, In::PRIMARY) {
                match orientation_field.value {
                    Value::Short(ref vec) if !vec.is_empty() => {
                        let orientation = vec[0];
                        info!("读取到EXIF方向信息: {}", orientation);
                        Some(orientation)
                    },
                    _ => {
                        info!("EXIF方向信息格式不正确");
                        None
                    }
                }
            } else {
                info!("未找到EXIF方向信息");
                None
            }
        },
        Err(e) => {
            info!("读取EXIF信息失败: {}", e);
            None
        }
    }
}

// 根据EXIF方向信息旋转图片
fn apply_exif_orientation(img: DynamicImage, orientation: u16) -> DynamicImage {
    match orientation {
        1 => {
            info!("EXIF方向1: 无需旋转");
            img
        },
        2 => {
            info!("EXIF方向2: 水平翻转");
            img.fliph()
        },
        3 => {
            info!("EXIF方向3: 旋转180度");
            img.rotate180()
        },
        4 => {
            info!("EXIF方向4: 垂直翻转");
            img.flipv()
        },
        5 => {
            info!("EXIF方向5: 逆时针旋转90度后水平翻转");
            img.rotate270().fliph()
        },
        6 => {
            info!("EXIF方向6: 顺时针旋转90度");
            img.rotate90()
        },
        7 => {
            info!("EXIF方向7: 顺时针旋转90度后水平翻转");
            img.rotate90().fliph()
        },
        8 => {
            info!("EXIF方向8: 逆时针旋转90度");
            img.rotate270()
        },
        _ => {
            info!("未知EXIF方向: {}, 保持原状", orientation);
            img
        }
    }
}

// mozjpeg 压缩函数
fn do_mozjpeg_compression(img: DynamicImage, quality: u8) -> Result<Vec<u8>, String> {
    info!("开始 mozjpeg 压缩");
    
    // 转换为 RGB
    let rgb_img = img.to_rgb8();
    let (width, height) = (rgb_img.width(), rgb_img.height());
    let raw_data = rgb_img.into_raw();
    
    info!("图片信息 - 宽: {}, 高: {}, 数据长度: {}", width, height, raw_data.len());
    
    // 使用 mozjpeg::Compress API
    let mut comp = mozjpeg::Compress::new(mozjpeg::ColorSpace::JCS_RGB);
    comp.set_size(width as usize, height as usize);
    comp.set_quality(quality as f32);
    comp.set_mem_dest();
    comp.start_compress();
    
    // 写入扫描线
    let line_size = width as usize * 3; // RGB = 3 bytes per pixel
    for y in 0..height as usize {
        let offset = y * line_size;
        let line = &raw_data[offset..offset + line_size];
        comp.write_scanlines(line);
    }
    
    comp.finish_compress();
    let jpeg_data = comp.data_to_vec().unwrap();
    
    info!("mozjpeg 压缩成功，输出大小: {} bytes", jpeg_data.len());
    Ok(jpeg_data)
}

// jpeg-encoder 压缩函数
fn do_jpeg_encoder_compression(img: DynamicImage, quality: u8) -> Result<Vec<u8>, String> {
    info!("开始 jpeg-encoder 压缩");
    
    // 转换为 RGB
    let rgb_img = img.to_rgb8();
    let (width, height) = (rgb_img.width(), rgb_img.height());
    let raw_data = rgb_img.into_raw();
    
    info!("图片信息 - 宽: {}, 高: {}, 数据长度: {}", width, height, raw_data.len());
    
    // 使用 jpeg-encoder
    use jpeg_encoder::{Encoder, ColorType};
    
    let mut output = Vec::new();
    let encoder = Encoder::new(&mut output, quality);
    encoder.encode(&raw_data, width as u16, height as u16, ColorType::Rgb)
        .map_err(|e| format!("JPEG encoder failed: {:?}", e))?;
    
    info!("jpeg-encoder 压缩成功，输出大小: {} bytes", output.len());
    Ok(output)
}

// PNG 压缩函数 - 基于 fast-image 项目的高性能实现
pub fn do_png_compression(rgba_data: &[u8], width: u32, height: u32) -> Result<(Vec<u8>, u32, u32), String> {
    info!("开始 PNG 压缩 - 尺寸: {}x{}, 数据大小: {} bytes", width, height, rgba_data.len());
    
    let start_time = Instant::now();
    
    // 设置默认质量为85，适用于PNG量化
    let quality = 85u8;
    
    let width_usize = width as usize;
    let height_usize = height as usize;
    
    // 内存优化信息
    let pixel_count = width_usize * height_usize;
    let memory_size_mb = (pixel_count * 4) / (1024 * 1024);
    
    // 使用 imagequant 进行颜色量化
    let mut liq = imagequant::new();
    liq.set_quality(0, quality)
        .map_err(|e| format!("Failed to set PNG quality: {:?}", e))?;
    
    // 优化的 RGBA 转换，支持零拷贝
    let use_zero_copy = can_use_zero_copy();
    
    let mut img_quantize = if use_zero_copy {
        // 零拷贝路径：直接重新解释内存布局
        // 这为大图片节省约50%的内存
        let rgba_pixels = unsafe {
            std::slice::from_raw_parts(
                rgba_data.as_ptr() as *const imagequant::RGBA,
                pixel_count,
            )
        };
        
        if memory_size_mb > 50 {
            info!("PNG compression: Using zero-copy optimization for {}MB image ({}x{})",
                  memory_size_mb, width, height);
        }
        
        liq.new_image(rgba_pixels, width_usize, height_usize, 0.0)
            .map_err(|e| format!("Failed to create quantized image with zero-copy: {:?}", e))?
    } else {
        // 预分配路径：最小化分配开销
        let mut rgba_pixels = Vec::with_capacity(rgba_data.len() / 4);
        
        // 使用 chunks_exact 获得更好的性能（无边界检查）
        for chunk in rgba_data.chunks_exact(4) {
            rgba_pixels.push(imagequant::RGBA {
                r: chunk[0],
                g: chunk[1],
                b: chunk[2],
                a: chunk[3],
            });
        }
        
        // 记录回退原因以便调试
        if memory_size_mb > 50 {
            info!("PNG compression: Using pre-allocation fallback for {}MB image ({}x{}) - zero-copy not available",
                  memory_size_mb, width, height);
        }
        
        liq.new_image(&rgba_pixels[..], width_usize, height_usize, 0.0)
            .map_err(|e| format!("Failed to create quantized image with pre-allocation: {:?}", e))?
    };
    
    // 量化图像
    let mut res = liq
        .quantize(&mut img_quantize)
        .map_err(|e| format!("Failed to quantize PNG: {:?}", e))?;
    
    // 设置抖动级别 (0.0 - 1.0)
    res.set_dithering_level(1.0)
        .map_err(|e| format!("Failed to set dithering: {:?}", e))?;
    
    // 获取量化数据
    let (palette, pixels) = res
        .remapped(&mut img_quantize)
        .map_err(|e| format!("Failed to remap PNG: {:?}", e))?;
    
    // 使用量化调色板创建 PNG
    let mut png_data = Vec::new();
    
    {
        let mut encoder = png::Encoder::new(Cursor::new(&mut png_data), width, height);
        encoder.set_color(png::ColorType::Indexed);
        encoder.set_depth(png::BitDepth::Eight);
        
        // 根据质量设置压缩级别（反向：质量越低 = 压缩越高）
        let compression_level = png::Compression::Best;
        encoder.set_compression(compression_level);
        
        // 转换调色板为 PNG 编码器期望的格式
        let png_palette: Vec<u8> = palette.iter()
            .flat_map(|color| [color.r, color.g, color.b])
            .collect();
        
        // 为索引 PNG 构建 tRNS（透明度）块以保留 alpha
        // PNG 规范：调色板图像的 tRNS 为前 N 个条目提供 alpha；
        // 剩余条目假定为完全不透明 (255)
        let mut trns: Vec<u8> = palette.iter().map(|c| c.a).collect();
        
        // 修剪尾部完全不透明的 alpha 值以保持块最小
        while trns.last().copied() == Some(255) {
            trns.pop();
        }
        
        encoder.set_palette(png_palette);
        // 仅在存在任何透明度时设置 tRNS
        if !trns.is_empty() {
            encoder.set_trns(trns);
        }
        
        let mut writer = encoder.write_header()
            .map_err(|e| format!("Failed to write PNG header: {}", e))?;
        
        // 写入索引像素数据
        writer.write_image_data(&pixels)
            .map_err(|e| format!("Failed to write PNG data: {}", e))?;
    }
    
    let duration = start_time.elapsed();
    info!("PNG 压缩完成 - 输出大小: {} bytes, 耗时: {:.2}ms", 
          png_data.len(), duration.as_secs_f64() * 1000.0);
    
    Ok((png_data, width, height))
}

/// 检查 imagequant::RGBA 的零拷贝转换是否安全
/// 这验证了 imagequant::RGBA 与 [u8; 4] 具有相同的内存布局
pub fn can_use_zero_copy() -> bool {
    // 验证内存布局兼容性
    std::mem::size_of::<imagequant::RGBA>() == 4
        && std::mem::align_of::<imagequant::RGBA>() == 1
        && is_rgba_layout_compatible()
}

/// imagequant::RGBA 内存布局的运行时验证
/// 测试 imagequant::RGBA 字段是否按 R,G,B,A 顺序排列
pub fn is_rgba_layout_compatible() -> bool {
    let test_bytes = [0x12u8, 0x34u8, 0x56u8, 0x78u8];
    let rgba: imagequant::RGBA = unsafe {
        std::mem::transmute(test_bytes)
    };
    
    rgba.r == 0x12 && rgba.g == 0x34 && rgba.b == 0x56 && rgba.a == 0x78
}

// WebP 压缩函数
pub fn do_webp_compression(_data: &[u8], _width: u32, _height: u32, _quality: u8) -> Result<Vec<u8>, String> {
    // 简单的 WebP 压缩实现
    // 这里可以使用 webp 库，但为了简化，我们先返回错误
    Err("WebP compression not yet implemented".to_string())
}
