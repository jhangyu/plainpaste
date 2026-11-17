//! Win32 clipboard implementation. Lazy provision uses delayed rendering:
//! SetClipboardData(CF_UNICODETEXT, NULL), then WM_RENDERFORMAT on a message-only owner window.
//! The owner window lives on the main thread, so the Slint (winit) message loop dispatches to it.

use std::cell::RefCell;

use windows::Win32::Foundation::{
    GlobalFree, HANDLE, HGLOBAL, HWND, LPARAM, LRESULT, SetLastError, WIN32_ERROR, WPARAM,
};
use windows::Win32::Graphics::Gdi::{GetEnhMetaFileBits, HENHMETAFILE, SetEnhMetaFileBits};
use windows::Win32::System::DataExchange::{
    CloseClipboard, EmptyClipboard, EnumClipboardFormats, GetClipboardData, GetClipboardOwner,
    IsClipboardFormatAvailable, OpenClipboard, SetClipboardData,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::Memory::{
    GMEM_MOVEABLE, GlobalAlloc, GlobalLock, GlobalSize, GlobalUnlock,
};
use windows::Win32::System::Ole::{
    CF_BITMAP, CF_DSPBITMAP, CF_DSPENHMETAFILE, CF_DSPMETAFILEPICT, CF_ENHMETAFILE,
    CF_METAFILEPICT, CF_OWNERDISPLAY, CF_PALETTE, CF_UNICODETEXT,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, GWLP_USERDATA, GetWindowLongPtrW, HWND_MESSAGE,
    RegisterClassW, SetWindowLongPtrW, WINDOW_EX_STYLE, WINDOW_STYLE, WM_RENDERFORMAT, WNDCLASSW,
};
use windows::core::{HRESULT, PCWSTR, w};

use super::{Clipboard, ClipboardError};

const OWNER_CLASS_NAME: PCWSTR = w!("PlainPasteClipboardOwner");
const UTF16_NUL: u16 = 0;
/// Last error of a documented NULL/FALSE success: GlobalUnlock at lock count zero,
/// SetClipboardData with a NULL handle (delayed rendering), GetClipboardOwner with no owner.
const NO_ERROR: HRESULT = HRESULT(0);
/// Formats whose handle is a GDI object, not HGLOBAL memory. Windows synthesizes
/// CF_BITMAP from CF_DIB and CF_METAFILEPICT from CF_ENHMETAFILE, so the
/// HGLOBAL siblings carry the same content. CF_ENHMETAFILE is saved through its bits.
const GDI_HANDLE_FORMATS: [u32; 7] = [
    CF_BITMAP.0 as u32,
    CF_METAFILEPICT.0 as u32,
    CF_PALETTE.0 as u32,
    CF_OWNERDISPLAY.0 as u32,
    CF_DSPBITMAP.0 as u32,
    CF_DSPMETAFILEPICT.0 as u32,
    CF_DSPENHMETAFILE.0 as u32,
];

pub struct WindowsSnapshot {
    formats: Vec<(u32, Vec<u8>)>,
}

/// State the window procedure reads on WM_RENDERFORMAT.
struct PendingOffer {
    text: String,
}

struct OwnerState {
    pending: RefCell<Option<PendingOffer>>,
}

pub struct WindowsClipboard {
    hwnd: HWND,
    /// Boxed so its address, stored in GWLP_USERDATA, stays fixed.
    state: Box<OwnerState>,
}

fn win_err(op: &'static str, api: &str, target: &str, e: windows::core::Error) -> ClipboardError {
    ClipboardError::new(
        op,
        format!(
            "{api} for {target}: expected success, actual HRESULT {:#010x} ({e})",
            e.code().0
        ),
    )
}

/// Closes the clipboard when dropped. CloseClipboard failure is logged, because Drop cannot return it.
struct OpenClipboardGuard;

impl OpenClipboardGuard {
    fn open(op: &'static str, owner: HWND) -> Result<Self, ClipboardError> {
        // SAFETY: owner is the live message-only window created in WindowsClipboard::new.
        unsafe { OpenClipboard(Some(owner)) }
            .map_err(|e| win_err(op, "OpenClipboard", "the clipboard", e))?;
        Ok(OpenClipboardGuard)
    }
}

impl Drop for OpenClipboardGuard {
    fn drop(&mut self) {
        // SAFETY: this guard exists only while this thread holds the clipboard open.
        if let Err(e) = unsafe { CloseClipboard() } {
            log::error!(
                "clipboard close failed: CloseClipboard: HRESULT {:#010x} ({e})",
                e.code().0
            );
        }
    }
}

fn copy_hglobal(op: &'static str, format: u32, handle: HANDLE) -> Result<Vec<u8>, ClipboardError> {
    let hglobal = HGLOBAL(handle.0);
    // SAFETY: handle is clipboard data for an HGLOBAL format, valid while the clipboard is open.
    unsafe {
        let size = GlobalSize(hglobal);
        let ptr = GlobalLock(hglobal) as *const u8;
        if ptr.is_null() {
            return Err(ClipboardError::new(
                op,
                format!("GlobalLock for format {format}: expected a pointer, actual NULL"),
            ));
        }
        let bytes = std::slice::from_raw_parts(ptr, size).to_vec();
        unlock(op, format, hglobal)?;
        Ok(bytes)
    }
}

fn unlock(op: &'static str, format: u32, hglobal: HGLOBAL) -> Result<(), ClipboardError> {
    // SAFETY: hglobal was locked once by the caller.
    match unsafe { GlobalUnlock(hglobal) } {
        Ok(()) => Ok(()),
        Err(e) if e.code() == NO_ERROR => Ok(()),
        Err(e) => Err(win_err(op, "GlobalUnlock", &format!("format {format}"), e)),
    }
}

fn alloc_hglobal(op: &'static str, format: u32, bytes: &[u8]) -> Result<HGLOBAL, ClipboardError> {
    // SAFETY: the allocation is sized for `bytes` and locked once while copying.
    unsafe {
        let hglobal = GlobalAlloc(GMEM_MOVEABLE, bytes.len()).map_err(|e| {
            win_err(
                op,
                "GlobalAlloc",
                &format!("format {format}, {} bytes", bytes.len()),
                e,
            )
        })?;
        let ptr = GlobalLock(hglobal) as *mut u8;
        if ptr.is_null() {
            return Err(ClipboardError::new(
                op,
                format!("GlobalLock for format {format}: expected a pointer, actual NULL"),
            ));
        }
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), ptr, bytes.len());
        unlock(op, format, hglobal)?;
        Ok(hglobal)
    }
}

