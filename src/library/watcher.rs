use std::path::Path;
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

    // Debounce thread. `pending` is None until a real filesystem event arrives,
    // so startup does not look like activity — seeding it with `Instant::now()`
    // used to fire a full rescan ~2s in with nothing having changed, and again
    // at ~2.5s because the timestamp was never reset after scanning.
    std::thread::spawn(move || {
        let debounce = Duration::from_secs(2);
        let mut pending: Option<Instant> = None;

        loop {
            match notify_rx.recv_timeout(Duration::from_millis(500)) {
                Ok(()) => {
                    pending = Some(Instant::now());
                }
                Err(crossbeam_channel::RecvTimeoutError::Timeout) => {
                    let Some(last_event) = pending else { continue };
                    if last_event.elapsed() < debounce {
                        continue;
                    }
                    // Drain anything that landed while we were deciding; if the
                    // batch is still arriving, wait for it to settle.
                    let mut had_events = false;
                    while notify_rx.try_recv().is_ok() {
                        had_events = true;
                    }
                    if had_events {
                        pending = Some(Instant::now());
                        continue;
                    }

                    pending = None;
                    let lib = Library::scan(&dir);
                    let _ = event_tx.send(Event::LibraryReady(Box::new(lib)));
                }
                Err(crossbeam_channel::RecvTimeoutError::Disconnected) => break,
            }
        }
    });

    Some(watcher)
}
