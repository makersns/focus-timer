//! A non-activating AppKit panel hosts the mini timer without switching Spaces.
//! Tauri's hidden window remains alive solely for the shared webview and IPC.
use std::{cell::RefCell, sync::mpsc, time::Duration};

use objc2::{
    available, define_class, msg_send, rc::Retained, runtime::ProtocolObject, DefinedClass,
    MainThreadMarker, MainThreadOnly,
};
use objc2_app_kit::{
    NSAppearance, NSAppearanceCustomization, NSAppearanceNameAqua, NSAppearanceNameDarkAqua,
    NSApplication, NSBackingStoreType, NSColor, NSEvent, NSEventType, NSFloatingWindowLevel,
    NSNormalWindowLevel, NSPanel, NSScreen, NSWindow, NSWindowCollectionBehavior, NSWindowDelegate,
    NSWindowStyleMask,
};
use objc2_foundation::{NSObjectProtocol, NSPoint, NSRect, NSSize, NSString};
use tauri::{AppHandle, Manager, Theme, WebviewWindow};

struct PanelIvars {
    app: AppHandle,
    on_close: fn(&AppHandle),
    mouse_down: RefCell<Option<Retained<NSEvent>>>,
}

define_class!(
    #[unsafe(super(NSPanel))]
    #[thread_kind = MainThreadOnly]
    #[name = "FocusMiniTimerPanel"]
    #[ivars = PanelIvars]
    struct MiniPanel;

    unsafe impl NSObjectProtocol for MiniPanel {}

    impl MiniPanel {
        #[unsafe(method(canBecomeKeyWindow))]
        fn can_become_key(&self) -> bool {
            true
        }

        #[unsafe(method(canBecomeMainWindow))]
        fn can_become_main(&self) -> bool {
            false
        }

        #[unsafe(method(sendEvent:))]
        fn send_event(&self, event: &NSEvent) {
            if event.r#type() == NSEventType::LeftMouseDown {
                // SAFETY: AppKit owns a valid NSEvent for this callback. Retain
                // the original mouse-down so asynchronous IPC can start a drag.
                let event = unsafe { Retained::retain(event as *const NSEvent as *mut NSEvent) };
                *self.ivars().mouse_down.borrow_mut() = event;
            } else if event.r#type() == NSEventType::LeftMouseUp {
                self.ivars().mouse_down.borrow_mut().take();
            }
            // No RefCell borrow survives the native event dispatch.
            unsafe { msg_send![super(self), sendEvent: event] }
        }
    }

    unsafe impl NSWindowDelegate for MiniPanel {
        #[unsafe(method(windowShouldClose:))]
        fn should_close(&self, _window: &NSWindow) -> bool {
            let app = self.ivars().app.clone();
            let callback = self.ivars().on_close;
            // Leave AppKit's delegate stack before changing shared preferences.
            // Dispatching from a worker also avoids immediate main-thread execution.
            tauri::async_runtime::spawn(async move {
                let handle = app.clone();
                let _ = app.run_on_main_thread(move || callback(&handle));
            });
            false
        }
    }
);

