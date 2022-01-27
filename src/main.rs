#[macro_use]
extern crate rocket;

#[macro_use]
extern crate serde_json;

extern crate async_std;
extern crate gethostname;
extern crate rocket_cors;

mod agent;
mod server;

#[tokio::main]
async fn main() {
    let args: Vec<_> = std::env::args().collect();
    let usage = "Usage: metricscat <server|agent>";

    if args.len() < 2 {
        println!("{}", usage);
        return;
    }

    let role = &args[1];

    match role.as_ref() {
        "server" => {
            let db = sqlx::postgres::PgPoolOptions::new()
                .max_connections(16)
                .connect(
                    &std::env::var("METRICSCAT_DATABASE_URL")
                        .unwrap_or("postgres:///metrics".to_string()),
                )
                .await
                .unwrap();
            let cors = rocket_cors::CorsOptions {
                ..Default::default()
            }
            .to_cors()
            .unwrap();

            match rocket::build()
                .mount(
                    "/",
                    routes![
                        server::index,
                        server::api_metrics_post,
                        server::api_metrics_get,
                        server::api_logs_post,
                        server::api_logs_get,
                        server::api_logs_search_get,
                    ],
                )
                .manage(db)
                .attach(cors)
                .ignite()
                .await
            {
                Ok(r) => r.launch().await.unwrap(),
                Err(err) => panic!("Rocket error: {}", err),
            };
        }
        "agent" => {
            agent::launch().await;
        }
        _ => {
            println!("{}", usage);
        }
    };
}
