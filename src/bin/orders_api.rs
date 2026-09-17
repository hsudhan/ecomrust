//! Orders REST API (specs.md #2): actix-web server on port 4001.
//!   GET /orders?page=&page_size=&sort=&order=  -> paginated list from Redis
//!   GET /order/{id}                            -> single order from Redis
//!
//! Run: cargo run --bin orders_api

use actix_web::{get, web, App, HttpResponse, HttpServer};

use ecomrust::cache::{Cache, DEFAULT_REDIS_URL, ORDERS_DATE_FIELD, ORDERS_PREFIX};
use ecomrust::error::{AppError, AppResult};
use ecomrust::models::OrderJson;
use ecomrust::rest::{list_body, parse_params, ListQuery};

#[get("/orders")]
async fn list_orders(
    cache: web::Data<Cache>,
    query: web::Query<ListQuery>,
) -> AppResult<HttpResponse> {
    let params = parse_params(&query)?;
    let (rows, total) = cache
        .list(ORDERS_PREFIX, ORDERS_DATE_FIELD, &params)
        .await?;
    Ok(HttpResponse::Ok().json(list_body(rows, total, &params)))
}

#[get("/order/{id}")]
async fn get_order(cache: web::Data<Cache>, path: web::Path<i64>) -> AppResult<HttpResponse> {
    let id = path.into_inner();
    if id < 1 {
        return Err(AppError::BadRequest("id must be >= 1".to_string()));
    }
    let order: OrderJson = cache.get(ORDERS_PREFIX, id).await?;
    Ok(HttpResponse::Ok().json(order))
}

#[get("/health")]
async fn health() -> HttpResponse {
    HttpResponse::Ok().json(serde_json::json!({ "status": "ok", "service": "orders" }))
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let redis_url =
        std::env::var("REDIS_URL").unwrap_or_else(|_| DEFAULT_REDIS_URL.to_string());
    let cache = Cache::connect(&redis_url)
        .await
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::ConnectionRefused, e.to_string()))?;

    println!("orders API listening on http://0.0.0.0:4001 (Redis: {redis_url})");
    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(cache.clone()))
            .service(list_orders)
            .service(get_order)
            .service(health)
    })
    .bind(("0.0.0.0", 4001))?
    .run()
    .await
}
