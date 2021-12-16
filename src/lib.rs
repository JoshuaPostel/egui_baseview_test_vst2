//! Barebones baseview egui plugin

#[macro_use]
extern crate vst;

use egui::CtxRef;

use baseview::{Size, WindowHandle, WindowOpenOptions, WindowScalePolicy};
use vst::buffer::AudioBuffer;
use vst::editor::Editor;
use vst::plugin::{CanDo, Category, Info, Plugin, PluginParameters, HostCallback};
use vst::util::AtomicFloat;
use vst::host::Host;

use vst::api::Events;
use vst::event::Event;
use vst::event::MidiEvent;
use ringbuf::{Producer, Consumer, RingBuffer};
use std::sync::Mutex;
use log;
use simplelog;


use egui_baseview::{EguiWindow, Queue, RenderSettings, Settings};
use raw_window_handle::{HasRawWindowHandle, RawWindowHandle};

use std::sync::Arc;

const WINDOW_WIDTH: usize = 1024;
const WINDOW_HEIGHT: usize = 512;

struct EditorState {
    params: Arc<GainEffectParameters>,
    midi_consumer: Arc<Mutex<Consumer<[u8; 3]>>>,
    last_note: Arc<Mutex<[u8; 3]>>,
}

struct TestPluginEditor {
    state: Arc<EditorState>,
    window_handle: Option<WindowHandle>,
    is_open: bool,
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

        let settings = Settings {
            window: WindowOpenOptions {
                title: String::from("imgui-baseview demo window"),
                size: Size::new(WINDOW_WIDTH as f64, WINDOW_HEIGHT as f64),
                scale: WindowScalePolicy::SystemScaleFactor,
            },
            render_settings: RenderSettings::default(),
        };
                    
        let window_handle = EguiWindow::open_parented(
            &VstParent(parent),
            settings,
            self.state.clone(),
            |_egui_ctx: &CtxRef, _queue: &mut Queue, _state: &mut Arc<EditorState>| {},
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
                            //128 => *state.last_note.lock().unwrap() = n,
                            _ => (),
                        }
                    }

                    ui.heading(format!("midi data: {:?}", state.last_note.lock().unwrap()[1]));

                    let mut val = state.params.amplitude.get();
                    if ui
                        .add(egui::Slider::new(&mut val, 0.0..=1.0).text("Gain"))
                        .changed()
                    {
                        log::info!("changed amplitude");
                        state.params.amplitude.set(val)
                    }
                });
            },
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
struct GainEffectParameters {
    // The plugin's state consists of a single parameter: amplitude.
    amplitude: AtomicFloat,
}
struct TestPlugin {
    params: Arc<GainEffectParameters>,
    editor: Option<TestPluginEditor>,
    midi_producer: Producer<[u8; 3]>,
    host: HostCallback,
}

impl TestPlugin {
    fn new(host: HostCallback) -> Self {
        let midi_ring = RingBuffer::<[u8; 3]>::new(1_000);
        let (midi_producer, midi_consumer) = midi_ring.split();
        let params = Arc::new(GainEffectParameters::default());
        let state = EditorState {
            params: params.clone(),
            midi_consumer: Arc::new(Mutex::new(midi_consumer)),
            last_note: Arc::new(Mutex::new([0, 0, 0])),
        };
        Self {
            params: params.clone(),
            editor: Some(TestPluginEditor {
                state: Arc::new(state),
                window_handle: None,
                is_open: false,
            }),
            midi_producer,
            host,
        }
    }
}

impl Default for TestPlugin {
    fn default() -> Self {
        let midi_ring = RingBuffer::<[u8; 3]>::new(1_000);
        let (midi_producer, midi_consumer) = midi_ring.split();
        let params = Arc::new(GainEffectParameters::default());
        let state = EditorState {
            params: params.clone(),
            midi_consumer: Arc::new(Mutex::new(midi_consumer)),
            last_note: Arc::new(Mutex::new([0, 0, 0])),
        };
        Self {
            params: params.clone(),
            editor: Some(TestPluginEditor {
                state: Arc::new(state),
                window_handle: None,
                is_open: false,
            }),
            midi_producer,
            host: HostCallback::default(),
        }
    }
}

impl Default for GainEffectParameters {
    fn default() -> GainEffectParameters {
        GainEffectParameters {
            amplitude: AtomicFloat::new(0.5),
        }
    }
}

impl Plugin for TestPlugin {
    fn new(host: HostCallback) -> Self {
        TestPlugin::new(host)
    }

