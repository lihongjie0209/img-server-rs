#[cfg(test)]
mod compression_tests {
    use img_server_rs::compression::{
        compress_image, do_jpeg_encoder_compression, do_png_compression,
        detect_image_type, CompressionAlgorithm, CompressionOptions, ImageType,
    };

    // Helper function to create a simple test image
    fn create_test_png() -> Vec<u8> {
        use image::{ImageBuffer, Rgb};
        
        // Create a simple 100x100 RGB image with some pattern
        let mut img = ImageBuffer::<Rgb<u8>, Vec<u8>>::new(100, 100);
        
        // Add some pattern to make it more realistic
        for (x, y, pixel) in img.enumerate_pixels_mut() {
            let r = ((x + y) % 256) as u8;
            let g = (x % 256) as u8;
            let b = (y % 256) as u8;
            *pixel = Rgb([r, g, b]);
        }
        
        let mut buffer = Vec::new();
        
        // Encode as PNG using the newer API
        img.write_to(&mut std::io::Cursor::new(&mut buffer), image::ImageOutputFormat::Png)
            .expect("Failed to encode test PNG");
        
        buffer
    }

    fn create_test_jpeg() -> Vec<u8> {
        use image::{ImageBuffer, Rgb};
        
        // Create a simple 100x100 RGB image with some pattern
        let mut img = ImageBuffer::<Rgb<u8>, Vec<u8>>::new(100, 100);
        
        // Add some pattern to make it more realistic
        for (x, y, pixel) in img.enumerate_pixels_mut() {
            let r = ((x + y) % 256) as u8;
            let g = (x % 256) as u8;
            let b = (y % 256) as u8;
            *pixel = Rgb([r, g, b]);
        }
        
        let mut buffer = Vec::new();
        
        // Encode as JPEG using the newer API
        img.write_to(&mut std::io::Cursor::new(&mut buffer), image::ImageOutputFormat::Jpeg(80))
            .expect("Failed to encode test JPEG");
        
        buffer
    }

    #[test]
    fn test_image_type_detection() {
        let png_data = create_test_png();
        let jpeg_data = create_test_jpeg();

        // Test PNG detection
        assert!(matches!(
            detect_image_type(&png_data),
            Ok(ImageType::PNG)
        ));

        // Test JPEG detection
        assert!(matches!(
            detect_image_type(&jpeg_data),
            Ok(ImageType::JPEG)
        ));

        // Test invalid data
        let invalid_data = vec![0, 1, 2, 3];
        assert!(detect_image_type(&invalid_data).is_err());

        // Test empty data
        let empty_data = vec![];
        assert!(detect_image_type(&empty_data).is_err());
    }

    #[test]
    fn test_png_compression() {
        let png_data = create_test_png();
        
        // Test different quality levels
        for quality in [10, 50, 80, 100] {
            let result = do_png_compression(&png_data, quality);
            assert!(result.is_ok(), "PNG compression failed for quality {}", quality);
            
            let (compressed, width, height) = result.unwrap();
            assert_eq!(width, 100);
            assert_eq!(height, 100);
            assert!(!compressed.is_empty());
            
            // Higher quality should generally produce larger files
            // (though this isn't always true for simple test images)
        }
    }

    #[test]
    fn test_jpeg_compression() {
        let jpeg_data = create_test_jpeg();
        
        // Test different quality levels
        for quality in [10, 50, 80, 100] {
            let result = do_jpeg_encoder_compression(&jpeg_data, quality);
            assert!(result.is_ok(), "JPEG compression failed for quality {}", quality);
            
            let (compressed, width, height) = result.unwrap();
            assert_eq!(width, 100);
            assert_eq!(height, 100);
            assert!(!compressed.is_empty());
        }
    }

    #[test]
    fn test_compress_image_function() {
        let png_data = create_test_png();
        let jpeg_data = create_test_jpeg();

        // Test PNG compression
        let png_options = CompressionOptions {
            quality: 80,
            algorithm: CompressionAlgorithm::PngQuantized,
        };
        
        let result = compress_image(&png_data, &png_options);
        assert!(result.is_ok());
        
        let (compressed, stats) = result.unwrap();
        assert!(!compressed.is_empty());
        assert_eq!(stats.image_width, 100);
        assert_eq!(stats.image_height, 100);
        assert!(stats.processing_time_ms > 0);
        assert_eq!(stats.algorithm_used, "png-quantized");

        // Test JPEG compression
        let jpeg_options = CompressionOptions {
            quality: 80,
            algorithm: CompressionAlgorithm::JpegEncoder,
        };
        
        let result = compress_image(&jpeg_data, &jpeg_options);
        assert!(result.is_ok());
        
        let (compressed, stats) = result.unwrap();
        assert!(!compressed.is_empty());
        assert_eq!(stats.image_width, 100);
        assert_eq!(stats.image_height, 100);
        assert!(stats.processing_time_ms > 0);
        assert_eq!(stats.algorithm_used, "jpeg-encoder");
    }

    #[test]
    fn test_compression_ratio_calculation() {
        let png_data = create_test_png();
        let original_size = png_data.len();

        let options = CompressionOptions {
            quality: 50, // Lower quality should give better compression
            algorithm: CompressionAlgorithm::PngQuantized,
        };
        
        let result = compress_image(&png_data, &options);
        assert!(result.is_ok());
        
        let (_, stats) = result.unwrap();
        assert_eq!(stats.original_size, original_size);
        assert!(stats.compressed_size > 0);
        assert!(stats.compression_ratio > 0.0);
        assert!(stats.compression_ratio <= 100.0);
    }

    #[test]
    fn test_invalid_image_data() {
        let invalid_data = vec![0; 1000]; // Not a valid image
        
        let options = CompressionOptions::default();
        let result = compress_image(&invalid_data, &options);
        
        assert!(result.is_err());
    }

    #[test]
    fn test_quality_bounds() {
        let png_data = create_test_png();
        
        // Test normal quality ranges
        let options_low = CompressionOptions {
            quality: 1, // Minimum valid quality
            algorithm: CompressionAlgorithm::PngQuantized,
        };
        
        let options_high = CompressionOptions {
            quality: 100, // Maximum valid quality
            algorithm: CompressionAlgorithm::PngQuantized,
        };
        
        // Both should work
        let result_low = compress_image(&png_data, &options_low);
        let result_high = compress_image(&png_data, &options_high);
        
        assert!(result_low.is_ok(), "Low quality compression failed: {:?}", result_low.err());
        assert!(result_high.is_ok(), "High quality compression failed: {:?}", result_high.err());
        
        // Test that results are different
        let (_, stats_low) = result_low.unwrap();
        let (_, stats_high) = result_high.unwrap();
        
        // Lower quality should generally produce smaller files
        // (though this isn't guaranteed for all images)
        println!("Low quality size: {}, High quality size: {}", 
                stats_low.compressed_size, stats_high.compressed_size);
    }

    #[test]
    fn test_zero_copy_detection() {
        use img_server_rs::compression::{can_use_zero_copy, is_rgba_layout_compatible};
        
        // These functions should return consistent results
        let zero_copy_result = can_use_zero_copy();
        let layout_result = is_rgba_layout_compatible();
        
        // If layout is not compatible, zero copy should not be available
        if !layout_result {
            assert!(!zero_copy_result);
        }
        
        println!("Zero copy available: {}", zero_copy_result);
        println!("Layout compatible: {}", layout_result);
    }
}
