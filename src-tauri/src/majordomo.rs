//! A Tokio MPSC Majorodomo inspeired by ZMQ.

use s2protocol::dir_stats::SC2ReplaysDirStats;
use swarmy_common::{ApiResponse, ApiResponseBuilder, MapStatsQuery, SwarmyError};
use tauri::AppHandle;
use tokio::runtime::Handle;
use tokio::sync::mpsc;
use tracing::{info, instrument};

use crate::map_stats::data::try_query_map_stats;
use crate::{get_current_app_config, try_get_snapshot_metadata, try_optimize_replay_path};

pub use swarmy_common::AsyncTask;

/// A MajordomoCoordinator that keeps the state to be shared across async tasks
#[derive(Debug)]
pub struct MajordomoCoordinator {
    rx: mpsc::Receiver<AsyncTask>,
    app_handle: AppHandle,
}
impl MajordomoCoordinator {
    /// Creates a new instance of the MajordomoCoordinator
    pub async fn new(
        main_rx: mpsc::Receiver<AsyncTask>,
        app: AppHandle,
    ) -> Result<Self, SwarmyError> {
        Ok(MajordomoCoordinator {
            rx: main_rx,
            app_handle: app,
        })
    }

    #[instrument(level = "info", skip(self))]
    pub async fn process_message_queue(&mut self) -> Result<(), SwarmyError> {
        info!("majordomo coordinator: Main loop starting");
        while let Some(message) = self.rx.recv().await {
            info!("majordomo coordinator: message: {:?}", message);
            match message {
                AsyncTask::Shutdown => {
                    info!("majordomo coordinator: Shutting down");
                    break;
                }
                AsyncTask::BasicScanReplayPath(res_tx) => {
                    self.spawn_try_basic_scan_replay_path(res_tx).await;
                }
                AsyncTask::OptimizeReplayPath(res_tx) => {
                    self.spawn_try_optimize_replay_path(res_tx).await;
                }
                AsyncTask::QueryMapStats {
                    map_title,
                    player_name,
                    res_tx,
                } => {
                    self.spawn_try_query_map_stats(map_title, player_name, res_tx)
                        .await;
                }
                AsyncTask::GetSnapshotStats(res_tx) => {
                    self.spawn_try_get_snapshot_stats(res_tx).await;
                }
            }
        }
        info!("majordomo coordinator: Main loop exiting");
        Ok(())
    }

    pub fn init_coordinator_thread(
        main_rx: mpsc::Receiver<AsyncTask>,
        app: AppHandle,
    ) -> Result<std::thread::JoinHandle<()>, SwarmyError> {
        tracing::info!("init_coordinator_thread: Starting Majordomo Coordinator thread");
        let current_tokio_handle = Handle::current();
        let coordinator_thread = std::thread::Builder::new()
            .name("Majordomo Coordinator I/O".to_owned())
            .spawn(move || {
                current_tokio_handle.spawn(async move {
                    let mut majordomo_coordinator = Self::new(main_rx, app)
                        .await
                        .expect("Unable to create Majordomo Coordinator");
                    majordomo_coordinator
                        .process_message_queue()
                        .await
                        .expect("Majordomo coordinator exited with error.");
                });
            })
            .expect("Unable to start Majordomo Coordinator async I/O thread");
        Ok(coordinator_thread)
    }

    #[instrument]
    pub async fn shutdown(tx: mpsc::Sender<AsyncTask>) {
        tx.send(AsyncTask::Shutdown).await.unwrap();
    }

    #[tracing::instrument(level = "debug", skip(self, res_tx))]
    async fn spawn_try_basic_scan_replay_path(
        &self,
        res_tx: tokio::sync::oneshot::Sender<ApiResponse>,
    ) {
        tracing::info!("spawn_try_basic_scan_replay_path: Spawning task to scan replays directory");
        let app_handle_clone = self.app_handle.clone();
        let current_tokio_handle = Handle::current();
        current_tokio_handle.spawn(async move {
            let res = ApiResponseBuilder::new();
            let app_settings = crate::settings::load_app_settings(app_handle_clone)
                .await
                .unwrap();
            let replay_path = app_settings.replay_path.clone();
            let disable_parallelism = app_settings.disable_parallelism;

            tracing::info!("Scanning replays directory: {}", replay_path);

            res_tx
                .send(res.process_result(
                    SC2ReplaysDirStats::from_directory(&replay_path, disable_parallelism).map_err(
                        |e| SwarmyError::Other(format!("Error scanning replays directory: {}", e)),
                    ),
                ))
                .expect("Failed to send response");
        });
    }

    #[tracing::instrument(level = "debug")]
    async fn spawn_try_optimize_replay_path(
        &self,
        res_tx: tokio::sync::oneshot::Sender<ApiResponse>,
    ) {
        let app_handle_clone = self.app_handle.clone();
        let current_tokio_handle = Handle::current();
        current_tokio_handle.spawn(async move {
            let res = ApiResponseBuilder::new();
            let snapshot_stats = try_optimize_replay_path(app_handle_clone).await;
            res_tx
                .send(res.process_result(snapshot_stats))
                .expect("Failed to send response");
        });
    }

    #[tracing::instrument(level = "debug")]
    async fn spawn_try_query_map_stats(
        &self,
        map_title: String,
        player_name: String,
        res_tx: tokio::sync::oneshot::Sender<ApiResponse>,
    ) {
        let query = MapStatsQuery {
            map_title,
            player_name,
        };
        let current_tokio_handle = Handle::current();
        let app_config = match get_current_app_config(self.app_handle.clone()).await {
            Ok(config) => config,
            Err(e) => {
                res_tx
                    .send(
                        ApiResponseBuilder::new()
                            .with_failure()
                            .with_message(format!("Error getting app config: {}", e))
                            .build(),
                    )
                    .expect("Failed to send response");
                return;
            }
        };
        current_tokio_handle.spawn(async move {
            let res = ApiResponseBuilder::new();
            res_tx.send(res.process_result(try_query_map_stats(app_config.replay_path, query)))
        });
    }

    #[tracing::instrument(level = "debug")]
    pub async fn spawn_try_get_snapshot_stats(
        &self,
        res_tx: tokio::sync::oneshot::Sender<ApiResponse>,
    ) {
        let res = ApiResponseBuilder::new();
        let app_config = match get_current_app_config(self.app_handle.clone()).await {
            Ok(config) => config,
            Err(e) => {
                res_tx
                    .send(
                        res.with_failure()
                            .with_message(format!("Error getting app config: {}", e))
                            .build(),
                    )
                    .expect("Failed to send response");
                return;
            }
        };
        let current_tokio_handle = Handle::current();
        current_tokio_handle.spawn(async move {
            res_tx
                .send(res.process_result(try_get_snapshot_metadata(
                    app_config.replay_path,
                    app_config.cache_path,
                )))
                .expect("Failed to send response");
        });
    }
}
