//! Shipment trackings REST API (specs.md #2): actix-web server on port 4008.
//!   GET /shipment-trackings?page=&page_size=&sort=&order=  -> paginated list, Redis first
//!   GET /shipment-tracking/{id}                            -> single tracking row, Redis first
//! Cache misses fall back to the PostgreSQL table and write back to Redis.
//!
//! Run: cargo run --bin shipment_trackings_api

use ecomrust::cache::{SHIPMENT_TRACKINGS_DATE_FIELD, SHIPMENT_TRACKINGS_PREFIX};
use ecomrust::db;
use ecomrust::rest::{serve, ServiceConfig};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    serve(ServiceConfig {
        service_name: "shipment-trackings",
        port: 4008,
        prefix: SHIPMENT_TRACKINGS_PREFIX,
        date_field: SHIPMENT_TRACKINGS_DATE_FIELD,
        list_path: "/shipment-trackings",
        item_path: "/shipment-tracking/{id}",
        fetch_all: db::fetch_all_shipment_trackings,
        fetch_one: db::fetch_shipment_tracking,
    })
    .await
}
