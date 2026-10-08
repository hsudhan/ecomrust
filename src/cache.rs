//! Redis cache layer (specs.md #1, #5).
//!
//! Key layout per entity (prefix = "orders" | "shipments" | "users" | ...):
//!   {prefix}:{id}            -> JSON string of the row        (GET by id)
//!   {prefix}:index:id        -> ZSET score=id, member=id      (sort by id)
//!   {prefix}:index:{column}  -> one ZSET per sortable column, encoded by
//!                               the column's SortKind (models.rs):
//!      Num  -> score=numeric value (NUMERIC-as-text parsed), member=id
//!      Date -> score=epoch ms of the RFC 3339 value,         member=id
//!      Str  -> score=0, member="{value}:{id:020}"  (lexicographic index:
//!              value-then-id ordering, read with ZRANGEBYLEX family)
//!
//! List pages are resolved with ZRANGE/ZREVRANGE (numeric indexes) or
//! ZRANGEBYLEX/ZREVRANGEBYLEX ... LIMIT (string indexes), then a single
//! MGET fetches the row documents — no per-row round trips (claude.md
//! rule #5: avoid N+1 patterns).
//!
//! Reads are cache-aside: the REST tier serves from Redis first and, on a
//! miss (single doc absent or index never loaded), fetches from PostgreSQL
//! and writes the result back through `load` before responding. Full loads
//! (`reload`) delete the entity's index keys before rebuilding, so rows
//! removed from PostgreSQL never linger in sorted pages.
//!
//! Note: Redis logical databases are numeric (0-15); "Database: ecomdb" from
//! specs.md is satisfied by connecting to DB 0 and namespacing every key with
//! the entity prefixes (`orders:`, `shipments:`, `users:`, ...).

use chrono::DateTime;
use redis::aio::MultiplexedConnection;
use redis::AsyncCommands;
use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::error::{AppError, AppResult};
use crate::models::{CacheDoc, PageParams, SortDir, SortField, SortKind};

pub const DEFAULT_REDIS_URL: &str = "redis://localhost:6379/0";

pub const ORDERS_PREFIX: &str = "orders";
pub const SHIPMENTS_PREFIX: &str = "shipments";
pub const USERS_PREFIX: &str = "users";
pub const LOGINS_PREFIX: &str = "logins";
pub const SHOPPING_CARTS_PREFIX: &str = "shopping-carts";
pub const PAYMENT_INFOS_PREFIX: &str = "payment-infos";
pub const PAYMENTS_PREFIX: &str = "payments";
pub const SHIPMENT_TRACKINGS_PREFIX: &str = "shipment-trackings";

/// Sortable columns per entity (excluding `id`, which is always sortable).
/// Orders and shipments: every column. Other entities: their date column.
pub const ORDERS_SORT_FIELDS: &[SortField] = &[
    SortField { name: "customer_id", kind: SortKind::Num },
    SortField { name: "order_date", kind: SortKind::Date },
    SortField { name: "status", kind: SortKind::Str },
    SortField { name: "total_amount", kind: SortKind::Num },
    SortField { name: "created_at", kind: SortKind::Date },
    SortField { name: "updated_at", kind: SortKind::Date },
];
pub const SHIPMENTS_SORT_FIELDS: &[SortField] = &[
    SortField { name: "order_id", kind: SortKind::Num },
    SortField { name: "shipment_date", kind: SortKind::Date },
    SortField { name: "shipment_status", kind: SortKind::Str },
    SortField { name: "shipment_cost", kind: SortKind::Num },
    SortField { name: "created_at", kind: SortKind::Date },
    SortField { name: "updated_at", kind: SortKind::Date },
];
pub const USERS_SORT_FIELDS: &[SortField] = &[SortField { name: "created_at", kind: SortKind::Date }];
pub const LOGINS_SORT_FIELDS: &[SortField] = &[SortField { name: "login_date", kind: SortKind::Date }];
pub const SHOPPING_CARTS_SORT_FIELDS: &[SortField] =
    &[SortField { name: "created_at", kind: SortKind::Date }];
pub const PAYMENT_INFOS_SORT_FIELDS: &[SortField] =
    &[SortField { name: "payment_date", kind: SortKind::Date }];
