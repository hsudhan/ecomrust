//! Cache loader (specs.md #1): fetches `orders` and `shipments` from
//! PostgreSQL and loads them into Redis under the `orders:` / `shipments:`
//! key prefixes.
//!
//! Run: cargo run --bin cache_loader

use ecomrust::cache::{Cache, DEFAULT_REDIS_URL};
use ecomrust::db::{self, DEFAULT_DATABASE_URL};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let db_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| DEFAULT_DATABASE_URL.to_string());
    let redis_url = std::env::var("REDIS_URL").unwrap_or_else(|_| DEFAULT_REDIS_URL.to_string());

    println!("cache_loader: connecting to PostgreSQL ({db_url})");
    let pool = db::connect(&db_url).await?;
    println!("cache_loader: connecting to Redis ({redis_url})");
    let cache = Cache::connect(&redis_url).await?;

    let orders = db::fetch_all_orders(&pool).await?;
    println!("cache_loader: fetched {} orders from PostgreSQL", orders.len());
    let n = cache.load_orders(orders).await?;
    println!("cache_loader: loaded {n} orders into Redis (orders:*)");

    let shipments = db::fetch_all_shipments(&pool).await?;
    println!(
        "cache_loader: fetched {} shipments from PostgreSQL",
        shipments.len()
    );
    let n = cache.load_shipments(shipments).await?;
    println!("cache_loader: loaded {n} shipments into Redis (shipments:*)");

    println!("cache_loader: done");
    Ok(())
}
