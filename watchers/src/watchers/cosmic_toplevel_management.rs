use super::Watcher;
use crate::report_client::ReportClient;
use anyhow::{anyhow, Context};
use async_trait::async_trait;
use cctk::{
    delegate_toplevel_info,
    sctk::registry::{ProvidesRegistryState, RegistryState},
    toplevel_info::{ToplevelInfoHandler, ToplevelInfoState},
    wayland_client::{
        globals::registry_queue_init, protocol::wl_registry, Connection, Dispatch, Proxy,
        QueueHandle,
    },
    wayland_protocols::ext::foreign_toplevel_list::v1::client::ext_foreign_toplevel_handle_v1,
};
use std::{sync::Arc, thread};
use tokio::sync::mpsc;

// This struct will be sent over the channel from the Wayland thread
// to the main async task.
#[derive(Clone, Debug)]
struct ActiveWindow {
    app_id: String,
    title: String,
}

// The state struct is now much simpler. It only needs to hold the
// receiver for the channel and the last known active window.
pub struct WindowWatcher {
    receiver: mpsc::Receiver<ActiveWindow>,
    last_active_window: Option<ActiveWindow>,
}

#[async_trait]
impl Watcher for WindowWatcher {
    async fn new(_: &Arc<ReportClient>) -> anyhow::Result<Self> {
        // Create a channel to communicate between the new thread and the async runtime.
        let (sender, receiver) = mpsc::channel(32);

        // Spawn a dedicated OS thread for all blocking Wayland communication.
        thread::spawn(move || {
            if let Err(e) = wayland_thread(sender) {
                log::error!("Wayland thread failed: {:?}", e);
            }
        });

        Ok(Self {
            receiver,
            last_active_window: None,
        })
    }

    async fn run_iteration(&mut self, client: &Arc<ReportClient>) -> anyhow::Result<()> {
        // Check for a new message from the Wayland thread without blocking.
        // We process all pending messages to get the most recent one.
        while let Ok(new_window) = self.receiver.try_recv() {
            self.last_active_window = Some(new_window);
        }

        // Report the last known active window. This is now fully non-blocking.
        if let Some(active_window) = &self.last_active_window {
            // Use a timeout wrapper to prevent the client call from blocking too long
            match tokio::time::timeout(
                std::time::Duration::from_millis(800), // Leave some margin for the 1s timeout
                client.send_active_window(&active_window.app_id, &active_window.title),
            )
            .await
            {
                Ok(result) => {
                    result.with_context(|| "Failed to send heartbeat for active window")?;
                }
                Err(_) => {
                    log::warn!("Client send_active_window timed out after 800ms");
                    // Don't return error, just skip this iteration
                }
            }
        }

        Ok(())
    }
}

// --- The Wayland Thread Implementation ---

// This is the state for the dedicated Wayland thread.
struct WaylandState {
    registry_state: RegistryState,
    toplevel_info_state: ToplevelInfoState,
    active_toplevel_identifier: Option<String>,
    // We hold the sender to communicate back to the main task.
    sender: mpsc::Sender<ActiveWindow>,
}

// This is the main function that runs on the new thread.
fn wayland_thread(sender: mpsc::Sender<ActiveWindow>) -> anyhow::Result<()> {
    log::info!("Starting COSMIC Wayland thread");

    let conn = Connection::connect_to_env()?;
    let (globals, mut event_queue) = registry_queue_init(&conn)?;
    let qh = event_queue.handle();

    let registry_state = RegistryState::new(&globals);
    let toplevel_info_state = ToplevelInfoState::try_new(&registry_state, &qh)
        .ok_or_else(|| anyhow!("Required COSMIC toplevel protocols not found"))?;

    log::info!("COSMIC toplevel protocols initialized successfully");

    let mut state = WaylandState {
        registry_state,
        toplevel_info_state,
        active_toplevel_identifier: None,
        sender,
    };

    // Initial roundtrip to get existing toplevels
    log::debug!("Performing initial roundtrip");
    event_queue.roundtrip(&mut state)?;
    log::debug!("Initial roundtrip completed");

    // The main event loop for the thread - this can block as needed
    loop {
        match event_queue.roundtrip(&mut state) {
            Ok(_) => {
                // Small sleep to prevent busy waiting
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
            Err(e) => {
                log::error!("Error in Wayland event loop: {:?}", e);
                break;
            }
        }
    }

    log::info!("COSMIC Wayland thread exiting");
    Ok(())
}

// --- Trait Implementations for WaylandState ---

impl ToplevelInfoHandler for WaylandState {
    fn toplevel_info_state(&mut self) -> &mut ToplevelInfoState {
        &mut self.toplevel_info_state
    }

    fn new_toplevel(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        toplevel: &ext_foreign_toplevel_handle_v1::ExtForeignToplevelHandleV1,
    ) {
        self.update_toplevel(_conn, _qh, toplevel);
    }

    fn update_toplevel(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        toplevel: &ext_foreign_toplevel_handle_v1::ExtForeignToplevelHandleV1,
    ) {
        if let Some(info) = self.toplevel_info_state.info(toplevel) {
            if info.state.contains(&cctk::cosmic_protocols::toplevel_info::v1::client::zcosmic_toplevel_handle_v1::State::Activated) {
                // If the active window has changed, send an update.
                if self.active_toplevel_identifier.as_ref() != Some(&info.identifier) {
                    log::debug!("Active window changed to: {} - {}", info.app_id, info.title);
                    self.active_toplevel_identifier = Some(info.identifier.clone());
                    let active_window = ActiveWindow {
                        app_id: info.app_id.clone(),
                        title: info.title.clone(),
                    };
                    // This send can fail if the receiver is dropped, which means the app is shutting down.
                    if self.sender.blocking_send(active_window).is_err() {
                        log::info!("Wayland thread shutting down: receiver closed.");
                        // Break the loop since the main task is no longer listening
                        return;
                    }
                }
            }
        }
    }

    fn toplevel_closed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _toplevel: &ext_foreign_toplevel_handle_v1::ExtForeignToplevelHandleV1,
    ) {
        // Logic to handle the active window closing could be added here if needed.
    }
}

delegate_toplevel_info!(WaylandState);

impl ProvidesRegistryState for WaylandState {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry_state
    }
    cctk::sctk::registry_handlers!();
}

cctk::sctk::delegate_registry!(WaylandState);

impl Dispatch<wl_registry::WlRegistry, ()> for WaylandState {
    fn event(
        _: &mut Self,
        _: &wl_registry::WlRegistry,
        _: <wl_registry::WlRegistry as Proxy>::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}
