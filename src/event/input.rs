use crossbeam_channel::Sender;
use crossterm::event::{self, Event as CtEvent, KeyEventKind};
use std::time::Duration;

use super::Event;

pub fn spawn_input_thread(tx: Sender<Event>) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        loop {
            if event::poll(Duration::from_millis(100)).unwrap_or(false) {
                match event::read() {
                    Ok(CtEvent::Key(key)) => {
                        if key.kind == KeyEventKind::Press
                            && tx.send(Event::Key(key)).is_err()
                        {
                            break;
                        }
                    }
                    Ok(CtEvent::Mouse(mouse)) => {
                        if tx.send(Event::Mouse(mouse)).is_err() {
                            break;
                        }
                    }
                    Ok(CtEvent::Resize(w, h)) => {
                        if tx.send(Event::Resize(w, h)).is_err() {
                            break;
                        }
                    }
                    _ => {}
                }
            }
        }
    })
}

pub fn spawn_tick_thread(tx: Sender<Event>, interval: Duration) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || loop {
        std::thread::sleep(interval);
        if tx.send(Event::Tick).is_err() {
            break;
        }
    })
}