    fn get_info(&self) -> Info {
        log::info!("called get_info");
        Info {
            name: "Egui Gain Effect in Rust".to_string(),
            vendor: "DGriffin".to_string(),
            unique_id: 243123073,
            version: 3,
            inputs: 2,
            outputs: 2,
            // This `parameters` bit is important; without it, none of our
            // parameters will be shown!
            parameters: 2,
            category: Category::Effect,
            ..Default::default()
        }
    }

    fn init(&mut self) {
        let log_folder = ::dirs::home_dir().unwrap().join("tmp");

        //::std::fs::create_dir(log_folder.clone()).expect("create tmp");

        let log_file = ::std::fs::File::create(log_folder.join("EGUIBaseviewTest.log")).unwrap();

        let log_config = simplelog::ConfigBuilder::new()
            .set_time_to_local(true)
            .build();

        let _ = simplelog::WriteLogger::init(simplelog::LevelFilter::Info, log_config, log_file);

        log::info!("init 4");
    }

    fn process_events(&mut self, events: &Events) {
        //log::info!("called process_events");
        let mut mutated_events: Vec<MidiEvent> = vec![];
        for e in events.events() {
            match e {
                Event::Midi(MidiEvent { data, .. }) => {
                    log::info!("got midi event: {:?}", data);
                    self.midi_producer.push(data).unwrap_or(());
                    mutated_events.push(MidiEvent { 
                        data: [data[0], data[1] + 1, data[2]],
                        delta_frames: 0,
                        live: false,
                        note_length: None,
                        note_offset: None,
                        detune: 0,
                        note_off_velocity: 0,
                    });
                },
                _ => (),
            }
        }
//        let new_events = Events { 
//            num_events: mutated_events.len() as i32,
//            _reserved: 0,
//            events: mutated_events,
//        };
        let new_events = Events { 
            num_events: 1,
            _reserved: 0,
            events: [mutated_events[0], 1],
        };
        self.host.process_events(&new_events);
    }

    fn get_editor(&mut self) -> Option<Box<dyn Editor>> {
        log::info!("called get_editor");
        if let Some(editor) = self.editor.take() {
            Some(Box::new(editor) as Box<dyn Editor>)
        } else {
            None
        }
    }

    // Here is where the bulk of our audio processing code goes.
    fn process(&mut self, buffer: &mut AudioBuffer<f32>) {
        //log::info!("called process");
        // Read the amplitude from the parameter object
        let amplitude = self.params.amplitude.get();
        // First, we destructure our audio buffer into an arbitrary number of
        // input and output buffers.  Usually, we'll be dealing with stereo (2 of each)
        // but that might change.
        for (input_buffer, output_buffer) in buffer.zip() {
            // Next, we'll loop through each individual sample so we can apply the amplitude
            // value to it.
            for (input_sample, output_sample) in input_buffer.iter().zip(output_buffer) {
                *output_sample = *input_sample * amplitude;
            }
        }
    }

    // Return the parameter object. This method can be omitted if the
    // plugin has no parameters.
    fn get_parameter_object(&mut self) -> Arc<dyn PluginParameters> {
        log::info!("called get_parameter_object");
        Arc::clone(&self.params) as Arc<dyn PluginParameters>
    }

    fn can_do(&self, can_do: CanDo) -> vst::api::Supported {
        log::info!("called can_do");
        use vst::api::Supported::*;
        use vst::plugin::CanDo::*;

        match can_do {
            SendEvents | SendMidiEvent | ReceiveEvents | ReceiveMidiEvent => Yes,
            _ => Maybe,
        }
    }
}

impl PluginParameters for GainEffectParameters {
    // the `get_parameter` function reads the value of a parameter.
    fn get_parameter(&self, index: i32) -> f32 {
        match index {
            0 => self.amplitude.get(),
            _ => 0.0,
        }
    }

    // the `set_parameter` function sets the value of a parameter.
    fn set_parameter(&self, index: i32, val: f32) {
        #[allow(clippy::single_match)]
        match index {
            0 => self.amplitude.set(val),
            _ => (),
        }
    }

    // This is what will display underneath our control.  We can
    // format it into a string that makes the most since.
    fn get_parameter_text(&self, index: i32) -> String {
        match index {
            0 => format!("{:.2}", (self.amplitude.get() - 0.5) * 2f32),
            _ => "".to_string(),
        }
    }

    // This shows the control's name.
    fn get_parameter_name(&self, index: i32) -> String {
        match index {
            0 => "Amplitude",
            _ => "",
        }
        .to_string()
    }
}

struct VstParent(*mut ::std::ffi::c_void);

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

plugin_main!(TestPlugin);