pub const PAYMENTS_SORT_FIELDS: &[SortField] =
    &[SortField { name: "payment_date", kind: SortKind::Date }];
pub const SHIPMENT_TRACKINGS_SORT_FIELDS: &[SortField] =
    &[SortField { name: "updated_at", kind: SortKind::Date }];

const PIPELINE_BATCH: usize = 5_000;

/// Numeric score for a Num sort field: JSON numbers as-is, NUMERIC-as-text
/// columns ("123.45") parsed.
fn json_num(json: &Value, field: &str) -> Option<f64> {
    match json.get(field) {
        Some(Value::Number(n)) => n.as_f64(),
        Some(Value::String(s)) => s.parse::<f64>().ok(),
        _ => None,
    }
}

/// Epoch-ms score for a Date sort field (RFC 3339 strings in the JSON docs).
fn json_date_epoch_ms(json: &Value, field: &str) -> Option<i64> {
    let s = json.get(field)?.as_str()?;
    Some(DateTime::parse_from_rfc3339(s).ok()?.timestamp_millis())
}

#[derive(Clone)]
pub struct Cache {
    conn: MultiplexedConnection,
}

impl Cache {
    pub async fn connect(redis_url: &str) -> AppResult<Self> {
        let client = redis::Client::open(redis_url)?;
        let conn = client.get_multiplexed_async_connection().await?;
        Ok(Self { conn })
    }

    // ---------------------------------------------------------- loading ---

    /// Upsert documents into Redis: one JSON key per row plus one member per
    /// sort index (id + every configured sort field), pipelined in batches.
    /// Used for single-doc write-back after a cache miss; full-table loads
    /// go through `reload` so stale index members are dropped first.
    pub async fn load(
        &self,
        prefix: &str,
        sort_fields: &[SortField],
        docs: &[CacheDoc],
    ) -> AppResult<usize> {
        let mut conn = self.conn.clone();
        for chunk in docs.chunks(PIPELINE_BATCH) {
            let mut pipe = redis::pipe();
            for doc in chunk {
                let id = doc.id.to_string();
                pipe.set(format!("{prefix}:{id}"), doc.json.clone()).ignore();
                pipe.zadd(format!("{prefix}:index:id"), id.clone(), doc.id as f64)
                    .ignore();
                let json: Value = serde_json::from_str(&doc.json)?;
                for field in sort_fields {
                    let key = format!("{prefix}:index:{}", field.name);
                    match field.kind {
                        SortKind::Num | SortKind::Date => {
                            let score = match field.kind {
                                SortKind::Num => json_num(&json, field.name),
                                _ => json_date_epoch_ms(&json, field.name).map(|ms| ms as f64),
                            }
                            .ok_or_else(|| {
                                AppError::BadRequest(format!(
                                    "{prefix} id {}: missing/invalid sort field '{}'",
                                    doc.id, field.name
                                ))
                            })?;
                            pipe.zadd(key, id.clone(), score).ignore();
                        }
                        SortKind::Str => {
                            let value = json.get(field.name).and_then(Value::as_str).unwrap_or("");
                            // Zero-padded id suffix: lexicographic tie-break
                            // matches numeric id order (stable pagination).
                            pipe.zadd(key, format!("{value}:{:020}", doc.id), 0.0).ignore();
                        }
                    }
                }
            }
            pipe.query_async::<()>(&mut conn).await?;
        }
        Ok(docs.len())
    }

    /// Full-table load: delete the entity's sort indexes, then rebuild them
    /// from `docs`. Rows deleted from PostgreSQL (or leftover members from
    /// an older index encoding) never linger in sorted pages. The per-row
    /// JSON keys are upserted, matching the previous overwrite semantics.
    pub async fn reload(
        &self,
        prefix: &str,
        sort_fields: &[SortField],
        docs: &[CacheDoc],
    ) -> AppResult<usize> {
        let mut conn = self.conn.clone();
        let mut del = redis::pipe();
        del.del(format!("{prefix}:index:id")).ignore();
        for field in sort_fields {
            del.del(format!("{prefix}:index:{}", field.name)).ignore();
        }
        del.query_async::<()>(&mut conn).await?;
        self.load(prefix, sort_fields, docs).await
    }

