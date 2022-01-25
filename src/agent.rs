// Agent collecting metrics and logs.

use async_std::prelude::*;
use std::collections::HashMap;

// System metrics
use sysinfo::{ProcessorExt, SystemExt};

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct Metric {
    pub name: String,
    pub value: f64,
    pub tags: HashMap<String, String>,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub enum LogLevel {
    Debug,
    Notice,
    Info,
    Warning,
    Error,
    Fatal,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct LogLine {
    pub line: String,
    pub level: Option<LogLevel>,
    pub created_at: Option<String>,
    pub tags: HashMap<String, String>,
}

pub async fn launch() {
    // Custom metrics collector
    let custom_metrics = tokio::task::spawn(async move {
        let sock = async_std::net::UdpSocket::bind("0.0.0.0:1337")
            .await
            .unwrap();

        println!("Listening for custom metrics on UDP port 1337");

        loop {
            let mut buf = vec![0u8; 512];
            let (n, peer) = sock.recv_from(&mut buf).await.unwrap();

            println!("Received {} bytes from peer {}", n, peer);

            tokio::task::spawn(async move {
                process_metric(buf).await;
            });
        }
    });

    // System metrics (system.*)
    let system_metrics = tokio::task::spawn(async move {
        let mut system = sysinfo::System::new_all();
        let duration = tokio::time::Duration::from_millis(1_000);
        let hostname = gethostname::gethostname();
        let tags = HashMap::from([(
            "hostname".to_string(),
            hostname.into_string().unwrap_or("unknown".to_string()),
        )]);

        println!("Starting collection of system metrics");

        loop {
            tokio::time::sleep(duration).await;
            system.refresh_all();

            // Memory
            let metrics = vec![
                Metric {
                    name: "system.mem.total".to_string(),
                    value: system.total_memory() as f64,
                    tags: tags.clone(),
                },
                Metric {
                    name: "system.mem.used".to_string(),
                    value: system.used_memory() as f64,
                    tags: tags.clone(),
                },
                Metric {
                    name: "system.cpu.utilization".to_string(),
                    value: system
                        .processors()
                        .iter()
                        .map(|cpu| cpu.cpu_usage() as f64)
                        .sum::<f64>()
                        / system.processors().len() as f64,
                    tags: tags.clone(),
                },
            ];

            tokio::task::spawn(async move {
                send_metrics(metrics).await;
            });
        }
    });

    // Logs collector.
    // Add your log files here!
    let log_files = vec![
        "/var/log/postgresql/postgresql-12-main.log",
        "/some/random/file.log",
    ];

    for log_file in log_files {
        tokio::task::spawn(async move {
            let mut offset = 0u64;
            let mut last_modified: Option<std::time::SystemTime> = None;
            let duration = tokio::time::Duration::from_millis(1_000);

            println!("Starting collection of logs from {}", log_file);

            loop {
                match async_std::fs::File::open(log_file).await {
                    Ok(f) => {
                        // Should be okay most places (except maybe if ZFS is used...)
                        let modified = f.metadata().await.unwrap().modified().unwrap();
                        let mut buf = async_std::io::BufReader::new(f);
                        match buf.seek(async_std::io::SeekFrom::Start(offset)).await {
                            Ok(_pos) => (),
                            Err(err) => {
                                println!("Could not seek to: {}, err: {}", offset, err);
                                offset = 0;
                            }
                        };

                        loop {
                            let mut line = String::new();
                            let n = match buf.read_line(&mut line).await {
                                Ok(n) => n,
                                Err(err) => {
                                    println!("Error reading file: {}", err);
                                    0
                                }
                            };

                            if n != 0 {
                                tokio::task::spawn(async move {
                                    send_logs(&vec![LogLine {
                                        line: line,
                                        level: None,
                                        created_at: None,
                                        tags: HashMap::from([(
                                            "filename".to_string(),
                                            log_file.to_string(),
                                        )]),
                                    }])
                                    .await;
                                });
                                offset += n as u64;

                                // File appended to
                                last_modified = Some(modified);
                            } else {
                                match last_modified {
                                    Some(timestamp) => {
                                        if timestamp < modified {
                                            // File has been rotated
                                            offset = 0;
                                            last_modified = Some(modified);
                                        }
                                    }

                                    None => {
                                        last_modified = Some(modified);
                                    }
                                }
                                break;
                            }
                        }
                    }

                    Err(err) => {
                        println!("Can't open file {}: {}", log_file, err);
                        break;
                    }
                };

                tokio::time::sleep(duration).await;
            }
        });
    }

    custom_metrics.await.unwrap();
    system_metrics.await.unwrap();
}

async fn process_metric(buf: Vec<u8>) {
    let custom_metric = String::from_utf8_lossy(&buf).trim().to_string();

    let parts: Vec<_> = custom_metric.split("|").map(|x| x.to_string()).collect();
    let name_value: Vec<_> = parts[0].split(":").map(|x| x.to_string()).collect();

    match name_value.as_slice() {
        [name, value] => {
            let name = name.to_string();
            let value = value.parse::<f64>().unwrap_or(0.0);

            send_metrics(vec![Metric {
                name: name,
                value: value,
                tags: HashMap::new(),
            }])
            .await;
        }
        _ => (),
    };
}

async fn send_metrics(metrics: Vec<Metric>) {
    let api =
        std::env::var("METRICSCAT_API_URL").unwrap_or("http://localhost:8000/api".to_string());
    let client = reqwest::Client::new();
    //
    // FIXME: Add suport for mTLS later.
    //
    // let client = match api.starts_with("https") {
    //     // Implementing mTLS. So far this is not working, not sure why,
    //     // I think something to do with the certificate being incorrectly formatted.
    //     true => {
    //         let cert_path =
    //             std::env::var("METRICSCAT_CERTIFICATE_PATH").unwrap_or("firefox.pem".to_string());
    //         match async_std::fs::File::open(&cert_path).await {
    //             Ok(mut f) => {
    //                 let mut buf = Vec::new();
    //                 f.read_to_end(&mut buf).await.unwrap();
    //                 match reqwest::Identity::from_pem(&buf) {
    //                     Ok(identity) => reqwest::ClientBuilder::new()
    //                         .identity(identity)
    //                         .build()
    //                         .unwrap_or(reqwest::Client::new()),
    //                     Err(err) => {
    //                         println!("Certificate is corrupt: {}", err);
    //                         reqwest::Client::new()
    //                     }
    //                 }
    //             }
    //             Err(err) => {
    //                 println!("Could not open certificate path: {}", err);
    //                 reqwest::Client::new()
    //             }
    //         }
    //     }

    //     // Basic HTTP, use inside a private network only.
    //     false => reqwest::Client::new(),
    // };

    let url = format!("{}/metrics", api);

    let response = client
        .post(&url)
        .body(json!(metrics).to_string())
        .send()
        .await
        .unwrap();

    if response.status() != reqwest::StatusCode::OK {
        println!(
            "Error submitting metrics, error: {}, body: {}",
            response.status(),
            response.text().await.unwrap()
        );
    }
}

async fn send_logs(logs: &Vec<LogLine>) {
    let api =
        std::env::var("METRICSCAT_API_URL").unwrap_or("http://localhost:8000/api".to_string());
    let url = format!("{}/logs", api);
    let client = reqwest::Client::new();

    let response = client
        .post(&url)
        .body(json!(logs).to_string())
        .send()
        .await
        .unwrap();

    if response.status() != reqwest::StatusCode::OK {
        println!(
            "Error submitting logs, error: {}, body: {}",
            response.status(),
            response.text().await.unwrap()
        );
    }
}
