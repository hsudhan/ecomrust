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

// ------------------------------------------------------------- users ------
// NOTE: the password column is deliberately never selected or cached —
// it stays inside PostgreSQL (matches the legacy Fastify API surface).

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct UserRow {
    pub id: i64,
    pub username: String,
    pub email: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserJson {
    pub id: i64,
    pub username: String,
    pub email: String,
    pub created_at: String,
    pub updated_at: String,
}

impl From<UserRow> for UserJson {
    fn from(r: UserRow) -> Self {
        Self {
            id: r.id,
            username: r.username,
            email: r.email,
            created_at: r.created_at.to_rfc3339(),
            updated_at: r.updated_at.to_rfc3339(),
        }
    }
}

// ------------------------------------------------------------ logins ------

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct LoginRow {
    pub id: i64,
    pub user_id: i32,
    pub login_date: DateTime<Utc>,
    pub ip_address: String,
    pub login_type: String,
    pub device_name: String,
    pub location: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginJson {
    pub id: i64,
    pub user_id: i32,
    pub login_date: String,
    pub ip_address: String,
    pub login_type: String,
    pub device_name: String,
    pub location: String,
}

impl From<LoginRow> for LoginJson {
    fn from(r: LoginRow) -> Self {
        Self {
            id: r.id,
            user_id: r.user_id,
            login_date: r.login_date.to_rfc3339(),
            ip_address: r.ip_address,
            login_type: r.login_type,
            device_name: r.device_name,
            location: r.location,
        }
    }
}

// ----------------------------------------------------- shopping carts -----

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ShoppingCartRow {
    pub id: i64,
    pub order_id: i32,
    pub customer_id: i32,
    pub product_id: i32,
    pub quantity: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShoppingCartJson {
    pub id: i64,
    pub order_id: i32,
    pub customer_id: i32,
    pub product_id: i32,
    pub quantity: i32,
    pub created_at: String,
    pub updated_at: String,
}

impl From<ShoppingCartRow> for ShoppingCartJson {
    fn from(r: ShoppingCartRow) -> Self {
        Self {
            id: r.id,
            order_id: r.order_id,
            customer_id: r.customer_id,
            product_id: r.product_id,
            quantity: r.quantity,
            created_at: r.created_at.to_rfc3339(),
            updated_at: r.updated_at.to_rfc3339(),
        }
    }
}

// ------------------------------------------------------- payment infos ----

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct PaymentInfoRow {
    pub id: i64,
    pub order_id: i32,
    pub payment_method: String,
    pub payment_amount: String, // NUMERIC::text
    pub payment_status: String,
    pub payment_date: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentInfoJson {
    pub id: i64,
    pub order_id: i32,
    pub payment_method: String,
    pub payment_amount: String,
    pub payment_status: String,
    pub payment_date: String,
    pub created_at: String,
    pub updated_at: String,
}

impl From<PaymentInfoRow> for PaymentInfoJson {
    fn from(r: PaymentInfoRow) -> Self {
        Self {
            id: r.id,
            order_id: r.order_id,
            payment_method: r.payment_method,
            payment_amount: r.payment_amount,
            payment_status: r.payment_status,
            payment_date: r.payment_date.to_rfc3339(),
            created_at: r.created_at.to_rfc3339(),
            updated_at: r.updated_at.to_rfc3339(),
        }
    }
}

// ------------------------------------------------------------ payments ----

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct PaymentRow {
    pub id: i64,
    pub order_id: i32,
    pub payment_date: DateTime<Utc>,
    pub payment_amount: String, // NUMERIC::text
    pub payment_status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentJson {
    pub id: i64,
    pub order_id: i32,
    pub payment_date: String,
    pub payment_amount: String,
    pub payment_status: String,
    pub created_at: String,
    pub updated_at: String,
}

impl From<PaymentRow> for PaymentJson {
    fn from(r: PaymentRow) -> Self {
        Self {
            id: r.id,
            order_id: r.order_id,
            payment_date: r.payment_date.to_rfc3339(),
            payment_amount: r.payment_amount,
            payment_status: r.payment_status,
            created_at: r.created_at.to_rfc3339(),
            updated_at: r.updated_at.to_rfc3339(),
        }
    }
}

// --------------------------------------------------- shipment trackings ---

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ShipmentTrackingRow {
    pub id: i64,
    pub shipment_id: i32,
    pub tracking_number: String,
    pub status: String,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShipmentTrackingJson {
    pub id: i64,
    pub shipment_id: i32,
    pub tracking_number: String,
    pub status: String,
    pub updated_at: String,
}

impl From<ShipmentTrackingRow> for ShipmentTrackingJson {
    fn from(r: ShipmentTrackingRow) -> Self {
        Self {
            id: r.id,
            shipment_id: r.shipment_id,
            tracking_number: r.tracking_number,
            status: r.status,
            updated_at: r.updated_at.to_rfc3339(),
        }
    }
}

// ---------------------------------------------------------- cache doc -----

/// A single cached document: the serialized JSON blob plus the metadata the
/// cache layer needs to maintain its sort indexes (id score + date score).
#[derive(Debug, Clone)]
pub struct CacheDoc {
    pub id: i64,
    pub date_epoch_ms: i64,
    pub json: String,
}

macro_rules! impl_cache_doc {
    ($row:ty, $json:ty, $date:ident) => {
        impl $row {
            /// Serialized doc + sort-index metadata for the Redis cache.
            pub fn cache_doc(&self) -> crate::error::AppResult<CacheDoc> {
                Ok(CacheDoc {
                    id: self.id,
                    date_epoch_ms: self.$date.timestamp_millis(),
                    json: serde_json::to_string(&<$json>::from(self.clone()))?,
                })
            }
        }
    };
}

impl_cache_doc!(OrderRow, OrderJson, order_date);
impl_cache_doc!(ShipmentRow, ShipmentJson, shipment_date);
impl_cache_doc!(UserRow, UserJson, created_at);
impl_cache_doc!(LoginRow, LoginJson, login_date);
impl_cache_doc!(ShoppingCartRow, ShoppingCartJson, created_at);
impl_cache_doc!(PaymentInfoRow, PaymentInfoJson, payment_date);
impl_cache_doc!(PaymentRow, PaymentJson, payment_date);
impl_cache_doc!(ShipmentTrackingRow, ShipmentTrackingJson, updated_at);

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
