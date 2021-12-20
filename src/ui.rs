use crate::state::EditorState;

use std::sync::Arc;

use baseview::{Size, WindowHandle, WindowOpenOptions, WindowScalePolicy};
use egui::CtxRef;
use egui_baseview::{EguiWindow, Queue, RenderSettings, Settings};
use log;
use raw_window_handle::{HasRawWindowHandle, RawWindowHandle};
use vst::editor::Editor;

const WINDOW_WIDTH: usize = 1024;
const WINDOW_HEIGHT: usize = 512;

pub struct TestPluginEditor {
    pub state: Arc<EditorState>,
    pub window_handle: Option<WindowHandle>,
    pub is_open: bool,
}

pub fn settings() -> Settings {
    Settings {
        window: WindowOpenOptions {
            title: String::from("imgui-baseview demo window"),
            size: Size::new(WINDOW_WIDTH as f64, WINDOW_HEIGHT as f64),
            scale: WindowScalePolicy::SystemScaleFactor,
        },
        render_settings: RenderSettings::default(),
    }
}

pub fn update() -> impl FnMut(&egui::CtxRef, &mut Queue, &mut Arc<EditorState>) {
    |egui_ctx: &CtxRef, _queue: &mut Queue, state: &mut Arc<EditorState>| {
        egui::Window::new("egui-baseview simple demo").show(&egui_ctx, |ui| {
            let mut midi_events = state.midi_consumer.lock().unwrap();

            // TODO could be dealing with lots of midi_events, not just one
            if let Some(n) = midi_events.pop() {
                log::info!("found midi data: {:?}", n);
                match n[0] {
                    // note on
                    144 => *state.last_note.lock().unwrap() = n,
                    // note off
                    128 => (),
                    _ => (),
                }
            }

            ui.heading(format!(
                "midi data: {:?}",
                state.last_note.lock().unwrap()[1]
            ));

            let mut val = state.params.amplitude.get();
            if ui
                .add(egui::Slider::new(&mut val, 0.0..=1.0).text("Gain"))
                .changed()
            {
                log::info!("changed amplitude");
                state.params.amplitude.set(val)
            }
        });
    }
}

impl Editor for TestPluginEditor {
    fn position(&self) -> (i32, i32) {
        (0, 0)
    }

    fn size(&self) -> (i32, i32) {
        (WINDOW_WIDTH as i32, WINDOW_HEIGHT as i32)
    }

    fn open(&mut self, parent: *mut ::std::ffi::c_void) -> bool {
        log::info!("Editor open");
        if self.is_open {
            return false;
        }

        self.is_open = true;

        let window_handle = EguiWindow::open_parented(
            &VstParent(parent),
            settings(),
            self.state.clone(),
            |_egui_ctx: &CtxRef, _queue: &mut Queue, _state: &mut Arc<EditorState>| {},
            update(),
        );

        self.window_handle = Some(window_handle);

        true
    }

    fn is_open(&mut self) -> bool {
        self.is_open
    }

    fn close(&mut self) {
        self.is_open = false;
        if let Some(mut window_handle) = self.window_handle.take() {
            window_handle.close();
        }
    }
}

pub struct VstParent(pub *mut ::std::ffi::c_void);

#[cfg(target_os = "macos")]
unsafe impl HasRawWindowHandle for VstParent {
    fn raw_window_handle(&self) -> RawWindowHandle {
        use raw_window_handle::macos::MacOSHandle;

        RawWindowHandle::MacOS(MacOSHandle {
            ns_view: self.0 as *mut ::std::ffi::c_void,
            ..MacOSHandle::empty()
        })
    }
}

#[cfg(target_os = "windows")]
unsafe impl HasRawWindowHandle for VstParent {
    fn raw_window_handle(&self) -> RawWindowHandle {
        use raw_window_handle::windows::WindowsHandle;

        RawWindowHandle::Windows(WindowsHandle {
            hwnd: self.0,
            ..WindowsHandle::empty()
        })
    }
}

#[cfg(target_os = "linux")]
unsafe impl HasRawWindowHandle for VstParent {
    fn raw_window_handle(&self) -> RawWindowHandle {
        use raw_window_handle::unix::XcbHandle;

        RawWindowHandle::Xcb(XcbHandle {
            window: self.0 as u32,
            ..XcbHandle::empty()
        })
    }
}
