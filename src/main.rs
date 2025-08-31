mod compression;
mod handlers;
mod errors;
mod config;

use actix_web::{middleware::Logger, web, App, HttpServer};
use config::Config;
use log::info;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Load configuration first
    let config = Config::load().unwrap_or_else(|e| {
        eprintln!("Failed to load configuration: {}", e);
        eprintln!("Using default configuration");
        Config::default()
    });

    // Initialize logger with configured level
    env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or(&config.logging.level)
    ).init();

    info!("Starting Image Compression Server v{}", env!("CARGO_PKG_VERSION"));
    info!("Server will listen on http://{}", config.bind_address());
    info!("Maximum payload size: {}MB", config.server.max_file_size_mb);
    info!("Default compression quality: {}", config.compression.default_quality);
    info!("Default compression algorithm: {}", config.compression.default_algorithm);

    let bind_address = config.bind_address();
    let max_payload_size = config.max_file_size_bytes();
    let worker_threads = config.server.worker_threads;

    let mut server = HttpServer::new(move || {
        App::new()
            .app_data(web::PayloadConfig::new(max_payload_size))
            .app_data(web::Data::new(config.clone()))
            .wrap(Logger::default())
            .wrap(
                actix_web::middleware::DefaultHeaders::new()
                    .add(("Access-Control-Allow-Origin", "*"))
                    .add(("Access-Control-Allow-Methods", "GET, POST, OPTIONS"))
                    .add(("Access-Control-Allow-Headers", "Content-Type, Authorization")),
            )
            .route("/health", web::get().to(handlers::health_check))
            .route("/info", web::get().to(handlers::info_endpoint))
            .route("/compress", web::post().to(handlers::compress_endpoint))
            // 静态文件服务 - 放在最后以避免拦截API路由
            .service(actix_files::Files::new("/", "./static").index_file("index.html"))
    });

    // Set worker threads if specified
    if let Some(workers) = worker_threads {
        server = server.workers(workers);
    }

    server.bind(&bind_address)?.run().await
}
