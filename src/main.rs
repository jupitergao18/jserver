use std::io::Write;
use std::sync::Arc;

use chrono::Local;
use clap::Parser;
use serde_json::Value;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::RwLock;

mod handler;

#[tokio::main]
async fn main() {
    let args = Args::parse();

    env_logger::Builder::default()
        .filter_level(if args.debug {
            log::LevelFilter::Debug
        } else {
            log::LevelFilter::Info
        })
        .format(|buf, record| {
            writeln!(
                buf,
                "{} {} [{}] {}",
                Local::now().format("%Y-%m-%d %H:%M:%S%.3f"),
                buf.default_styled_level(record.level()),
                record.module_path().unwrap_or("<none>"),
                &record.args()
            )
        })
        .init();

    log::info!(
        "Server config: bind_address={}, db_path={}, public_path={}",
        args.bind_address,
        args.db_path,
        args.public_path
    );

    let mut db_file = match tokio::fs::File::open(&args.db_path).await {
        Ok(file) => file,
        Err(e) => match e.kind() {
            std::io::ErrorKind::NotFound => {
                log::warn!("Database file not found, creating...");
                let mut file = tokio::fs::File::create(&args.db_path)
                    .await
                    .expect("Error creating database file");
                file.write_all(b"{}")
                    .await
                    .expect("Error writing database file");
                match tokio::fs::File::open(&args.db_path).await {
                    Ok(file) => file,
                    Err(e) => {
                        log::error!("Error opening database file after create: {}", e);
                        panic!()
                    }
                }
            }
            _ => {
                log::error!("Error opening database file: {}", e);
                panic!()
            }
        },
    };

    let mut db_content = String::new();
    match db_file.read_to_string(&mut db_content).await {
        Ok(_) => log::info!("Database file loaded"),
        Err(e) => {
            log::error!("Error reading database file: {}", e);
            panic!()
        }
    };
    drop(db_file);

    let db_value = match serde_json::from_str::<serde_json::Value>(&db_content) {
        Ok(v) => {
            log::info!("Database file parsed");
            v
        }
        Err(e) => {
            log::error!("Error parsing database file: {}", e);
            panic!()
        }
    };
    drop(db_content);

    if let Err(e) = tokio::fs::create_dir_all(&args.public_path).await {
        log::error!("Error creating public path: {}", e);
        panic!()
    }

    let app_state = AppState {
        db_value: Arc::new(RwLock::new(db_value)),
        dirty: Arc::new(RwLock::new(false)),
        id: args.id.to_string(),
        public_path: args.public_path.clone(),
    };

    let (server_tx, server_rx) = std::sync::mpsc::channel::<bool>();

    let app_state_for_server = app_state.clone();
    let args_for_server = args.clone();
    let server_tack = tokio::spawn(async move {
        match axum::Server::try_bind(
            &args
                .bind_address
                .parse()
                .expect("Error parsing bind_address"),
        ) {
            Ok(server) => {
                server_tx
                    .send(true)
                    .expect("Error sending server start success signal");
                match server
                    .serve(
                        handler::build_router(app_state_for_server, args_for_server)
                            .await
                            .into_make_service(),
                    )
                    .await
                {
                    Ok(_) => log::info!("Server exited"),
                    Err(e) => log::error!("Server exited with error: {}", e),
                };
            }
            Err(e) => {
                log::error!("Error binding server: {}", e);
                server_tx
                    .send(false)
                    .expect("Error sending server start fail signal");
            }
        }
    });

    if let Ok(false) = server_rx.recv() {
        log::error!("Server failed to start");
        return;
    }

    let db_path = args.db_path.clone();
    let app_state_for_save = app_state.clone();
    let save_task = tokio::spawn(async move {
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
            let mut dirty = app_state_for_save.dirty.write().await;
            if !*dirty {
                log::trace!("Database file saving... skipped");
                continue;
            }
            save(app_state_for_save.clone(), &db_path).await;
            *dirty = false;
        }
    });

    let (cctx, ccrx) = std::sync::mpsc::channel();

    ctrlc::set_handler(move || {
        cctx.send(()).expect("Error sending CTRL+C signal");
    })
    .expect("Error setting CTRL+C handler");

    ccrx.recv().expect("Could not receive from channel.");

    log::info!("Ctrl-C received");
    server_tack.abort();
    save_task.abort();
    let dirty = app_state.dirty.read().await;
    if *dirty {
        drop(dirty);
        save(app_state, &args.db_path).await;
    }
    log::info!("Server exited");
}

async fn save(app_state: AppState, db_path: &str) {
    log::info!("Database file saving...");
    let db_value = app_state.db_value.read().await;
    let db_content = serde_json::to_string(&*db_value).expect("Error serializing database file");
    drop(db_value);
    let temp_file = format!("{}.tmp", db_path);
    let mut db_file = tokio::fs::File::create(&temp_file)
        .await
        .expect("Error creating database file");
    db_file
        .write_all(db_content.as_bytes())
        .await
        .expect("Error writing database file");
    tokio::fs::remove_file(db_path)
        .await
        .expect("Error removing old database file");
    tokio::fs::rename(&temp_file, db_path)
        .await
        .expect("Error renaming database file");
    log::info!("Database file saved");
}

#[derive(Parser, Debug, Clone)]
#[command(version, about, long_about = None)]
pub struct Args {
    #[arg(short, long, default_value = "0.0.0.0:2901")]
    bind_address: String,
    #[arg(short, long, default_value = "./data.json")]
    db_path: String,
    #[arg(short, long, default_value = "./public")]
    public_path: String,
    #[arg(short, long, default_value = "id")]
    id: String,
    #[arg(short, long, default_value = "100")]
    max_body_limit_m: usize,
    #[arg(long, default_value_t = false)]
    debug: bool,
}

#[derive(Clone)]
pub struct AppState {
    db_value: Arc<RwLock<Value>>,
    id: String,
    dirty: Arc<RwLock<bool>>,
    public_path: String,
}
