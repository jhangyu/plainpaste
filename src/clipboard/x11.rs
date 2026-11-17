//! X11 CLIPBOARD selection implementation. The app becomes the selection owner and
//! answers SelectionRequest itself, so the offered data is lazy by nature.
//! A dedicated thread blocks in `wait_for_event` (event push, no polling) and
//! serves requests. The caller thread issues requests on the shared connection.

use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use x11rb::COPY_DEPTH_FROM_PARENT;
use x11rb::connection::{Connection, RequestConnection};
use x11rb::protocol::Event;
use x11rb::protocol::xproto::{
    Atom, AtomEnum, ConnectionExt, CreateWindowAux, EventMask, PropMode, SELECTION_NOTIFY_EVENT,
    SelectionNotifyEvent, SelectionRequestEvent, Time, Window, WindowClass,
};
use x11rb::rust_connection::RustConnection;
use x11rb::wrapper::ConnectionExt as _;

use super::{Clipboard, ClipboardError};

/// Fail-fast IO bound for one ConvertSelection round trip to another client.
/// An unresponsive selection owner must not block the hotkey path forever.
const SELECTION_REPLY_TIMEOUT: Duration = Duration::from_secs(2);
const FORMAT_8_BIT: u8 = 8;
const FORMAT_32_BIT: u8 = 32;
const BYTES_PER_32_BIT_ITEM: usize = 4;
/// GetProperty length is counted in 32-bit units. This asks for everything.
const WHOLE_PROPERTY_LENGTH: u32 = u32::MAX / BYTES_PER_32_BIT_ITEM as u32;
/// Space reserved for the ChangeProperty request header inside the maximum request size.
const CHANGE_PROPERTY_HEADER_BYTES: usize = 24;

x11rb::atom_manager! {
    Atoms: AtomsCookie {
        CLIPBOARD,
        TARGETS,
        MULTIPLE,
        TIMESTAMP,
        SAVE_TARGETS,
        INCR,
        UTF8_STRING,
        TEXT_PLAIN_UTF8: b"text/plain;charset=utf-8",
        PLAINPASTE_TRANSFER,
    }
}

/// One served or saved selection target.
#[derive(Clone)]
struct Format {
    target: Atom,
    type_: Atom,
    format: u8,
    bytes: Vec<u8>,
}

pub struct X11Snapshot {
    formats: Vec<Format>,
}

/// What this client currently serves as the CLIPBOARD owner.
#[derive(Default)]
struct Served {
    formats: Vec<Format>,
}

pub struct X11Clipboard {
    conn: Arc<RustConnection>,
    window: Window,
    atoms: Atoms,
    served: Arc<Mutex<Served>>,
    replies: Receiver<SelectionNotifyEvent>,
}

fn conn_err(op: &'static str, request: &str, e: impl std::fmt::Display) -> ClipboardError {
    ClipboardError::new(op, format!("X11 {request}: expected success, actual {e}"))
}

impl X11Clipboard {
    pub fn new() -> Result<Self, ClipboardError> {
        const OP: &str = "connect";
        let (conn, screen_num) =
            RustConnection::connect(None).map_err(|e| conn_err(OP, "connect($DISPLAY)", e))?;
        let root = conn.setup().roots[screen_num].root;
        let window = conn
            .generate_id()
            .map_err(|e| conn_err(OP, "generate_id", e))?;
        conn.create_window(
            COPY_DEPTH_FROM_PARENT,
            window,
            root,
            0,
            0,
            1,
            1,
            0,
            WindowClass::INPUT_ONLY,
            x11rb::COPY_FROM_PARENT,
            &CreateWindowAux::new().event_mask(EventMask::PROPERTY_CHANGE),
        )
        .map_err(|e| conn_err(OP, "CreateWindow", e))?;
        let atoms = Atoms::new(&conn)
            .map_err(|e| conn_err(OP, "InternAtom", e))?
            .reply()
            .map_err(|e| conn_err(OP, "InternAtom reply", e))?;
        conn.flush().map_err(|e| conn_err(OP, "flush", e))?;

        let conn = Arc::new(conn);
        let served = Arc::new(Mutex::new(Served::default()));
        let (reply_tx, replies) = mpsc::channel();
        let thread_conn = Arc::clone(&conn);
        let thread_served = Arc::clone(&served);
        thread::Builder::new()
            .name("x11-clipboard".into())
            .spawn(move || event_thread(thread_conn, atoms, thread_served, reply_tx))
            .map_err(|e| conn_err(OP, "spawn event thread", e))?;
        Ok(X11Clipboard {
            conn,
            window,
            atoms,
            served,
            replies,
        })
    }

