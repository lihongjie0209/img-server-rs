#[cfg(test)]
mod api_tests {
    use actix_web::{test, web, App};
    use img_server_rs::handlers::{compress_endpoint, health_check, info_endpoint};

    #[actix_web::test]
    async fn test_health_endpoint() {
        let app = test::init_service(
            App::new().route("/health", web::get().to(health_check))
        ).await;

        let req = test::TestRequest::get()
            .uri("/health")
            .to_request();
        
        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());

        let body = test::read_body(resp).await;
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        
        assert_eq!(json["status"], "ok");
        assert_eq!(json["service"], "image-compression-server");
    }

    #[actix_web::test]
    async fn test_info_endpoint() {
        let app = test::init_service(
            App::new().route("/info", web::get().to(info_endpoint))
        ).await;

        let req = test::TestRequest::get()
            .uri("/info")
            .to_request();
        
        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());

        let body = test::read_body(resp).await;
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        
        assert_eq!(json["service"], "Image Compression Server");
        assert!(json["supported_algorithms"].is_array());
    }

    #[actix_web::test]
    async fn test_compress_endpoint_no_file() {
        let app = test::init_service(
            App::new()
                .app_data(web::PayloadConfig::new(100 * 1024 * 1024))
                .route("/compress", web::post().to(compress_endpoint))
        ).await;

        // Test request without file
        let req = test::TestRequest::post()
            .uri("/compress")
            .set_form(&[("quality", "80")])
            .to_request();
        
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 400); // Bad request - no file provided
    }

    // Helper function to create test image data
    fn create_simple_png() -> Vec<u8> {
        use image::{ImageBuffer, Rgb};
        
        let mut img = ImageBuffer::<Rgb<u8>, Vec<u8>>::new(50, 50);
        
        // Create a simple pattern
        for (x, y, pixel) in img.enumerate_pixels_mut() {
            *pixel = Rgb([
                (x % 256) as u8,
                (y % 256) as u8,
                ((x + y) % 256) as u8,
            ]);
        }
        
        let mut buffer = Vec::new();
        img.write_to(
            &mut std::io::Cursor::new(&mut buffer),
            image::ImageOutputFormat::Png
        ).expect("Failed to encode test image");
        
        buffer
    }

    // Note: Testing multipart/form-data endpoints with actix-web test framework
    // is complex and requires additional setup. The compress endpoint would need
    // more sophisticated testing with actual multipart data, which is typically
    // done with integration tests using a real HTTP client.
    
    #[test]
    fn test_image_creation_helper() {
        let png_data = create_simple_png();
        assert!(!png_data.is_empty());
        
        // Verify it's actually PNG data
        assert!(png_data.starts_with(&[0x89, 0x50, 0x4E, 0x47])); // PNG signature
    }
}
