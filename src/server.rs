use crate::agent;
use rocket::serde::json::Json;
use rocket::State;
use sqlx::PgPool;
use std::collections::BTreeSet;
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

#[derive(Debug, PartialEq, FromFormField)]
pub enum Function {
    Min,
    Avg,
    Max,
    Sum,
    P50,
    P75,
    P99,
    P9999,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct MetricPoint {
    value: f64,
    recorded_at: String,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct LogLine {
    line: String,
    recorded_at: String,
    offset: i64,
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

    let metrics_rows: Vec<(i64,)> = sqlx::query_as(
        "INSERT INTO metrics 
    	(metric_name_id, value, recorded_at) 
    	SELECT unnest($1), unnest($2), $3 
    	RETURNING id",
    )
    .bind(&ids)
    .bind(values)
    .bind(now)
    .fetch_all(pool.inner())
    .await
    .unwrap();

    let (mut tag_names, mut tag_values) = (BTreeSet::new(), BTreeSet::new());
    metrics.iter().for_each(|x| {
        let tnames: Vec<_> = x.tags.keys().map(|x| x.clone()).collect();
        let tvalues: Vec<_> = x.tags.values().map(|x| x.clone()).collect();

        tag_names.extend(tnames);
        tag_values.extend(tvalues);
    });

    let tag_names_rows: Vec<(i64, String)> =
        sqlx::query_as("SELECT id, name FROM tag_names WHERE name = ANY($1)")
            .bind(&tag_names.clone().into_iter().collect::<Vec<_>>())
            .fetch_all(pool.inner())
            .await
            .unwrap();

    let tag_values_rows: Vec<(i64, String)> =
        sqlx::query_as("SELECT id, value FROM tag_values WHERE value = ANY($1)")
            .bind(&tag_values.clone().into_iter().collect::<Vec<_>>())
            .fetch_all(pool.inner())
            .await
            .unwrap();

    let (mut tag_names_map, mut tag_values_map) = (
        std::collections::HashMap::new(),
        std::collections::HashMap::new(),
    );

    for x in &tag_names_rows {
        tag_names_map.insert(&x.1, x.0);
    }

    for x in &tag_values_rows {
        tag_values_map.insert(&x.1, x.0);
    }

    for tag_name in &tag_names {
        if !tag_names_map.contains_key(tag_name) {
            let row: (i64,) = sqlx::query_as(
                "INSERT INTO tag_names (name) VALUES ($1) ON CONFLICT (name) DO NOTHING RETURNING id"
            )
            .bind(tag_name)
            .fetch_one(pool.inner())
            .await
            .unwrap();

            tag_names_map.insert(&tag_name, row.0);
        }
    }

    for tag_value in &tag_values {
        if !tag_values_map.contains_key(tag_value) {
            let row: (i64,) = sqlx::query_as(
                "INSERT INTO tag_values (value) VALUES ($1) ON CONFLICT (value) DO NOTHING RETURNING id"
            )
            .bind(tag_value)
            .fetch_one(pool.inner())
            .await
            .unwrap();

            tag_values_map.insert(&tag_value, row.0);
        }
    }

    let (mut metric_ids, mut tag_name_ids, mut tag_value_ids) =
        (Vec::new(), Vec::new(), Vec::new());

    for (idx, metric) in metrics.iter().enumerate() {
        let id = metrics_rows[idx].0;

        for (tag_name, tag_value) in &metric.tags {
            tag_name_ids.push(tag_names_map[&tag_name]);
            tag_value_ids.push(tag_values_map[&tag_value]);
            metric_ids.push(id);
        }
    }

    let _rows: Vec<(i64,)> = sqlx::query_as(
        "INSERT INTO metric_tags (metric_id, tag_name_id, tag_value_id, recorded_at)
        SELECT unnest($1), unnest($2), unnest($3), $4
        RETURNING id",
    )
    .bind(&metric_ids)
    .bind(&tag_name_ids)
    .bind(&tag_value_ids)
    .bind(now)
    .fetch_all(pool.inner())
    .await
    .unwrap();
}

#[get("/api/metrics?<name>&<interval>&<range_start>&<range_end>&<function>")]
pub async fn api_metrics_get(
    name: &str,
    interval: Option<Interval>,
    range_start: Option<&str>,
    range_end: Option<&str>,
    function: Option<Function>,
    pool: &State<PgPool>,
) -> Json<Vec<MetricPoint>> {
    let now = chrono::offset::Utc::now().naive_utc();
    let interval = interval.unwrap_or(Interval::Minute1);

    let range_start = match range_start {
        Some(range_start) => range_start
            .parse::<chrono::naive::NaiveDateTime>()
            .unwrap_or(now),
        None => match interval {
            Interval::Minute1 => now - chrono::Duration::minutes(1),
            Interval::Minute5 => now - chrono::Duration::minutes(5),
            Interval::Minute15 => now - chrono::Duration::minutes(15),
            Interval::Hour1 => now - chrono::Duration::hours(1),
            Interval::Hour4 => now - chrono::Duration::hours(4),
            Interval::Day => now - chrono::Duration::days(1),
        },
    };

    let truncate_to = match interval {
        Interval::Minute1 => ("second", 1),
        Interval::Minute5 => ("second", 1),
        Interval::Minute15 => ("second", 15),
        Interval::Hour1 => ("second", 30),
        Interval::Hour4 => ("minute", 1),
        Interval::Day => ("minute", 30),
    };

    let range_end = match range_end {
        Some(range_end) => range_end
            .parse::<chrono::naive::NaiveDateTime>()
            .unwrap_or(now),
        None => now,
    };

    let function = match function.unwrap_or(Function::Avg) {
        Function::Avg => "AVG",
        Function::Max => "MAX",
        Function::Min => "MIN",
        Function::Sum => "SUM",
        _ => "AVG",
    };

    let rows: Vec<(f64, chrono::naive::NaiveDateTime)> = sqlx::query_as(&format!(
        "SELECT
            {}(A.value) AS value,
            DATE_TRUNC($4, A.recorded_at) AS recorded_at 
    	FROM metrics A
    	INNER JOIN metric_names B
    	ON A.metric_name_id = B.id
    	WHERE B.name = $1
    	AND recorded_at > $2
    	AND recorded_at < $3
        GROUP BY recorded_at
    	ORDER BY recorded_at ASC",
        function
    ))
    .bind(name)
    .bind(range_start)
    .bind(range_end)
    .bind(truncate_to.0)
    .fetch_all(pool.inner())
    .await
    .unwrap();

    let result: Vec<MetricPoint> = rows
        .iter()
        .map(|x| MetricPoint {
            value: x.0,
            recorded_at: x.1.to_string(),
        })
        .collect();

    Json(result)
}

#[post("/api/logs", data = "<log_lines>")]
pub async fn api_logs_post(log_lines: Json<Vec<agent::LogLine>>, pool: &State<PgPool>) {
    if log_lines.len() == 0 {
        return;
    }

    // Grab the lines.
    let lines: Vec<_> = log_lines
        .iter()
        .map(|x| x.line.trim_end().to_string())
        .collect();

    // Put everything into one query for performance.

    let (mut log_parts, mut separator_parts, mut query_parts) =
        (Vec::new(), Vec::new(), Vec::new());

    // Counts the Pg placeholders, e.g. $1, $2, etc.
    let mut c = 1;
    for (idx, line) in lines.iter().enumerate() {
        // TODO: implement various tokenizers
        let parts: Vec<_> = line.split_whitespace().collect();
        let separators: Vec<_> = parts.iter().map(|_x| " ".to_string()).collect();

        log_parts.push(parts);
        separator_parts.push(separators);
        query_parts.push(format!(
            "(${}, ${}, TIMEZONE('UTC', NOW()))",
            idx + c,
            idx + c + 1
        ));

        c += 1;
    }

    // Build the query from query_parts
    let q = format!(
        "INSERT INTO logs (log_parts, separators, created_at) VALUES {} RETURNING id",
        query_parts.join(", ")
    );
    let mut query = sqlx::query_as(&q);

    for (idx, lp) in log_parts.iter().enumerate() {
        query = query
            // log_parts
            .bind(lp)
            // separators
            .bind(separator_parts[idx].clone());
    }

    // Execute this
    let _rows: Vec<(i64,)> = query.fetch_all(pool.inner()).await.unwrap();
}

#[get("/api/logs?<offset>")]
pub async fn api_logs_get(offset: Option<i64>, pool: &State<PgPool>) -> Json<Vec<LogLine>> {
    let offset = offset.unwrap_or(0);

    let rows: Vec<(i64, Vec<String>, Vec<String>, chrono::naive::NaiveDateTime)> = sqlx::query_as(
        "SELECT id, log_parts, separators, created_at FROM logs WHERE id > $1 ORDER BY id DESC LIMIT 25",
    )
    .bind(offset)
    .fetch_all(pool.inner())
    .await
    .unwrap();

    let result: Vec<LogLine> = rows
        .iter()
        .map(|x| {
            let mut line = String::new();

            for (idx, part) in x.1.iter().enumerate() {
                let sep = x.2.get(idx).unwrap_or(&"".to_string()).clone();
                line += &(part.clone() + &sep);
            }

            LogLine {
                line: line,
                recorded_at: x.3.to_string(),
                offset: x.0,
            }
        })
        .collect();

    Json(result)
}

#[get("/api/logs/search?<term>&<created_at>")]
pub async fn api_logs_search_get(
    term: String,
    created_at: Option<String>,
    pool: &State<PgPool>,
) -> Json<Vec<LogLine>> {
    let now = chrono::offset::Utc::now().naive_utc();

    let created_at = match created_at {
        Some(created_at) => {
            chrono::naive::NaiveDateTime::parse_from_str(&created_at, "%Y-%m-%dT%H:%M:%S")
                .unwrap_or(now)
        }
        None => now,
    };

    // TODO: implement various tokenizers
    let term: Vec<_> = term.split_whitespace().collect();

    let rows: Vec<(i64, Vec<String>, Vec<String>, chrono::naive::NaiveDateTime)> = sqlx::query_as(
        "SELECT id, log_parts, separators, created_at
        FROM logs
        WHERE log_parts @> $1::VARCHAR[]
        AND created_at < $2
        AND created_at > $2 - INTERVAL '5 minute'
        ORDER BY created_at
        LIMIT 100",
    )
    .bind(&term)
    .bind(created_at)
    .fetch_all(pool.inner())
    .await
    .unwrap();

    let result: Vec<_> = rows
        .iter()
        .map(|x| {
            let mut line = String::new();

            for (idx, part) in x.1.iter().enumerate() {
                let sep = x.2.get(idx).unwrap_or(&"".to_string()).clone();
                line += &(part.clone() + &sep);
            }

            LogLine {
                line: line,
                recorded_at: x.3.to_string(),
                offset: x.0,
            }
        })
        .collect();

    Json(result)
}
