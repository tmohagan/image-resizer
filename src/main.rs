use actix_web::{web, App, HttpServer, HttpResponse, Error};
use actix_multipart::Multipart;
use actix_cors::Cors;
use futures::{StreamExt, TryStreamExt};
use image::{ImageOutputFormat, DynamicImage};
use std::io::Cursor;
use std::env;
use log::{info, error};
use dotenv::dotenv;
use lru_cache::LruCache;
use std::sync::Mutex;

use lazy_static::lazy_static;

lazy_static! {
    static ref CACHE: Mutex<LruCache<String, Vec<u8>>> = Mutex::new(LruCache::new(100));
}

fn resize_image_rayon(img: DynamicImage, width: u32, height: u32, quality: u8) -> Vec<u8> {
    let resized = img.resize_exact(width, height, image::imageops::FilterType::Lanczos3);
    let mut cursor = Cursor::new(Vec::new());
    resized.write_to(&mut cursor, ImageOutputFormat::Jpeg(quality)).unwrap();
    cursor.into_inner()
}

async fn resize_image(mut payload: Multipart) -> Result<HttpResponse, Error> {
    info!("Received resize request");
    while let Ok(Some(mut field)) = payload.try_next().await {
        let content_disposition = field.content_disposition();
        
        let filename = content_disposition
            .get_filename()
            .map(|f| f.to_owned())
            .ok_or_else(|| {
                error!("Filename not found");
                actix_web::error::ErrorBadRequest("Filename not found")
            })?;

        let mut bytes = web::BytesMut::new();
        while let Some(chunk) = field.next().await {
            bytes.extend_from_slice(&chunk.map_err(|e| {
                error!("Error reading chunk: {:?}", e);
                actix_web::error::ErrorInternalServerError(e)
            })?);
        }

        let width = env::var("RESIZE_WIDTH").unwrap_or_else(|_| "800".to_string()).parse().unwrap_or(800);
        let height = env::var("RESIZE_HEIGHT").unwrap_or_else(|_| "600".to_string()).parse().unwrap_or(600);
        let quality = env::var("JPEG_QUALITY").unwrap_or_else(|_| "80".to_string()).parse().unwrap_or(80);

        let cache_key = format!("{}_{}_{}_{}", filename, width, height, quality);
        
        if let Some(cached_image) = CACHE.lock().unwrap().get_mut(&cache_key) {
            info!("Serving cached image for '{}'", filename);
            return Ok(HttpResponse::Ok()
                .content_type("image/jpeg")
                .append_header((
                    "Content-Disposition", 
                    format!("attachment; filename=\"resized_{filename}\"")
                ))
                .body(cached_image.clone()));
        }

        let img = image::load_from_memory(&bytes).map_err(|e| {
            error!("Error loading image: {:?}", e);
            actix_web::error::ErrorBadRequest(e.to_string())
        })?;

        let resized_image = web::block(move || {
            resize_image_rayon(img, width, height, quality)
        }).await?;

        CACHE.lock().unwrap().insert(cache_key, resized_image.clone());

        info!("Image '{}' resized successfully", filename);
        return Ok(HttpResponse::Ok()
            .content_type("image/jpeg")
            .append_header((
                "Content-Disposition", 
                format!("attachment; filename=\"resized_{filename}\"")
            ))
            .body(resized_image));
    }
    
    Err(actix_web::error::ErrorBadRequest("No image found"))
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();
    
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let port = env::var("PORT").unwrap_or_else(|_| "8080".to_string());
    let host = env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
    let addr = format!("{}:{}", host, port);

    let allowed_origins: Vec<String> = env::var("ALLOWED_ORIGINS")
        .unwrap_or_else(|_| "https://www.tim-ohagan.com,https://mern-blog-api-rouge.vercel.app".to_string())
        .split(',')
        .map(|s| s.trim().to_string())
        .collect();

    info!("Starting server at: {}", addr);

    HttpServer::new(move || {
        let mut cors = Cors::default()
            .allowed_methods(vec!["GET", "POST"])
            .allowed_headers(vec![
                actix_web::http::header::AUTHORIZATION,
                actix_web::http::header::ACCEPT,
                actix_web::http::header::CONTENT_TYPE,
            ])
            .max_age(3600);

        for origin in &allowed_origins {
            cors = cors.allowed_origin(origin);
        }

        App::new()
            .wrap(cors)
            .route("/resize", web::post().to(resize_image))
            .route("/health", web::get().to(|| async { HttpResponse::Ok().body("Healthy") }))
    })
    .bind(&addr)?
    .run()
    .await
}