impl MiniPanel {
    fn new(
        mtm: MainThreadMarker,
        frame: NSRect,
        app: AppHandle,
        on_close: fn(&AppHandle),
    ) -> Option<Retained<Self>> {
        let this = Self::alloc(mtm).set_ivars(PanelIvars {
            app,
            on_close,
            mouse_down: RefCell::new(None),
        });
        // SAFETY: A real NSPanel subclass is allocated and initialized using
        // AppKit's designated initializer. Its class never changes at runtime.
        unsafe {
            msg_send![super(this),
                initWithContentRect: frame,
                styleMask: NSWindowStyleMask::Borderless | NSWindowStyleMask::NonactivatingPanel,
                backing: NSBackingStoreType::Buffered,
                defer: false
            ]
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
struct Applied {
    theme: Option<Theme>,
    size: (f64, f64),
    pinned: bool,
    visible: bool,
}

struct Host {
    panel: Retained<MiniPanel>,
    applied: Option<Applied>,
}

thread_local! {
    static HOST: RefCell<Option<Host>> = const { RefCell::new(None) };
}

pub fn initialize(window: &WebviewWindow, on_close: fn(&AppHandle)) -> Result<(), String> {
    let mtm = MainThreadMarker::new().ok_or("widget_main_thread_unavailable")?;
    if HOST.with(|host| host.borrow().is_some()) {
        return Ok(());
    }
    let raw = window
        .ns_window()
        .map_err(|_| "widget_window_unavailable")?;
    // SAFETY: Tauri retains this NSWindow while the hidden resource owner lives.
    let owner = unsafe { &*raw.cast::<NSWindow>() };
    let content = owner.contentView().ok_or("widget_content_unavailable")?;
    let panel = MiniPanel::new(mtm, owner.frame(), window.app_handle().clone(), on_close)
        .ok_or("widget_panel_unavailable")?;
    panel.setTitle(&NSString::from_str("Focus — Mini"));
    // SAFETY: HOST explicitly retains the panel; native Close must not release it.
    unsafe { panel.setReleasedWhenClosed(false) };
    panel.setHidesOnDeactivate(false);
    panel.setBecomesKeyOnlyIfNeeded(false);
    panel.setOpaque(false);
    panel.setBackgroundColor(Some(&NSColor::clearColor()));
    panel.setHasShadow(false);
    panel.setMovable(true);
    panel.setMovableByWindowBackground(false);
    panel.setRestorable(false);
    panel.setDelegate(Some(ProtocolObject::from_ref(&*panel)));

    owner.orderOut(None);
    owner.setContentView(None);
    panel.setContentView(Some(&content));
    HOST.with(|host| {
        *host.borrow_mut() = Some(Host {
            panel,
            applied: None,
        });
    });
    Ok(())
}

fn collection_behavior(pinned: bool, joins_applications: bool) -> NSWindowCollectionBehavior {
    if pinned {
        let mut behavior = NSWindowCollectionBehavior::CanJoinAllSpaces
            | NSWindowCollectionBehavior::FullScreenAuxiliary
            | NSWindowCollectionBehavior::Transient
            | NSWindowCollectionBehavior::IgnoresCycle;
        if joins_applications {
            behavior |= NSWindowCollectionBehavior::CanJoinAllApplications;
        }
        behavior
    } else {
        NSWindowCollectionBehavior::Managed
            | NSWindowCollectionBehavior::FullScreenNone
            | NSWindowCollectionBehavior::IgnoresCycle
    }
}

fn resized_frame(previous: NSRect, size: (f64, f64), work: Option<NSRect>) -> NSRect {
    // AppKit uses logical points with a bottom-left origin. Preserve the top-left
    // corner when changing between the ring, tomato, and shorter capsule.
    let mut x = previous.origin.x;
    let mut y = previous.origin.y + previous.size.height - size.1;
    if let Some(work) = work {
        let min_x = work.origin.x + 12.0;
        let min_y = work.origin.y + 12.0;
        let max_x = (work.origin.x + work.size.width - size.0 - 12.0).max(min_x);
        let max_y = (work.origin.y + work.size.height - size.1 - 12.0).max(min_y);
        x = x.clamp(min_x, max_x);
        y = y.clamp(min_y, max_y);
    }
    NSRect::new(NSPoint::new(x, y), NSSize::new(size.0, size.1))
}

fn apply_theme(panel: &MiniPanel, theme: Option<Theme>) {
    let appearance = match theme {
        // SAFETY: Both appearance names exist on our minimum supported macOS.
        Some(Theme::Dark) => unsafe { NSAppearance::appearanceNamed(NSAppearanceNameDarkAqua) },
        Some(Theme::Light) => unsafe { NSAppearance::appearanceNamed(NSAppearanceNameAqua) },
        _ => None,
    };
    panel.setAppearance(appearance.as_deref());
}

pub fn sync(
    theme: Option<Theme>,
    size: (f64, f64),
    pinned: bool,
    visible: bool,
    explicit_show: bool,
) -> Result<(), String> {
    let mtm = MainThreadMarker::new().ok_or("widget_main_thread_unavailable")?;
    if !size.0.is_finite() || !size.1.is_finite() || size.0 <= 0.0 || size.1 <= 0.0 {
        return Err("widget_size_invalid".to_owned());
    }
    let next = Applied {
        theme,
        size,
        pinned,
        visible,
    };
    let (panel, previous) = HOST.with(|host| {
        let mut slot = host.borrow_mut();
        let host = slot.as_mut().ok_or("widget_panel_unavailable")?;
        let previous = host.applied.replace(next);
        Ok::<_, String>((host.panel.clone(), previous))
    })?;
    // All AppKit operations below happen after the HOST borrow has ended.
    if previous.is_none_or(|old| old.theme != theme) {
        apply_theme(&panel, theme);
    }
    if previous.is_none_or(|old| old.size != size) {
        let work = panel
            .screen()
            .or_else(|| NSScreen::mainScreen(mtm))
            .map(|screen| screen.visibleFrame());
        panel.setFrame_display(resized_frame(panel.frame(), size, work), panel.isVisible());
    }
    let pin_changed = previous.is_none_or(|old| old.pinned != pinned);
    if pin_changed {
        panel.setFloatingPanel(pinned);
        panel.setLevel(if pinned {
            NSFloatingWindowLevel
        } else {
            NSNormalWindowLevel
        });
        // macOS 13 introduced the explicit floating/system-overlay behavior.
        panel.setCollectionBehavior(collection_behavior(pinned, available!(macos = 13.0)));
    }
    let visibility_changed = previous.is_none_or(|old| old.visible != visible);
    if visible && (visibility_changed || explicit_show || (pin_changed && pinned)) {
        // Showing or pinning is explicit. Theme/size changes and timer ticks do
        // not raise the panel or activate Focus's ordinary main window.
        panel.orderFrontRegardless();
    } else if !visible && visibility_changed {
        panel.orderOut(None);
    }
    Ok(())
}

fn drag_on_main() -> Result<(), String> {
    let mtm = MainThreadMarker::new().ok_or("widget_main_thread_unavailable")?;
    let panel = HOST
        .with(|host| host.borrow().as_ref().map(|host| host.panel.clone()))
        .ok_or("widget_panel_unavailable")?;
    if !panel.isVisible() || NSEvent::pressedMouseButtons() & 1 == 0 {
        return Err("widget_drag_unavailable".to_owned());
    }
    let original = panel.ivars().mouse_down.borrow_mut().take();
    let event = if let Some(event) = original {
        event
    } else {
        let event = NSApplication::sharedApplication(mtm)
            .currentEvent()
            .ok_or("widget_drag_unavailable")?;
        if event.r#type() == NSEventType::LeftMouseDown
            && event.windowNumber() == panel.windowNumber()
        {
            event
        } else if event.r#type().0 == 0x15 {
            // Tao handles this AppKit synthetic event the same way. Use this
            // panel's window number and local position, never an unrelated window.
            let location = panel.convertPointFromScreen(NSEvent::mouseLocation());
            let frame = panel.frame();
            if location.x < 0.0
                || location.y < 0.0
                || location.x > frame.size.width
                || location.y > frame.size.height
            {
                return Err("widget_drag_unavailable".to_owned());
            }
            NSEvent::mouseEventWithType_location_modifierFlags_timestamp_windowNumber_context_eventNumber_clickCount_pressure(
                NSEventType::LeftMouseDown,
                location,
                event.modifierFlags(),
                event.timestamp(),
                panel.windowNumber(),
                None,
                0,
                1,
                1.0,
            )
            .ok_or("widget_drag_unavailable")?
        } else {
            return Err("widget_drag_unavailable".to_owned());
        }
    };
    // AppKit hands tracking to WindowServer and returns immediately.
    panel.performWindowDragWithEvent(&event);
    Ok(())
}

pub fn drag(app: &AppHandle) -> Result<(), String> {
    if MainThreadMarker::new().is_some() {
        return drag_on_main();
    }
    let (send, receive) = mpsc::sync_channel(1);
    app.run_on_main_thread(move || {
        let _ = send.send(drag_on_main());
    })
    .map_err(|_| "widget_main_thread_unavailable".to_owned())?;
    receive
        .recv_timeout(Duration::from_secs(2))
        .map_err(|_| "widget_drag_unavailable".to_owned())?
}

#[cfg(test)]
mod tests {
    use super::{
        collection_behavior, resized_frame, NSPoint, NSRect, NSSize, NSWindowCollectionBehavior,
    };

    #[test]
    fn widget_resizing_preserves_the_top_left_without_screen_scale_conversion() {
        let before = NSRect::new(NSPoint::new(400.0, 200.0), NSSize::new(260.0, 260.0));
        let after = resized_frame(before, (340.0, 112.0), None);
        assert_eq!(after.origin, NSPoint::new(400.0, 348.0));
        assert_eq!(after.size, NSSize::new(340.0, 112.0));
        assert_eq!(
            after.origin.y + after.size.height,
            before.origin.y + before.size.height
        );
    }

    #[test]
    fn larger_widgets_stay_inside_negative_origin_display_work_areas() {
        let before = NSRect::new(NSPoint::new(-180.0, -90.0), NSSize::new(100.0, 80.0));
        let work = NSRect::new(NSPoint::new(-1920.0, -1080.0), NSSize::new(1920.0, 1056.0));
        let after = resized_frame(before, (340.0, 280.0), Some(work));
        assert_eq!(after.origin, NSPoint::new(-352.0, -316.0));
        assert_eq!(after.origin.y + after.size.height, -36.0);
    }

    #[test]
    fn unpinning_removes_other_app_fullscreen_and_all_spaces_membership() {
        let pinned = collection_behavior(true, true);
        assert!(pinned.contains(NSWindowCollectionBehavior::CanJoinAllSpaces));
        assert!(pinned.contains(NSWindowCollectionBehavior::CanJoinAllApplications));
        assert!(pinned.contains(NSWindowCollectionBehavior::FullScreenAuxiliary));
        let legacy = collection_behavior(true, false);
        assert!(!legacy.contains(NSWindowCollectionBehavior::CanJoinAllApplications));
        let unpinned = collection_behavior(false, true);
        assert!(!unpinned.contains(NSWindowCollectionBehavior::CanJoinAllSpaces));
        assert!(!unpinned.contains(NSWindowCollectionBehavior::CanJoinAllApplications));
        assert!(!unpinned.contains(NSWindowCollectionBehavior::FullScreenAuxiliary));
        assert!(unpinned.contains(NSWindowCollectionBehavior::FullScreenNone));
    }
}
