use crate::try_download_replay_caches;
use swarmy_tauri_common::SwarmyTauriError;
use tauri::AppHandle;
use tokio::sync::mpsc;
/// A Tokio MPSC Majorodomo inspeired by ZMQ.
///
use tokio::{runtime::Handle, sync::oneshot};
use tracing::{info, instrument};

#[derive(Debug)]
pub enum AsyncTask {
    Shutdown,
    DownloadCaches(oneshot::Sender<()>),
}

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
    ) -> Result<Self, SwarmyTauriError> {
        Ok(MajordomoCoordinator {
            rx: main_rx,
            app_handle: app,
        })
    }

    #[instrument]
    pub async fn process_message_queue(&mut self) -> Result<(), SwarmyTauriError> {
        info!("majordomo coordinator: Main loop starting");
        let current_tokio_handle = Handle::current();
        while let Some(message) = self.rx.recv().await {
            info!("majordomo coordinator: message: {:?}", message);
            match message {
                AsyncTask::Shutdown => {
                    info!("majordomo coordinator: Shutting down");
                    break;
                }
                AsyncTask::DownloadCaches(res_tx) => {
                    info!("majordomo coordinator: Downloading caches");
                    // Spawn a task to download the caches so we don't block the main loop
                    let app_handle_clone = self.app_handle.clone();
                    current_tokio_handle.spawn(async move {
                        // Call the download caches function and wait for it to complete
                        let result = try_download_replay_caches(app_handle_clone).await;
                        // Handle the result of the download caches function
                        match result {
                            Ok(_) => info!(
                                "majordomo coordinator: Download caches completed successfully"
                            ),
                            Err(e) => {
                                info!("majordomo coordinator: Error downloading caches: {:?}", e)
                            }
                        }
                    });
                    res_tx.send(()).unwrap();
                }
            }
        }
        info!("majordomo coordinator: Main loop exiting");
        Ok(())
    }

    pub fn init_coordinator_thread(
        main_rx: mpsc::Receiver<AsyncTask>,
        app: AppHandle,
    ) -> Result<std::thread::JoinHandle<()>, SwarmyTauriError> {
        log::info!("init_coordinator_thread: Starting Majordomo Coordinator thread");
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
}