fn utf16_bytes(text: &str) -> Vec<u8> {
    text.encode_utf16()
        .chain(std::iter::once(UTF16_NUL))
        .flat_map(u16::to_le_bytes)
        .collect()
}

/// Hands `bytes` to the clipboard. The clipboard owns the memory only on success.
fn set_hglobal(op: &'static str, format: u32, bytes: &[u8]) -> Result<(), ClipboardError> {
    let hglobal = alloc_hglobal(op, format, bytes)?;
    // SAFETY: the clipboard is open (or we are inside WM_RENDERFORMAT) and hglobal is a fresh allocation.
    unsafe {
        if let Err(e) = SetClipboardData(format, Some(HANDLE(hglobal.0))) {
            if let Err(free_err) = GlobalFree(Some(hglobal)) {
                log::error!(
                    "clipboard {op}: GlobalFree after failed SetClipboardData for format {format}: {free_err}"
                );
            }
            return Err(win_err(
                op,
                "SetClipboardData",
                &format!("format {format}"),
                e,
            ));
        }
    }
    Ok(())
}

unsafe extern "system" fn owner_wndproc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if msg == WM_RENDERFORMAT && wparam.0 as u32 == CF_UNICODETEXT.0 as u32 {
        // SAFETY: GWLP_USERDATA holds the OwnerState pointer set in WindowsClipboard::new; the box outlives the window.
        let state = unsafe { &*(GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const OwnerState) };
        let taken = state
            .pending
            .borrow_mut()
            .as_mut()
            .map(|p| p.text.clone());
        if let Some(text) = taken
            && let Err(e) = set_hglobal(
                "render format",
                CF_UNICODETEXT.0 as u32,
                &utf16_bytes(&text),
            )
        {
            log::error!("{e}");
        }
        return LRESULT(0);
    }
    // SAFETY: default handling for every other message.
    unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
}

