# CLAUDE.md

## Project Status: `awatcher` COSMIC Support - COMPLETED ✅

### 1. Objective

**COMPLETED:** Successfully added a Wayland window watcher to the `awatcher` project that is compatible with the COSMIC desktop environment.

### 2. Core Problem & Key Library

-   The existing `awatcher` Wayland watcher uses the `wlr-foreign-toplevel-management` protocol.
-   COSMIC DE does not use the `wlr` protocol. It uses the `ext-foreign-toplevel-list` protocol, extended with its own `cosmic-toplevel-info` protocol for extra details like window activation state.
-   **IMPLEMENTED:** Using `cctk` (`cosmic-client-toolkit`) from the `pop-os/cosmic-protocols` git repository.

### 3. Architectural Challenge & Solution

The primary technical challenge was integrating the blocking, event-driven nature of the Wayland client library with the `async` `tokio` runtime used by `awatcher`.

-   **Initial Failed Approach:** Placing the Wayland `event_queue.roundtrip()` call directly inside the `async run_iteration` function.
-   **Reason for Failure:** `roundtrip()` is a **synchronous, blocking** call. It stalls the `tokio` worker thread, causing the main application loop to time out (1-second timeout), resulting in log errors and gaps in activity data.
-   **IMPLEMENTED SOLUTION:** The blocking Wayland code runs on its own dedicated OS thread (`std::thread::spawn`). This thread runs a perpetual Wayland event loop. Communication back to the main `async` task is handled via a `tokio::sync::mpsc::channel`.
    -   The Wayland thread blocks as needed, waiting for events. When the active window changes, it sends a message containing the new window's `app_id` and `title` through the channel.
    -   The `async run_iteration` function is non-blocking. It does a `try_recv()` on the channel to get the latest active window info and sends its heartbeat.

### 4. Final Implementation Details

**File:** `watchers/src/watchers/cosmic_toplevel_management.rs`

**Key Components:**
1. **WindowWatcher struct:** Holds the receiver channel and last known active window
2. **WaylandState struct:** Manages COSMIC protocol state and channel sender
3. **wayland_thread function:** Runs on dedicated thread with blocking `roundtrip()` calls
4. **ToplevelInfoHandler implementation:** Detects window activation events and sends updates

**Critical Fixes Applied:**
- **Threading Architecture:** Separate OS thread for blocking Wayland operations
- **Channel Communication:** `tokio::sync::mpsc` for thread-safe async communication  
- **Timeout Handling:** 800ms timeout wrapper around `client.send_active_window()` calls to prevent ActivityWatch delays from causing iteration timeouts
- **Event Loop Optimization:** Added 10ms sleep in Wayland thread to prevent busy waiting
- **Error Handling:** Graceful degradation when client calls timeout

### 5. Performance & Behavior

**VERIFIED WORKING:**
- ✅ Continuous data collection without gaps
- ✅ Real-time window switching detection
- ✅ No timeout errors in normal operation
- ✅ Graceful handling of ActivityWatch server delays
- ✅ Proper integration with existing watcher selection system

**Integration:**
- Added to watcher selection in `watchers/src/watchers.rs` as "Wayland window (COSMIC ext-foreign-toplevel-list)"
- Automatically selected when COSMIC protocols are available
- Falls back to other watchers if COSMIC protocols not found

### 6. Testing Notes

The implementation was tested on COSMIC DE and confirmed to:
- Detect window activation events correctly
- Handle rapid window switching without data loss  
- Maintain stable operation over extended periods
- Gracefully handle network delays to ActivityWatch server

### 7. Dependencies

Required in `watchers/Cargo.toml`:
```toml
cctk = { git = "https://github.com/pop-os/cosmic-protocols", package = "cosmic-client-toolkit" }
```

**Status: Implementation complete and verified working** ✅

# important-instruction-reminders
Do what has been asked; nothing more, nothing less.
NEVER create files unless they're absolutely necessary for achieving your goal.
ALWAYS prefer editing an existing file to creating a new one.
NEVER proactively create documentation files (*.md) or README files. Only create documentation files if explicitly requested by the User.