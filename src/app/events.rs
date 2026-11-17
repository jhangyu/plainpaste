//! The app event type, its channel, and the wiring of every event source (hotkey, UI).

use std::collections::VecDeque;
use std::sync::{Arc, Mutex, MutexGuard};
use std::task::{Poll, Waker};

use global_hotkey::{GlobalHotKeyEvent, HotKeyState};
use plainpaste::config::Theme;

use super::recorder::KeyModifiers;
use crate::Flyout;

#[derive(Debug)]
pub enum AppEvent {
    Hotkey {
        id: u32,
        state: HotKeyState,
    },
    OpenFlyout {
        anchor: Option<tray_icon::Rect>,
    },
    FlyoutFocusLost,
    PasteReadTimeout {
        session: u64,
    },
    StartRecording,
    CancelRecording,
    RecordKey {
        text: String,
        modifiers: KeyModifiers,
    },
    ResetHotkey,
    ToggleKeepLineBreaks,
    ToggleRestoreClipboard,
    ChooseTheme(Theme),
    OpenAccessibilitySettings,
    Quit,
}

/// The event queue shared by every `EventSender` and the single `EventReceiver`.
struct Queue {
    events: VecDeque<AppEvent>,
    /// Waker of the receiver while it waits on an empty queue. Stored under the same lock as
    /// `events`, so a send can never slip between the empty check and the waker store.
    waker: Option<Waker>,
    receiver_alive: bool,
}

type Shared = Arc<Mutex<Queue>>;

fn lock(shared: &Shared) -> MutexGuard<'_, Queue> {
    shared
        .lock()
        .expect("app event queue lock: expected unpoisoned, actual poisoned by a panic")
}

/// Cloneable, thread-safe sender for `AppEvent`.
#[derive(Clone)]
pub struct EventSender(Shared);

impl EventSender {
    pub fn send(&self, event: AppEvent) {
        if let Err(event) = self.try_send(event) {
            log::error!(
                "app event channel: expected open, actual closed. Dropped {:?}",
                event
            );
        }
    }

    /// Queues `event` and wakes the receiver. Returns the event back if the receiver is gone.
    fn try_send(&self, event: AppEvent) -> Result<(), AppEvent> {
        let waker = {
            let mut queue = lock(&self.0);
            if !queue.receiver_alive {
                return Err(event);
            }
            queue.events.push_back(event);
            queue.waker.take()
        };
        if let Some(waker) = waker {
            waker.wake();
        }
        Ok(())
    }
}

/// The single consumer of the app event queue.
pub struct EventReceiver(Shared);

impl EventReceiver {
    /// Waits for the next event, in send order.
    // ponytail: never ends. Every event source keeps a sender alive for the whole run, so no
    // "all senders dropped" state is tracked.
    pub async fn recv(&mut self) -> AppEvent {
        std::future::poll_fn(|cx| {
            let mut queue = lock(&self.0);
            match queue.events.pop_front() {
                Some(event) => Poll::Ready(event),
                None => {
                    queue.waker = Some(cx.waker().clone());
                    Poll::Pending
                }
            }
        })
        .await
    }
}

impl Drop for EventReceiver {
    fn drop(&mut self) {
        let mut queue = lock(&self.0);
        queue.receiver_alive = false;
        queue.events.clear();
    }
}

pub fn channel() -> (EventSender, EventReceiver) {
    let shared = Arc::new(Mutex::new(Queue {
        events: VecDeque::new(),
        waker: None,
        receiver_alive: true,
    }));
    (EventSender(shared.clone()), EventReceiver(shared))
}

/// Wires every UI callback and the hotkey handler to `events`.
pub fn connect_sources(ui: &Flyout, events: &EventSender) {
    let tx = events.clone();
    GlobalHotKeyEvent::set_event_handler(Some(move |e: GlobalHotKeyEvent| {
        tx.send(AppEvent::Hotkey {
            id: e.id,
            state: e.state,
        })
    }));
    let tx = events.clone();
    ui.on_start_recording(move || tx.send(AppEvent::StartRecording));
    let tx = events.clone();
    ui.on_cancel_recording(move || tx.send(AppEvent::CancelRecording));
    let tx = events.clone();
    ui.on_record_key(move |text, control, alt, shift, meta| {
        tx.send(AppEvent::RecordKey {
            text: text.to_string(),
            modifiers: KeyModifiers {
                control,
                alt,
                shift,
                meta,
            },
        })
    });
    let tx = events.clone();
    ui.on_reset_hotkey(move || tx.send(AppEvent::ResetHotkey));
    let tx = events.clone();
    ui.on_toggle_keep_line_breaks(move || tx.send(AppEvent::ToggleKeepLineBreaks));
    let tx = events.clone();
    ui.on_toggle_restore_clipboard(move || tx.send(AppEvent::ToggleRestoreClipboard));
    let tx = events.clone();
    ui.on_choose_theme(move |choice| tx.send(AppEvent::ChooseTheme(choice.into())));
    let tx = events.clone();
    ui.on_open_accessibility_settings(move || tx.send(AppEvent::OpenAccessibilitySettings));
    let tx = events.clone();
    ui.on_quit(move || tx.send(AppEvent::Quit));
}

#[cfg(test)]
mod tests {
    use std::future::Future;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::task::{Context, Wake};

    use super::*;

    struct Flag(AtomicBool);

    impl Wake for Flag {
        fn wake(self: Arc<Self>) {
            self.0.store(true, Ordering::SeqCst);
        }
    }

    #[test]
    fn send_from_another_thread_wakes_waiting_receiver() {
        let (tx, mut rx) = channel();
        let flag = Arc::new(Flag(AtomicBool::new(false)));
        let waker = Waker::from(flag.clone());
        let mut cx = Context::from_waker(&waker);
        let mut recv = Box::pin(rx.recv());

        assert!(recv.as_mut().poll(&mut cx).is_pending());
        std::thread::spawn(move || tx.send(AppEvent::Quit))
            .join()
            .unwrap();

        assert!(
            flag.0.load(Ordering::SeqCst),
            "expected the send to wake the waiting receiver, actual: not woken"
        );
        assert!(matches!(
            recv.as_mut().poll(&mut cx),
            Poll::Ready(AppEvent::Quit)
        ));
    }

    #[test]
    fn events_arrive_in_send_order() {
        let (tx, mut rx) = channel();
        for session in 0..3 {
            tx.send(AppEvent::PasteReadTimeout { session });
        }
        let mut cx = Context::from_waker(Waker::noop());
        for expected in 0..3 {
            let mut recv = Box::pin(rx.recv());
            match recv.as_mut().poll(&mut cx) {
                Poll::Ready(AppEvent::PasteReadTimeout { session }) => {
                    assert_eq!(session, expected)
                }
                other => {
                    panic!("expected PasteReadTimeout {{ session: {expected} }}, actual {other:?}")
                }
            }
        }
    }

    #[test]
    fn send_after_receiver_drop_errors() {
        let (tx, rx) = channel();
        drop(rx);
        assert!(matches!(tx.try_send(AppEvent::Quit), Err(AppEvent::Quit)));
    }
}
