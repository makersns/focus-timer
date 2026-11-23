//! The menu belongs to its status item, not to a normal desktop window.
//! AppKit keeps the arrow, display scale and full-screen Space in sync.
use std::{
    cell::{Cell, RefCell},
    time::{Duration, Instant},
};

use objc2::{
    define_class, msg_send, rc::Retained, runtime::ProtocolObject, MainThreadMarker, MainThreadOnly,
};
use objc2_app_kit::{
    NSAppearance, NSAppearanceNameAqua, NSAppearanceNameDarkAqua, NSPopUpMenuWindowLevel,
    NSPopover, NSPopoverBehavior, NSPopoverDelegate, NSViewController, NSWindow,
    NSWindowCollectionBehavior,
};
use objc2_foundation::{NSNotification, NSObject, NSObjectProtocol, NSRectEdge, NSSize, NSString};
use tauri::{AppHandle, Manager, Theme, WebviewWindow};

const WIDTH: f64 = 264.0;
const HEIGHT: f64 = 252.0;

thread_local! {
    // These AppKit objects never leave the main thread. The hidden Tauri window
    // remains alive as the owner of its webview and IPC resources.
    static HOST: RefCell<Option<PopoverHost>> = const { RefCell::new(None) };
    static LAST_CLOSED: Cell<Option<Instant>> = const { Cell::new(None) };
}

define_class!(
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "FocusMenuPopoverDelegate"]
    struct MenuDelegate;

    unsafe impl NSObjectProtocol for MenuDelegate {}

    unsafe impl NSPopoverDelegate for MenuDelegate {
        #[unsafe(method(popoverWillShow:))]
        fn will_show(&self, _notification: &NSNotification) {
            configure_current_panel();
        }

        #[unsafe(method(popoverDidClose:))]
        fn did_close(&self, _notification: &NSNotification) {
            LAST_CLOSED.with(|closed| closed.set(Some(Instant::now())));
        }
    }
);

impl MenuDelegate {
    fn new(mtm: MainThreadMarker) -> Retained<Self> {
        // SAFETY: NSObject initialization is performed on our allocated,
        // main-thread-only subclass.
        unsafe { msg_send![Self::alloc(mtm), init] }
    }
}

struct PopoverHost {
    popover: Retained<NSPopover>,
    controller: Retained<NSViewController>,
    _delegate: Retained<MenuDelegate>,
}

fn configure_current_panel() {
    // Do not keep a RefCell borrow across an AppKit call: AppKit can synchronously
    // invoke delegate callbacks while showing or closing a popover.
    let controller = HOST.with(|host| host.borrow().as_ref().map(|h| h.controller.clone()));
    if let Some(window) = controller.and_then(|controller| controller.view().window()) {
        window.setTitle(&NSString::from_str("Focus — Menu"));
        window.setLevel(NSPopUpMenuWindowLevel);
        window.setCollectionBehavior(
            NSWindowCollectionBehavior::CanJoinAllSpaces
                | NSWindowCollectionBehavior::FullScreenAuxiliary
                | NSWindowCollectionBehavior::Transient
                | NSWindowCollectionBehavior::IgnoresCycle,
        );
    }
}

fn initialize(window: &WebviewWindow, mtm: MainThreadMarker) -> Result<(), String> {
    if HOST.with(|host| host.borrow().is_some()) {
        return Ok(());
    }
    let native = window.ns_window().map_err(|_| "menu_window_unavailable")?;
    // SAFETY: Tauri retains this NSWindow for the lifetime of its WebviewWindow.
    // All access occurs on the main thread. We transfer only the content view;
    // the original window and Tauri webview resource remain alive.
    let native = unsafe { &*native.cast::<NSWindow>() };
    let content = native.contentView().ok_or("menu_content_unavailable")?;
    let controller = NSViewController::new(mtm);
    native.setContentView(None);
    controller.setView(&content);
    controller.setPreferredContentSize(NSSize::new(WIDTH, HEIGHT));

    let delegate = MenuDelegate::new(mtm);
    let popover = NSPopover::new(mtm);
    popover.setContentViewController(Some(&controller));
    popover.setContentSize(NSSize::new(WIDTH, HEIGHT));
    popover.setBehavior(NSPopoverBehavior::Transient);
    popover.setAnimates(false);
    popover.setDelegate(Some(ProtocolObject::from_ref(&*delegate)));

    HOST.with(|host| {
        *host.borrow_mut() = Some(PopoverHost {
            popover,
            controller,
            _delegate: delegate,
        });
    });
    // Only a constant attribute is evaluated. AppKit supplies the arrow and
    // surrounding chrome, so the HTML card needs no second arrow or outer gutter.
    let _ = window.eval("document.documentElement.dataset.nativePopover = 'true'");
    Ok(())
}

fn apply_theme(popover: &NSPopover, theme: Option<Theme>) {
    let appearance = match theme {
        // SAFETY: These AppKit appearance-name constants are always available
        // on our supported macOS versions.
        Some(Theme::Dark) => unsafe { NSAppearance::appearanceNamed(NSAppearanceNameDarkAqua) },
        Some(Theme::Light) => unsafe { NSAppearance::appearanceNamed(NSAppearanceNameAqua) },
        _ => None,
    };
    popover.setAppearance(appearance.as_deref());
}

pub fn set_theme(app: &AppHandle, theme: Option<Theme>) {
    let _ = app.run_on_main_thread(move || {
        let popover = HOST.with(|host| host.borrow().as_ref().map(|h| h.popover.clone()));
        if let Some(popover) = popover {
            apply_theme(&popover, theme);
        }
    });
}

pub fn hide(app: &AppHandle) {
    let _ = app.run_on_main_thread(|| {
        let popover = HOST.with(|host| host.borrow().as_ref().map(|h| h.popover.clone()));
        if let Some(popover) = popover {
            popover.close();
        }
    });
}

pub fn toggle(app: &AppHandle, theme: Option<Theme>) -> Result<(), String> {
    let window = app
        .get_webview_window("tray")
        .ok_or("menu_window_unavailable")?;
    let tray = app
        .tray_by_id("focus")
        .ok_or("menu_status_item_unavailable")?;
    tray.with_inner_tray_icon(move |inner| -> Result<(), String> {
        let mtm = MainThreadMarker::new().ok_or("menu_main_thread_unavailable")?;
        let status = inner
            .ns_status_item()
            .ok_or("menu_status_item_unavailable")?;
        let button = status.button(mtm).ok_or("menu_status_button_unavailable")?;
        initialize(&window, mtm)?;
        let popover = HOST
            .with(|host| host.borrow().as_ref().map(|h| h.popover.clone()))
            .ok_or("menu_popover_unavailable")?;
        if popover.isShown() {
            popover.close();
            return Ok(());
        }
        // AppKit can dismiss a transient popover on MouseDown, before the tray's
        // MouseUp callback. That click must close it rather than reopen it.
        if LAST_CLOSED.with(|closed| {
            closed
                .get()
                .is_some_and(|at| at.elapsed() < Duration::from_millis(180))
        }) {
            return Ok(());
        }
        apply_theme(&popover, theme);
        popover.showRelativeToRect_ofView_preferredEdge(button.bounds(), &button, NSRectEdge::MinY);
        configure_current_panel();
        if popover.isShown() {
            Ok(())
        } else {
            Err("menu_anchor_not_visible".to_owned())
        }
    })
    .map_err(|_| "menu_main_thread_unavailable".to_owned())?
}
