//! PostgreSQL access (primary storage). Used by the cache loader;
//! runtime API reads are served from Redis (specs.md design requirement #1).
//! NUMERIC -> text casts preserve exact decimal precision.

use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;

use crate::error::AppResult;
use crate::models::{OrderRow, ShipmentRow};

pub const DEFAULT_DATABASE_URL: &str = "postgresql://harir@localhost:5432/ecomdb";

pub async fn connect(database_url: &str) -> AppResult<PgPool> {
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(database_url)
        .await?;
    Ok(pool)
}

pub async fn fetch_all_orders(pool: &PgPool) -> AppResult<Vec<OrderRow>> {
    let rows = sqlx::query_as::<_, OrderRow>(
        r#"SELECT id, customer_id, order_date, status,
                  total_amount::text AS total_amount,
                  created_at, updated_at
           FROM ecommerce."order"
           ORDER BY id"#,
    )
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

pub async fn fetch_all_shipments(pool: &PgPool) -> AppResult<Vec<ShipmentRow>> {
    let rows = sqlx::query_as::<_, ShipmentRow>(
        r#"SELECT id, order_id, shipment_date, shipment_status,
                  shipment_cost::text AS shipment_cost,
                  created_at, updated_at
           FROM ecommerce.shipment
           ORDER BY id"#,
    )
    .fetch_all(pool)
    .await?;
    Ok(rows)
}
