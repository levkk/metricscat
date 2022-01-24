use crate::agent;
use rocket::serde::json::Json;
use rocket::State;
use sqlx::PgPool;
// use chrono::prelude::*;

#[derive(Debug, PartialEq, FromFormField)]
pub enum Interval {
    Minute1,
    Minute5,
    Minute15,
    Hour1,
    Hour4,
    Day,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct MetricPoint {
    value: f64,
    recorded_at: String,
}

#[get("/")]
pub fn index() -> &'static str {
    "Hello, world!"
}

#[post("/api/metrics", data = "<metrics>")]
pub async fn api_metrics_post(metrics: Json<Vec<agent::Metric>>, pool: &State<PgPool>) {
    let names: Vec<_> = metrics.iter().map(|x| x.name.clone()).collect();
    let rows: Vec<(i64, String)> = sqlx::query_as(
        "SELECT id, name 
        	FROM metric_names 
        	WHERE name = ANY($1)",
    )
    .bind(&names)
    .fetch_all(pool.inner())
    .await
    .unwrap_or(Vec::new());

    let mut map = std::collections::HashMap::new();

    for row in &rows {
        map.insert(&row.1, row.0);
    }

    for name in &names {
        if !map.contains_key(name) {
            let row: (i64,) = sqlx::query_as(
                "INSERT INTO
            		metric_names (name) 
            		VALUES ($1) 
            		ON CONFLICT (name) DO NOTHING
            	RETURNING id",
            )
            .bind(name)
            .fetch_one(pool.inner())
            .await
            .unwrap_or((0,));

            map.insert(name, row.0);
        }
    }

    let ids: Vec<i64> = metrics.iter().map(|x| map[&x.name]).collect();
    let values: Vec<f64> = metrics.iter().map(|x| x.value).collect();
    let now = chrono::offset::Utc::now().naive_utc();

    let _rows: Vec<(i64,)> = sqlx::query_as(
        "INSERT INTO metrics 
    	(metric_id, value, recorded_at) 
    	SELECT unnest($1), unnest($2), $3 
    	RETURNING id",
    )
    .bind(ids)
    .bind(values)
    .bind(now)
    .fetch_all(pool.inner())
    .await
    .unwrap();
}

#[get("/api/metrics?<name>&<interval>&<range_start>&<range_end>")]
pub async fn api_metrics_get(
    name: &str,
    interval: Option<Interval>,
    range_start: Option<&str>,
    range_end: Option<&str>,
    pool: &State<PgPool>,
) -> Json<Vec<MetricPoint>> {
    let now = chrono::offset::Utc::now().naive_utc();

    let range_start = match range_start {
        Some(range_start) => range_start
            .parse::<chrono::naive::NaiveDateTime>()
            .unwrap_or(now),
        None => match interval.unwrap_or(Interval::Minute1) {
            Interval::Minute1 => now - chrono::Duration::minutes(1),
            Interval::Minute5 => now - chrono::Duration::minutes(5),
            Interval::Minute15 => now - chrono::Duration::minutes(15),
            Interval::Hour1 => now - chrono::Duration::hours(1),
            Interval::Hour4 => now - chrono::Duration::hours(4),
            Interval::Day => now - chrono::Duration::days(1),
        },
    };

    let range_end = match range_end {
        Some(range_end) => range_end
            .parse::<chrono::naive::NaiveDateTime>()
            .unwrap_or(now),
        None => now,
    };

    let rows: Vec<(i64, f64, chrono::naive::NaiveDateTime)> = sqlx::query_as(
        "SELECT A.id, A.value, A.recorded_at
    	FROM metrics A
    	INNER JOIN metric_names B
    	ON A.metric_id = B.id
    	WHERE B.name = $1
    	AND recorded_at > $2
    	AND recorded_at < $3
    	ORDER BY A.recorded_at ASC",
    )
    .bind(name)
    .bind(range_start)
    .bind(range_end)
    .fetch_all(pool.inner())
    .await
    .unwrap();

    let result: Vec<MetricPoint> = rows
        .iter()
        .map(|x| MetricPoint {
            value: x.1,
            recorded_at: x.2.to_string(),
        })
        .collect();

    Json(result)
}
