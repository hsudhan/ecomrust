//! Shopping carts REST API (specs.md #2): actix-web server on port 4005.
//!   GET /shopping-carts?page=&page_size=&sort=&order=  -> paginated list, Redis first
//!   GET /shopping-cart/{id}                            -> single cart row, Redis first
//! Cache misses fall back to the PostgreSQL table and write back to Redis.
//!
//! Run: cargo run --bin shopping_carts_api

use ecomrust::cache::{SHOPPING_CARTS_DATE_FIELD, SHOPPING_CARTS_PREFIX};
use ecomrust::db;
use ecomrust::rest::{serve, ServiceConfig};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    serve(ServiceConfig {
        service_name: "shopping-carts",
        port: 4005,
        prefix: SHOPPING_CARTS_PREFIX,
        date_field: SHOPPING_CARTS_DATE_FIELD,
        list_path: "/shopping-carts",
        item_path: "/shopping-cart/{id}",
        fetch_all: db::fetch_all_shopping_carts,
        fetch_one: db::fetch_shopping_cart,
    })
    .await
}
