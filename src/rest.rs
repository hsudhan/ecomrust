//! Shared actix-web helpers for the two REST services: query validation
//! (specs.md #6: API validation and error handling) and the paginated
//! response envelope.

use serde::Deserialize;
use serde_json::{json, Value};

use crate::error::{AppError, AppResult};
use crate::models::{PageParams, SortDir};

#[derive(Debug, Deserialize)]
pub struct ListQuery {
    pub page: Option<i64>,
    pub page_size: Option<i64>,
    pub sort: Option<String>,
    pub order: Option<String>,
}

pub fn parse_params(q: &ListQuery) -> AppResult<PageParams> {
    let page = q.page.unwrap_or(1);
    if page < 1 {
        return Err(AppError::BadRequest("page must be >= 1".to_string()));
    }
    let page_size = q.page_size.unwrap_or(50);
    if !(1..=200).contains(&page_size) {
        return Err(AppError::BadRequest(
            "page_size must be between 1 and 200".to_string(),
        ));
    }
    let dir = match q.order.as_deref() {
        None | Some("asc") => SortDir::Asc,
        Some("desc") => SortDir::Desc,
        Some(other) => {
            return Err(AppError::BadRequest(format!(
                "invalid order '{other}'; allowed: asc, desc"
            )))
        }
    };
    Ok(PageParams {
        page,
        page_size,
        sort: q.sort.clone().unwrap_or_else(|| "id".to_string()),
        dir,
    })
}

pub fn list_body(rows: Vec<Value>, total: i64, params: &PageParams) -> Value {
    json!({
        "data": rows,
        "page": params.page,
        "page_size": params.page_size,
        "total_records": total,
        "total_pages": ((total + params.page_size - 1) / params.page_size).max(1),
        "sort": params.sort,
        "order": match params.dir {
            SortDir::Asc => "asc",
            SortDir::Desc => "desc",
        },
    })
}