impl WindowsClipboard {
    pub fn new() -> Result<Self, ClipboardError> {
        const OP: &str = "create owner window";
        // SAFETY: registering a class and creating a message-only window on the main thread.
        unsafe {
            let instance = GetModuleHandleW(None)
                .map_err(|e| win_err(OP, "GetModuleHandleW", "this module", e))?;
            let class = WNDCLASSW {
                lpfnWndProc: Some(owner_wndproc),
                hInstance: instance.into(),
                lpszClassName: OWNER_CLASS_NAME,
                ..Default::default()
            };
            if RegisterClassW(&class) == 0 {
                return Err(win_err(
                    OP,
                    "RegisterClassW",
                    "PlainPasteClipboardOwner",
                    windows::core::Error::from_thread(),
                ));
            }
            let hwnd = CreateWindowExW(
                WINDOW_EX_STYLE(0),
                OWNER_CLASS_NAME,
                w!(""),
                WINDOW_STYLE(0),
                0,
                0,
                0,
                0,
                Some(HWND_MESSAGE),
                None,
                Some(instance.into()),
                None,
            )
            .map_err(|e| win_err(OP, "CreateWindowExW", "the clipboard owner window", e))?;
            let state = Box::new(OwnerState {
                pending: RefCell::new(None),
            });
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, &*state as *const OwnerState as isize);
            Ok(WindowsClipboard { hwnd, state })
        }
    }
}

impl Clipboard for WindowsClipboard {
    type Snapshot = WindowsSnapshot;

    fn read_text(&mut self) -> Result<Option<String>, ClipboardError> {
        const OP: &str = "read text";
        let _open = OpenClipboardGuard::open(OP, self.hwnd)?;
        let format = CF_UNICODETEXT.0 as u32;
        // SAFETY: the clipboard is open.
        if unsafe { IsClipboardFormatAvailable(format) }.is_err() {
            return Ok(None);
        }
        // SAFETY: the clipboard is open and CF_UNICODETEXT is available.
        let handle = unsafe { GetClipboardData(format) }
            .map_err(|e| win_err(OP, "GetClipboardData", "CF_UNICODETEXT", e))?;
        let bytes = copy_hglobal(OP, format, handle)?;
        let units: Vec<u16> = bytes
            .as_chunks::<2>()
            .0
            .iter()
            .map(|pair| u16::from_le_bytes(*pair))
            .take_while(|unit| *unit != UTF16_NUL)
            .collect();
        String::from_utf16(&units).map(Some).map_err(|e| {
            ClipboardError::new(
                OP,
                format!("CF_UNICODETEXT: expected valid UTF-16, actual {e}"),
            )
        })
    }

    fn write_text(&mut self, text: &str) -> Result<(), ClipboardError> {
        const OP: &str = "write text";
        *self.state.pending.borrow_mut() = None;
        let _open = OpenClipboardGuard::open(OP, self.hwnd)?;
        // SAFETY: the clipboard is open.
        unsafe { EmptyClipboard() }
            .map_err(|e| win_err(OP, "EmptyClipboard", "the clipboard", e))?;
        set_hglobal(OP, CF_UNICODETEXT.0 as u32, &utf16_bytes(text))
    }

