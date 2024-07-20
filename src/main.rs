use actix_web::{web, App, HttpServer, HttpResponse, Error};
use actix_multipart::Multipart;
use futures::{StreamExt, TryStreamExt};
use image::ImageOutputFormat;
use std::io::Cursor;

async fn resize_image(mut payload: Multipart) -> Result<HttpResponse, Error> {
    while let Ok(Some(mut field)) = payload.try_next().await {
        let mut bytes = web::BytesMut::new();
        while let Some(chunk) = field.next().await {
            bytes.extend_from_slice(&chunk?);
        }

        // Use map_err to convert ImageError to a String, which implements ResponseError
        let img = image::load_from_memory(&bytes)
            .map_err(|e| actix_web::error::ErrorBadRequest(e.to_string()))?;
        
        let resized = img.resize(800, 600, image::imageops::FilterType::Lanczos3);

        let mut cursor = Cursor::new(Vec::new());
        resized.write_to(&mut cursor, ImageOutputFormat::Jpeg(80))
            .map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?;

        return Ok(HttpResponse::Ok().content_type("image/jpeg").body(cursor.into_inner()));
    }
    
    Ok(HttpResponse::BadRequest().body("No image found"))
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    std::env::set_var("RUST_LOG", "actix_web=info");
    env_logger::init();

    HttpServer::new(|| {
        App::new()
            .route("/resize", web::post().to(resize_image))
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}