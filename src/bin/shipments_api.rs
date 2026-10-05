//! Shipments REST API (specs.md #2): actix-web server on port 4002.
//!   GET /shipments?page=&page_size=&sort=&order=  -> paginated list, Redis first
//!   GET /shipment/{id}                            -> single shipment, Redis first
//! Cache misses fall back to the PostgreSQL table and write back to Redis.
//!
//! Run: cargo run --bin shipments_api

use ecomrust::cache::{SHIPMENTS_DATE_FIELD, SHIPMENTS_PREFIX};
use ecomrust::db;
use ecomrust::rest::{serve, ServiceConfig};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    serve(ServiceConfig {
        service_name: "shipments",
        port: 4002,
        prefix: SHIPMENTS_PREFIX,
        date_field: SHIPMENTS_DATE_FIELD,
        list_path: "/shipments",
        item_path: "/shipment/{id}",
        fetch_all: db::fetch_all_shipments,
        fetch_one: db::fetch_shipment,
    })
    .await
}