    /// True when the entity's id sort index exists, i.e. the cache has been
    /// loaded for this entity at least once.
    pub async fn index_exists(&self, prefix: &str) -> AppResult<bool> {
        let mut conn = self.conn.clone();
        Ok(conn.exists(format!("{prefix}:index:id")).await?)
    }

    // ---------------------------------------------------------- reading ---

    /// Paginated + sorted list from the cache.
    /// Returns (rows for the page, total_records).
    pub async fn list(
        &self,
        prefix: &str,
        sort_fields: &[SortField],
        params: &PageParams,
    ) -> AppResult<(Vec<Value>, i64)> {
        // API validation (specs.md #6): sort field restricted to known indexes.
        let sort_field = sort_fields.iter().find(|f| f.name == params.sort);
        if params.sort != "id" && sort_field.is_none() {
            let allowed: Vec<&str> = std::iter::once("id")
                .chain(sort_fields.iter().map(|f| f.name))
                .collect();
            return Err(AppError::BadRequest(format!(
                "invalid sort '{}'; allowed: {}",
                params.sort,
                allowed.join(", ")
            )));
        }
        if params.page < 1 {
            return Err(AppError::BadRequest("page must be >= 1".to_string()));
        }
        if !(1..=200).contains(&params.page_size) {
            return Err(AppError::BadRequest(
                "page_size must be between 1 and 200".to_string(),
            ));
        }

        let index_key = format!("{prefix}:index:{}", params.sort);
        let mut conn = self.conn.clone();

        let total: i64 = conn.zcard(&index_key).await?;
        let start = (params.page - 1) * params.page_size;
        let stop = start + params.page_size - 1;

        let ids: Vec<String> = match sort_field {
            // Lexicographic index: members are "{value}:{id:020}", scores all
            // 0, so pages come from the ZRANGEBYLEX family; the row id is the
            // zero-padded suffix after the last ':'.
            Some(SortField { kind: SortKind::Str, .. }) => {
                let members: Vec<String> = match params.dir {
                    SortDir::Asc => {
                        redis::cmd("ZRANGEBYLEX")
                            .arg(&index_key)
                            .arg("-")
                            .arg("+")
                            .arg("LIMIT")
                            .arg(start)
                            .arg(params.page_size)
                            .query_async(&mut conn)
                            .await?
                    }
                    SortDir::Desc => {
                        redis::cmd("ZREVRANGEBYLEX")
                            .arg(&index_key)
                            .arg("+")
                            .arg("-")
                            .arg("LIMIT")
                            .arg(start)
                            .arg(params.page_size)
                            .query_async(&mut conn)
                            .await?
                    }
                };
                members
                    .iter()
                    .filter_map(|m| m.rsplit(':').next()?.parse::<i64>().ok())
                    .map(|id| id.to_string())
                    .collect()
            }
            _ => match params.dir {
                SortDir::Asc => conn.zrange(&index_key, start as isize, stop as isize).await?,
                SortDir::Desc => {
                    conn.zrevrange(&index_key, start as isize, stop as isize).await?
                }
            },
        };

        if ids.is_empty() {
            return Ok((Vec::new(), total));
        }

        let keys: Vec<String> = ids.iter().map(|id| format!("{prefix}:{id}")).collect();
        let docs: Vec<Option<String>> = conn.mget(&keys).await?;
        let rows = docs
            .into_iter()
            .flatten()
            .map(|s| serde_json::from_str(&s))
            .collect::<Result<Vec<Value>, _>>()?;
        Ok((rows, total))
    }

    /// Raw cached JSON for one row; `None` on cache miss. The REST tier
    /// serves the stored string verbatim (no re-serialization).
    pub async fn get_raw(&self, prefix: &str, id: i64) -> AppResult<Option<String>> {
        let mut conn = self.conn.clone();
        let doc: Option<String> = conn.get(format!("{prefix}:{id}")).await?;
        Ok(doc)
    }

    /// Single row by id from the cache, deserialized. 404s when absent.
    /// Used by the gRPC tier.
    pub async fn get<T: DeserializeOwned>(&self, prefix: &str, id: i64) -> AppResult<T> {
        match self.get_raw(prefix, id).await? {
            Some(s) => Ok(serde_json::from_str(&s)?),
            None => Err(AppError::NotFound(format!("{prefix} id {id} not in cache"))),
        }
    }
}
