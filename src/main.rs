use actix_multipart::Multipart;
use actix_web::{web, App, Error, HttpResponse, HttpServer};
use futures::{StreamExt, TryStreamExt};
use image::{ImageFormat, ImageOutputFormat};
use std::io::Cursor;
use std::path::Path;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Get the current directory
    let current_dir = std::env::current_dir()?;
    let static_path = current_dir.join("static");
    
    // Check if static directory exists
    if !static_path.exists() {
        eprintln!("ERROR: 'static' directory not found at: {:?}", static_path);
        eprintln!("Please create this directory and place index.html inside it.");
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "static directory not found",
        ));
    }
    
    println!("Starting server at http://127.0.0.1:8080");
    println!("Serving static files from: {:?}", static_path);
    
    HttpServer::new(move || {
        App::new()
            .service(web::resource("/api/convert").route(web::post().to(convert_image)))
            .service(actix_files::Files::new("/", static_path.clone()).index_file("index.html"))
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}

async fn convert_image(mut payload: Multipart) -> Result<HttpResponse, Error> {
    let mut image_data = Vec::new();
    let mut format = String::new();
    
    // Process multipart form data
    while let Ok(Some(mut field)) = payload.try_next().await {
        let content_disposition = field.content_disposition();
        let name = content_disposition.get_name().unwrap();
        match name {
            "image" => {
                // Read image data
                while let Some(chunk) = field.next().await {
                    let data = chunk?;
                    image_data.extend_from_slice(&data);
                }
            }
            "format" => {
                // Read format
                while let Some(chunk) = field.next().await {
                    let data = chunk?;
                    format = String::from_utf8(data.to_vec()).unwrap_or_default();
                }
            }
            _ => {
                // Skip other fields
                while let Some(_) = field.next().await {}
            }
        }
    }
    
    if image_data.is_empty() {
        return Ok(HttpResponse::BadRequest().body("No image provided"));
    }
    
    if format.is_empty() {
        return Ok(HttpResponse::BadRequest().body("No format specified"));
    }
    
    // Convert the image
    match convert(&image_data, &format) {
        Ok(converted_data) => {
            // Set the appropriate content type
            let content_type = match format.as_str() {
                "webp" => "image/webp",
                "avif" => "image/avif",
                _ => "application/octet-stream",
            };
            
            Ok(HttpResponse::Ok()
                .content_type(content_type)
                .body(converted_data))
        }
        Err(e) => {
            Ok(HttpResponse::InternalServerError().body(format!("Conversion failed: {}", e)))
        }
    }
}

fn convert(data: &[u8], format: &str) -> Result<Vec<u8>, String> {
    // Load the image
    let img = image::load_from_memory(data)
        .map_err(|e| format!("Failed to load image: {}", e))?;
    
    // Prepare output buffer
    let mut output = Cursor::new(Vec::new());
    
    // Convert to the requested format
    match format {
        "webp" => {
            // Use the WebP format with lossless compression for best quality
            img.write_to(&mut output, ImageOutputFormat::WebP)
                .map_err(|e| format!("Failed to convert to WebP: {}", e))?;
        }
        "avif" => {
            // Note: as of early 2024, the image crate still doesn't fully support AVIF
            // This is a placeholder - in a real application, you would use a library like ravif
            return Err("AVIF conversion is not yet implemented in this example".to_string());
        }
        _ => {
            return Err(format!("Unsupported format: {}", format));
        }
    }
    
    Ok(output.into_inner())
}
