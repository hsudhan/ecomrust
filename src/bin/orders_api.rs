//! Orders REST API (specs.md #2): actix-web server on port 4001.
//!   GET /orders?page=&page_size=&sort=&order=  -> paginated list, Redis first
//!   GET /order/{id}                            -> single order, Redis first
//! Cache misses fall back to the PostgreSQL table and write back to Redis.
//!
//! Run: cargo run --bin orders_api

use ecomrust::cache::{ORDERS_PREFIX, ORDERS_SORT_FIELDS};
use ecomrust::db;
use ecomrust::rest::{serve, ServiceConfig};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    serve(ServiceConfig {
        service_name: "orders",
        port: 4001,
        prefix: ORDERS_PREFIX,
        sort_fields: ORDERS_SORT_FIELDS,
        list_path: "/orders",
        item_path: "/order/{id}",
        fetch_all: db::fetch_all_orders,
        fetch_one: db::fetch_order,
    })
    .await
}
