// Agent collecting metrics and logs.

use std::collections::HashMap;

// System metrics
use sysinfo::{ProcessorExt, SystemExt};

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct Metric {
    pub name: String,
    pub value: f64,
    pub tags: HashMap<String, String>,
}

pub async fn launch() {
    // Custom metrics collector
    let custom_metrics = tokio::task::spawn(async move {
        let sock = async_std::net::UdpSocket::bind("0.0.0.0:1337")
            .await
            .unwrap();

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
    let client = reqwest::Client::new();

    client
        .post("http://localhost:8000/api/metrics")
        .body(json!(metrics).to_string())
        .send()
        .await
        .unwrap();
}
