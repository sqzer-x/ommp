use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use crossbeam_channel::Sender;
use notify::{RecommendedWatcher, RecursiveMode, Watcher, EventKind};

use crate::event::Event;
use crate::library::Library;

pub fn spawn_watcher(music_dir: &Path, event_tx: Sender<Event>) -> Option<RecommendedWatcher> {
    let (notify_tx, notify_rx) = crossbeam_channel::unbounded();
    let dir = music_dir.to_path_buf();

    let mut watcher = RecommendedWatcher::new(
        move |res: Result<notify::Event, notify::Error>| {
            if let Ok(ev) = res {
                let dominated = matches!(
                    ev.kind,
                    EventKind::Create(_) | EventKind::Remove(_) | EventKind::Modify(_)
                );
                if dominated {
                    let _ = notify_tx.send(());
                }
            }
        },
        notify::Config::default(),
    ).ok()?;

    watcher.watch(music_dir, RecursiveMode::Recursive).ok()?;

    let scanning = Arc::new(AtomicBool::new(false));

    // Debounce thread
    let scanning_clone = scanning.clone();
    std::thread::spawn(move || {
        let debounce = Duration::from_secs(2);
        let mut last_event = Instant::now();

        loop {
            match notify_rx.recv_timeout(Duration::from_millis(500)) {
                Ok(()) => {
                    last_event = Instant::now();
                }
                Err(crossbeam_channel::RecvTimeoutError::Timeout) => {
                    if last_event.elapsed() >= debounce
                        && !scanning_clone.load(Ordering::Relaxed)
                    {
                        // Check if there were any events recently
                        // Drain any pending events
                        let mut had_events = false;
                        while notify_rx.try_recv().is_ok() {
                            had_events = true;
                        }
                        if had_events {
                            last_event = Instant::now();
                            continue;
                        }

                        // Only rescan if we actually saw events since last scan
                        if last_event.elapsed() < debounce + Duration::from_millis(600) {
                            scanning_clone.store(true, Ordering::Relaxed);
                            let lib = Library::scan(&dir);
                            let _ = event_tx.send(Event::LibraryReady(lib));
                            scanning_clone.store(false, Ordering::Relaxed);
                        }
                    }
                }
                Err(crossbeam_channel::RecvTimeoutError::Disconnected) => break,
            }
        }
    });

    Some(watcher)
}
