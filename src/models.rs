//! Row models (PostgreSQL) and JSON models (Redis cache + REST payloads).
//! NUMERIC columns are cast to text in SQL so amounts keep exact precision;
//! timestamptz columns map to chrono and render as RFC 3339 strings.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// ------------------------------------------------------------- orders -----

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct OrderRow {
    pub id: i64,
    pub customer_id: i32,
    pub order_date: DateTime<Utc>,
    pub status: String,
    pub total_amount: String, // NUMERIC::text
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderJson {
    pub id: i64,
    pub customer_id: i32,
    pub order_date: String,
    pub status: String,
    pub total_amount: String,
    pub created_at: String,
    pub updated_at: String,
}

impl From<OrderRow> for OrderJson {
    fn from(r: OrderRow) -> Self {
        Self {
            id: r.id,
            customer_id: r.customer_id,
            order_date: r.order_date.to_rfc3339(),
            status: r.status,
            total_amount: r.total_amount,
            created_at: r.created_at.to_rfc3339(),
            updated_at: r.updated_at.to_rfc3339(),
        }
    }
}

// ---------------------------------------------------------- shipments -----

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ShipmentRow {
    pub id: i64,
    pub order_id: i32,
    pub shipment_date: DateTime<Utc>,
    pub shipment_status: String,
    pub shipment_cost: String, // NUMERIC::text
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShipmentJson {
    pub id: i64,
    pub order_id: i32,
    pub shipment_date: String,
    pub shipment_status: String,
    pub shipment_cost: String,
    pub created_at: String,
    pub updated_at: String,
}

impl From<ShipmentRow> for ShipmentJson {
    fn from(r: ShipmentRow) -> Self {
        Self {
            id: r.id,
            order_id: r.order_id,
            shipment_date: r.shipment_date.to_rfc3339(),
            shipment_status: r.shipment_status,
            shipment_cost: r.shipment_cost,
            created_at: r.created_at.to_rfc3339(),
            updated_at: r.updated_at.to_rfc3339(),
        }
    }
}

// --------------------------------------------------------- pagination -----

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortDir {
    Asc,
    Desc,
}

#[derive(Debug, Clone)]
pub struct PageParams {
    pub page: i64,      // 1-based
    pub page_size: i64, // 1..=200
    pub sort: String,   // validated against the entity's allow-list
    pub dir: SortDir,
}

impl Default for PageParams {
    fn default() -> Self {
        Self {
            page: 1,
            page_size: 50,
            sort: "id".to_string(),
            dir: SortDir::Asc,
        }
    }
}