    /// Asks the current owner to convert CLIPBOARD to `target`, then reads the result.
    /// `Ok(None)` means the owner refused the target (or there is no owner).
    fn convert(
        &mut self,
        op: &'static str,
        target: Atom,
    ) -> Result<Option<Format>, ClipboardError> {
        let transfer = self.atoms.PLAINPASTE_TRANSFER;
        self.conn
            .convert_selection(
                self.window,
                self.atoms.CLIPBOARD,
                target,
                transfer,
                Time::CURRENT_TIME,
            )
            .map_err(|e| conn_err(op, "ConvertSelection", e))?;
        self.conn.flush().map_err(|e| conn_err(op, "flush", e))?;
        let notify = loop {
            let event = self.replies.recv_timeout(SELECTION_REPLY_TIMEOUT).map_err(|e| {
                ClipboardError::new(
                    op,
                    format!("SelectionNotify for target {target}: expected within {SELECTION_REPLY_TIMEOUT:?}, actual {e}"),
                )
            })?;
            if event.target == target {
                break event;
            }
            log::warn!(
                "clipboard {op}: dropped stale SelectionNotify for target {}",
                event.target
            );
        };
        if notify.property == u32::from(AtomEnum::NONE) {
            return Ok(None);
        }
        let reply = self
            .conn
            .get_property(
                true,
                self.window,
                transfer,
                AtomEnum::ANY,
                0,
                WHOLE_PROPERTY_LENGTH,
            )
            .map_err(|e| conn_err(op, "GetProperty", e))?
            .reply()
            .map_err(|e| conn_err(op, "GetProperty reply", e))?;
        if reply.type_ == self.atoms.INCR {
            return Err(ClipboardError::new(
                op,
                format!(
                    "target {target}: expected a single-transfer property, actual INCR (large data, not supported)"
                ),
            ));
        }
        Ok(Some(Format {
            target,
            type_: reply.type_,
            format: reply.format,
            bytes: reply.value,
        }))
    }

    /// Serves `formats` as the CLIPBOARD owner.
    fn own(
        &mut self,
        op: &'static str,
        formats: Vec<Format>,
    ) -> Result<(), ClipboardError> {
        *self.served.lock().expect("served mutex poisoned") = Served { formats };
        self.conn
            .set_selection_owner(self.window, self.atoms.CLIPBOARD, Time::CURRENT_TIME)
            .map_err(|e| conn_err(op, "SetSelectionOwner", e))?;
        let owner = self
            .conn
            .get_selection_owner(self.atoms.CLIPBOARD)
            .map_err(|e| conn_err(op, "GetSelectionOwner", e))?
            .reply()
            .map_err(|e| conn_err(op, "GetSelectionOwner reply", e))?
            .owner;
        if owner != self.window {
            return Err(ClipboardError::new(
                op,
                format!(
                    "CLIPBOARD owner: expected window {}, actual {owner}",
                    self.window
                ),
            ));
        }
        Ok(())
    }

    fn text_formats(&self, text: &str) -> Vec<Format> {
        [self.atoms.UTF8_STRING, self.atoms.TEXT_PLAIN_UTF8]
            .into_iter()
            .map(|target| Format {
                target,
                type_: self.atoms.UTF8_STRING,
                format: FORMAT_8_BIT,
                bytes: text.as_bytes().to_vec(),
            })
            .collect()
    }
}

impl Clipboard for X11Clipboard {
    type Snapshot = X11Snapshot;

    fn read_text(&mut self) -> Result<Option<String>, ClipboardError> {
        const OP: &str = "read text";
        match self.convert(OP, self.atoms.UTF8_STRING)? {
            None => Ok(None),
            Some(format) => String::from_utf8(format.bytes).map(Some).map_err(|e| {
                ClipboardError::new(OP, format!("UTF8_STRING: expected valid UTF-8, actual {e}"))
            }),
        }
    }

    fn write_text(&mut self, text: &str) -> Result<(), ClipboardError> {
        let formats = self.text_formats(text);
        self.own("write text", formats)
    }

    fn snapshot(&mut self) -> Result<X11Snapshot, ClipboardError> {
        const OP: &str = "snapshot";
        let Some(targets) = self.convert(OP, self.atoms.TARGETS)? else {
            return Ok(X11Snapshot {
                formats: Vec::new(),
            });
        };
        let skipped = [
            self.atoms.TARGETS,
            self.atoms.MULTIPLE,
            self.atoms.TIMESTAMP,
            self.atoms.SAVE_TARGETS,
        ];
        let mut formats = Vec::new();
        let (items, _) = targets.bytes.as_chunks::<BYTES_PER_32_BIT_ITEM>();
        for chunk in items {
            let target = u32::from_ne_bytes(*chunk);
            if skipped.contains(&target) {
                continue;
            }
            let format = self.convert(OP, target)?.ok_or_else(|| {
                ClipboardError::new(
                    OP,
                    format!("target {target}: expected data (owner listed it in TARGETS), actual refusal"),
                )
            })?;
            formats.push(format);
        }
        Ok(X11Snapshot { formats })
    }