    fn snapshot(&mut self) -> Result<WindowsSnapshot, ClipboardError> {
        const OP: &str = "snapshot";
        let _open = OpenClipboardGuard::open(OP, self.hwnd)?;
        let mut formats = Vec::new();
        let mut format = 0u32;
        loop {
            // SAFETY: the clipboard is open; 0 starts the enumeration.
            format = unsafe { EnumClipboardFormats(format) };
            if format == 0 {
                break;
            }
            if GDI_HANDLE_FORMATS.contains(&format) {
                log::info!(
                    "clipboard snapshot: GDI handle format {format} not saved, Windows synthesizes it from its HGLOBAL sibling"
                );
                continue;
            }
            // SAFETY: the clipboard is open and `format` was just enumerated.
            let handle = unsafe { GetClipboardData(format) }
                .map_err(|e| win_err(OP, "GetClipboardData", &format!("format {format}"), e))?;
            let bytes = if format == CF_ENHMETAFILE.0 as u32 {
                let hemf = HENHMETAFILE(handle.0);
                // SAFETY: the handle is an enhanced metafile owned by the clipboard.
                let size = unsafe { GetEnhMetaFileBits(hemf, None) };
                let mut bits = vec![0u8; size as usize];
                // SAFETY: `bits` has exactly `size` bytes.
                if size == 0 || unsafe { GetEnhMetaFileBits(hemf, Some(&mut bits)) } != size {
                    return Err(ClipboardError::new(
                        OP,
                        format!(
                            "GetEnhMetaFileBits for CF_ENHMETAFILE: expected {size} bytes, actual a failed copy"
                        ),
                    ));
                }
                bits
            } else {
                copy_hglobal(OP, format, handle)?
            };
            formats.push((format, bytes));
        }
        Ok(WindowsSnapshot { formats })
    }

    fn offer_lazy_text(&mut self, text: String) -> Result<(), ClipboardError> {
        const OP: &str = "offer lazy text";
        let _open = OpenClipboardGuard::open(OP, self.hwnd)?;
        // SAFETY: the clipboard is open; EmptyClipboard makes self.hwnd the owner.
        unsafe { EmptyClipboard() }
            .map_err(|e| win_err(OP, "EmptyClipboard", "the clipboard", e))?;
        *self.state.pending.borrow_mut() = Some(PendingOffer { text });
        // SAFETY: a NULL handle requests delayed rendering. Delayed rendering returns NULL
        // on success, which the wrapper reports as an error, so success is "last error unchanged
        // from NO_ERROR"; SetLastError resets it first.
        match unsafe {
            SetLastError(WIN32_ERROR(0));
            SetClipboardData(CF_UNICODETEXT.0 as u32, None)
        } {
            Ok(_) => Ok(()),
            Err(e) if e.code() == NO_ERROR => Ok(()),
            Err(e) => Err(win_err(OP, "SetClipboardData(NULL)", "CF_UNICODETEXT", e)),
        }
    }

    fn offer_is_current(&mut self) -> Result<bool, ClipboardError> {
        const OP: &str = "check offer";
        // SAFETY: GetClipboardOwner only reads the owner handle. NULL means "no owner", reported
        // by the wrapper as an error with last error NO_ERROR; SetLastError resets it first.
        match unsafe {
            SetLastError(WIN32_ERROR(0));
            GetClipboardOwner()
        } {
            Ok(owner) => Ok(owner == self.hwnd && self.state.pending.borrow().is_some()),
            Err(e) if e.code() == NO_ERROR => Ok(false),
            Err(e) => Err(win_err(OP, "GetClipboardOwner", "the clipboard", e)),
        }
    }

    fn restore(&mut self, snapshot: WindowsSnapshot) -> Result<(), ClipboardError> {
        const OP: &str = "restore";
        *self.state.pending.borrow_mut() = None;
        let _open = OpenClipboardGuard::open(OP, self.hwnd)?;
        // SAFETY: the clipboard is open.
        unsafe { EmptyClipboard() }
            .map_err(|e| win_err(OP, "EmptyClipboard", "the clipboard", e))?;
        for (format, bytes) in snapshot.formats {
            if format == CF_ENHMETAFILE.0 as u32 {
                // SAFETY: `bytes` came from GetEnhMetaFileBits.
                let hemf = unsafe { SetEnhMetaFileBits(&bytes) };
                if hemf.is_invalid() {
                    return Err(ClipboardError::new(
                        OP,
                        "SetEnhMetaFileBits for CF_ENHMETAFILE: expected a handle, actual NULL",
                    ));
                }
                // SAFETY: the clipboard is open and takes ownership of hemf on success.
                unsafe { SetClipboardData(format, Some(HANDLE(hemf.0))) }
                    .map_err(|e| win_err(OP, "SetClipboardData", "CF_ENHMETAFILE", e))?;
            } else {
                set_hglobal(OP, format, &bytes)?;
            }
        }
        Ok(())
    }
}
