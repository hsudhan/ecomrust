//! Shipments REST API (specs.md #2): actix-web server on port 4002.
//!   GET /shipments?page=&page_size=&sort=&order=  -> paginated list from Redis
//!   GET /shipment/{id}                            -> single shipment from Redis
//!
//! Run: cargo run --bin shipments_api

use actix_web::{get, web, App, HttpResponse, HttpServer};

use ecomrust::cache::{Cache, DEFAULT_REDIS_URL, SHIPMENTS_DATE_FIELD, SHIPMENTS_PREFIX};
use ecomrust::error::{AppError, AppResult};
use ecomrust::models::ShipmentJson;
use ecomrust::rest::{list_body, parse_params, ListQuery};

#[get("/shipments")]
async fn list_shipments(
    cache: web::Data<Cache>,
    query: web::Query<ListQuery>,
) -> AppResult<HttpResponse> {
    let params = parse_params(&query)?;
    let (rows, total) = cache
        .list(SHIPMENTS_PREFIX, SHIPMENTS_DATE_FIELD, &params)
        .await?;
    Ok(HttpResponse::Ok().json(list_body(rows, total, &params)))
}

#[get("/shipment/{id}")]
async fn get_shipment(cache: web::Data<Cache>, path: web::Path<i64>) -> AppResult<HttpResponse> {
    let id = path.into_inner();
    if id < 1 {
        return Err(AppError::BadRequest("id must be >= 1".to_string()));
    }
    let shipment: ShipmentJson = cache.get(SHIPMENTS_PREFIX, id).await?;
    Ok(HttpResponse::Ok().json(shipment))
}

#[get("/health")]
async fn health() -> HttpResponse {
    HttpResponse::Ok().json(serde_json::json!({ "status": "ok", "service": "shipments" }))
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let redis_url =
        std::env::var("REDIS_URL").unwrap_or_else(|_| DEFAULT_REDIS_URL.to_string());
    let cache = Cache::connect(&redis_url)
        .await
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::ConnectionRefused, e.to_string()))?;

    println!("shipments API listening on http://0.0.0.0:4002 (Redis: {redis_url})");
    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(cache.clone()))
            .service(list_shipments)
            .service(get_shipment)
            .service(health)
    })
    .bind(("0.0.0.0", 4002))?
    .run()
    .await
}