    fn offer_lazy_text(&mut self, text: String) -> Result<(), ClipboardError> {
        let formats = self.text_formats(&text);
        self.own("offer lazy text", formats)
    }

    fn offer_is_current(&mut self) -> Result<bool, ClipboardError> {
        const OP: &str = "offer is current";
        let owner = self
            .conn
            .get_selection_owner(self.atoms.CLIPBOARD)
            .map_err(|e| conn_err(OP, "GetSelectionOwner", e))?
            .reply()
            .map_err(|e| conn_err(OP, "GetSelectionOwner reply", e))?
            .owner;
        Ok(owner == self.window)
    }

    fn restore(&mut self, snapshot: X11Snapshot) -> Result<(), ClipboardError> {
        self.own("restore", snapshot.formats)
    }
}

fn event_thread(
    conn: Arc<RustConnection>,
    atoms: Atoms,
    served: Arc<Mutex<Served>>,
    replies: Sender<SelectionNotifyEvent>,
) {
    loop {
        let event = match conn.wait_for_event() {
            Ok(event) => event,
            Err(e) => {
                log::error!(
                    "clipboard event thread: X11 wait_for_event failed: {e}. Clipboard serving stopped."
                );
                return;
            }
        };
        match event {
            Event::SelectionNotify(notify) => {
                if replies.send(notify).is_err() {
                    log::error!(
                        "clipboard event thread: reply receiver dropped. Clipboard serving stopped."
                    );
                    return;
                }
            }
            Event::SelectionRequest(request) => {
                if let Err(e) = answer(&conn, &atoms, &served, &request) {
                    log::error!("{e}");
                }
            }
            Event::SelectionClear(clear) if clear.selection == atoms.CLIPBOARD => {
                *served.lock().expect("served mutex poisoned") = Served::default();
            }
            Event::Error(e) => log::error!("clipboard event thread: X11 error {e:?}"),
            _ => {}
        }
    }
}

fn answer(
    conn: &RustConnection,
    atoms: &Atoms,
    served: &Mutex<Served>,
    request: &SelectionRequestEvent,
) -> Result<(), ClipboardError> {
    const OP: &str = "serve request";
    // Obsolete clients send property None and expect the target name to be used.
    let property = if request.property == u32::from(AtomEnum::NONE) {
        request.target
    } else {
        request.property
    };
    let max_payload = conn.maximum_request_bytes() - CHANGE_PROPERTY_HEADER_BYTES;
    let served = served.lock().expect("served mutex poisoned");
    let stored = if request.target == atoms.TARGETS {
        let mut list: Vec<Atom> = vec![atoms.TARGETS];
        list.extend(served.formats.iter().map(|f| f.target));
        conn.change_property32(
            PropMode::REPLACE,
            request.requestor,
            property,
            AtomEnum::ATOM,
            &list,
        )
        .map_err(|e| conn_err(OP, "ChangeProperty(TARGETS)", e))?;
        true
    } else if let Some(format) = served.formats.iter().find(|f| f.target == request.target) {
        if format.bytes.len() > max_payload {
            log::error!(
                "clipboard {OP}: target {}: expected at most {max_payload} bytes, actual {} (INCR not supported), refused",
                request.target,
                format.bytes.len()
            );
            false
        } else {
            let unit = if format.format == FORMAT_32_BIT {
                BYTES_PER_32_BIT_ITEM
            } else {
                1
            };
            conn.change_property(
                PropMode::REPLACE,
                request.requestor,
                property,
                format.type_,
                format.format,
                (format.bytes.len() / unit) as u32,
                &format.bytes,
            )
            .map_err(|e| conn_err(OP, "ChangeProperty", e))?;
            true
        }
    } else {
        false
    };
    drop(served);

    let notify = SelectionNotifyEvent {
        response_type: SELECTION_NOTIFY_EVENT,
        sequence: 0,
        time: request.time,
        requestor: request.requestor,
        selection: request.selection,
        target: request.target,
        property: if stored {
            property
        } else {
            AtomEnum::NONE.into()
        },
    };
    conn.send_event(false, request.requestor, EventMask::NO_EVENT, notify)
        .map_err(|e| conn_err(OP, "SendEvent(SelectionNotify)", e))?;
    conn.flush().map_err(|e| conn_err(OP, "flush", e))?;
    Ok(())
}
