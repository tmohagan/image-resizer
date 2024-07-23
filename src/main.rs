use actix_web::{web, App, HttpServer, HttpResponse, Error};
use actix_multipart::Multipart;
use actix_cors::Cors;
use futures::{StreamExt, TryStreamExt};
use image::ImageOutputFormat;
use std::io::Cursor;
use std::env;
use log::{info, error};

async fn resize_image(mut payload: Multipart) -> Result<HttpResponse, Error> {
    info!("Received resize request");
    while let Ok(Some(mut field)) = payload.try_next().await {
        let content_type = field.content_disposition().ok_or_else(|| {
            error!("Content-Type not found");
            actix_web::error::ErrorBadRequest("Content-Type not found")
        })?;
        
        let filename = content_type.get_filename().ok_or_else(|| {
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

        let img = image::load_from_memory(&bytes).map_err(|e| {
            error!("Error loading image: {:?}", e);
            actix_web::error::ErrorBadRequest(e.to_string())
        })?;
        
        let resized = img.resize_exact(800, 600, image::imageops::FilterType::Lanczos3);

        let mut cursor = Cursor::new(Vec::new());
        resized.write_to(&mut cursor, ImageOutputFormat::Jpeg(80))
            .map_err(|e| {
                error!("Error writing image: {:?}", e);
                actix_web::error::ErrorInternalServerError(e.to_string())
            })?;

        info!("Image '{}' resized successfully", filename);
        return Ok(HttpResponse::Ok()
            .content_type("image/jpeg")
            .append_header((
                "Content-Disposition", 
                format!("attachment; filename=\"resized_{filename}\"")
            ))
            .body(cursor.into_inner()));
    }
    
    Err(actix_web::error::ErrorBadRequest("No image found"))
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Initialize logger
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    // Get the port from the environment variable or use a default
    let port = env::var("PORT").unwrap_or_else(|_| "8080".to_string());
    let addr = format!("0.0.0.0:{}", port);

    info!("Starting server at: {}", addr);

    HttpServer::new(|| {
        let cors = Cors::default()
            .allowed_origin("https://www.tim-ohagan.com")
            .allowed_origin("https://mern-blog-api-rouge.vercel.app")
            .allowed_methods(vec!["GET", "POST"])
            .allowed_headers(vec![
                actix_web::http::header::AUTHORIZATION,
                actix_web::http::header::ACCEPT,
                actix_web::http::header::CONTENT_TYPE,
            ])
            .max_age(3600);

        App::new()
            .wrap(cors)
            .route("/resize", web::post().to(resize_image))
            .route("/health", web::get().to(|| async { HttpResponse::Ok().body("Healthy") }))
    })
    .bind(&addr)?
    .run()
    .await
}