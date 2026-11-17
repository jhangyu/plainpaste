//! NSPasteboard implementation. Lazy provision uses NSPasteboardItemDataProvider.

use objc2::rc::Retained;
use objc2::runtime::{NSObject, NSObjectProtocol, ProtocolObject};
use objc2::{AllocAnyThread, DefinedClass, define_class, msg_send};
use objc2_app_kit::{
    NSPasteboard, NSPasteboardItem, NSPasteboardItemDataProvider, NSPasteboardType,
    NSPasteboardTypeString, NSPasteboardWriting,
};
use objc2_foundation::{NSArray, NSData, NSString};

use super::{Clipboard, ClipboardError};

/// One pasteboard item: (UTI type, bytes) pairs in the order the item lists them.
struct SnapshotItem {
    formats: Vec<(String, Vec<u8>)>,
}

pub struct MacosSnapshot {
    items: Vec<SnapshotItem>,
}

struct ProviderIvars {
    text: String,
}

define_class!(
    // SAFETY: NSObject has no subclassing requirements and TextProvider does not implement Drop.
    #[unsafe(super(NSObject))]
    #[name = "PlainPasteTextProvider"]
    #[ivars = ProviderIvars]
    struct TextProvider;

    unsafe impl NSObjectProtocol for TextProvider {}

    unsafe impl NSPasteboardItemDataProvider for TextProvider {
        #[unsafe(method(pasteboard:item:provideDataForType:))]
        fn provide_data_for_type(
            &self,
            _pasteboard: Option<&NSPasteboard>,
            item: &NSPasteboardItem,
            data_type: &NSPasteboardType,
        ) {
            let text = NSString::from_str(&self.ivars().text);
            if !item.setString_forType(&text, data_type) {
                log::error!(
                    "clipboard provide-data failed: NSPasteboardItem setString:forType:{data_type} returned NO"
                );
            }
        }
    }
);

impl TextProvider {
    fn new(text: String) -> Retained<Self> {
        let this = Self::alloc().set_ivars(ProviderIvars { text });
        // SAFETY: NSObject's designated initializer is `init`.
        unsafe { msg_send![super(this), init] }
    }
}

pub struct MacosClipboard {
    pasteboard: Retained<NSPasteboard>,
    /// The last lazy offer; `None` when no offer is on the pasteboard.
    offer: Option<LazyOffer>,
}

struct LazyOffer {
    /// Kept alive while its offer is on the pasteboard.
    _provider: Retained<TextProvider>,
    /// `changeCount` recorded right after the offer was written.
    change_count: isize,
}

impl MacosClipboard {
    pub fn new() -> Result<Self, ClipboardError> {
        Ok(MacosClipboard {
            pasteboard: NSPasteboard::generalPasteboard(),
            offer: None,
        })
    }

    fn write_items(
        &self,
        op: &'static str,
        items: &[Retained<NSPasteboardItem>],
    ) -> Result<(), ClipboardError> {
        self.pasteboard.clearContents();
        let writers: Vec<Retained<ProtocolObject<dyn NSPasteboardWriting>>> = items
            .iter()
            .map(|item| ProtocolObject::from_retained(item.clone()))
            .collect();
        if self
            .pasteboard
            .writeObjects(&NSArray::from_retained_slice(&writers))
        {
            Ok(())
        } else {
            Err(ClipboardError::new(
                op,
                format!(
                    "NSPasteboard writeObjects: expected YES, actual NO for {} item(s)",
                    items.len()
                ),
            ))
        }
    }
}

impl Clipboard for MacosClipboard {
    type Snapshot = MacosSnapshot;

    fn read_text(&mut self) -> Result<Option<String>, ClipboardError> {
        // SAFETY: NSPasteboardTypeString is an immutable AppKit constant.
        let string_type = unsafe { NSPasteboardTypeString };
        Ok(self
            .pasteboard
            .stringForType(string_type)
            .map(|s| s.to_string()))
    }

    fn write_text(&mut self, text: &str) -> Result<(), ClipboardError> {
        self.offer = None;
        self.pasteboard.clearContents();
        // SAFETY: NSPasteboardTypeString is an immutable AppKit constant.
        let string_type = unsafe { NSPasteboardTypeString };
        if self
            .pasteboard
            .setString_forType(&NSString::from_str(text), string_type)
        {
            Ok(())
        } else {
            Err(ClipboardError::new(
                "write text",
                format!("NSPasteboard setString:forType:{string_type}: expected YES, actual NO"),
            ))
        }
    }

    fn snapshot(&mut self) -> Result<MacosSnapshot, ClipboardError> {
        let Some(items) = self.pasteboard.pasteboardItems() else {
            return Err(ClipboardError::new(
                "snapshot",
                "NSPasteboard pasteboardItems: expected an item array, actual nil",
            ));
        };
        let mut snapshot = MacosSnapshot { items: Vec::new() };
        for (index, item) in items.iter().enumerate() {
            let mut formats = Vec::new();
            for data_type in item.types().iter() {
                // macOS also advertises synthesized conversion types (for example
                // public.utf16-external-plain-text) that return nil. The pasteboard
                // re-synthesizes them from the restored source types.
                match item.dataForType(&data_type) {
                    Some(data) => formats.push((data_type.to_string(), data.to_vec())),
                    None => log::info!(
                        "clipboard snapshot: NSPasteboardItem #{index} type {data_type} has no data, not saved"
                    ),
                }
            }
            snapshot.items.push(SnapshotItem { formats });
        }
        Ok(snapshot)
    }

    fn offer_lazy_text(&mut self, text: String) -> Result<(), ClipboardError> {
        const OP: &str = "offer lazy text";
        let provider = TextProvider::new(text);
        let item = NSPasteboardItem::new();
        // SAFETY: NSPasteboardTypeString is an immutable AppKit constant.
        let string_type = unsafe { NSPasteboardTypeString };
        let types = NSArray::from_slice(&[string_type]);
        if !item.setDataProvider_forTypes(ProtocolObject::from_ref(&*provider), &types) {
            return Err(ClipboardError::new(
                OP,
                format!(
                    "NSPasteboardItem setDataProvider:forTypes:[{string_type}]: expected YES, actual NO"
                ),
            ));
        }
        self.write_items(OP, &[item])?;
        self.offer = Some(LazyOffer {
            _provider: provider,
            change_count: self.pasteboard.changeCount(),
        });
        Ok(())
    }

    fn offer_is_current(&mut self) -> Result<bool, ClipboardError> {
        Ok(self.offer.as_ref().map(|o| o.change_count) == Some(self.pasteboard.changeCount()))
    }

    fn restore(&mut self, snapshot: MacosSnapshot) -> Result<(), ClipboardError> {
        const OP: &str = "restore";
        self.offer = None;
        let mut items = Vec::with_capacity(snapshot.items.len());
        for (index, saved) in snapshot.items.into_iter().enumerate() {
            let item = NSPasteboardItem::new();
            for (data_type, bytes) in saved.formats {
                if !item.setData_forType(&NSData::from_vec(bytes), &NSString::from_str(&data_type))
                {
                    return Err(ClipboardError::new(
                        OP,
                        format!(
                            "NSPasteboardItem #{index} setData:forType:{data_type}: expected YES, actual NO"
                        ),
                    ));
                }
            }
            items.push(item);
        }
        if items.is_empty() {
            self.pasteboard.clearContents();
            return Ok(());
        }
        self.write_items(OP, &items)
    }
}
