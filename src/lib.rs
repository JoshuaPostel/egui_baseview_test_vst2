//! Barebones baseview egui plugin

#[macro_use]
extern crate vst;

pub mod state;
pub mod ui;
use state::{DawParameters, EditorState};
use ui::TestPluginEditor;

use std::sync::Arc;
use std::sync::Mutex;

use log;
use ringbuf::{Producer, RingBuffer};
use simplelog;
use vst::buffer::{AudioBuffer, SendEventBuffer};
use vst::editor::Editor;
use vst::event::Event;
use vst::plugin::{CanDo, Category, HostCallback, Info, Plugin, PluginParameters};

struct TestPlugin {
    params: Arc<DawParameters>,
    editor: Option<TestPluginEditor>,
    midi_producer: Producer<[u8; 3]>,
    host: HostCallback,
    send_buffer: SendEventBuffer,
}

impl TestPlugin {
    fn new(host: HostCallback) -> Self {
        let midi_ring = RingBuffer::<[u8; 3]>::new(1_000);
        let (midi_producer, midi_consumer) = midi_ring.split();
        let params = Arc::new(DawParameters::default());
        let state = EditorState::new(&params, midi_consumer);
        //        let state = EditorState {
        //            params: params.clone(),
        //            midi_consumer: Arc::new(Mutex::new(midi_consumer)),
        //            last_note: Arc::new(Mutex::new([0, 0, 0])),
        //        };
        Self {
            params: params.clone(),
            editor: Some(TestPluginEditor {
                state: Arc::new(state),
                window_handle: None,
                is_open: false,
            }),
            midi_producer,
            host,
            send_buffer: SendEventBuffer::default(),
        }
    }
}

impl Default for TestPlugin {
    fn default() -> Self {
        let midi_ring = RingBuffer::<[u8; 3]>::new(1_000);
        let (midi_producer, midi_consumer) = midi_ring.split();
        let params = Arc::new(DawParameters::default());
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
            send_buffer: SendEventBuffer::default(),
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
            midi_outputs: 1,
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

    fn process_events(&mut self, events: &vst::api::Events) {
        let mut output_midi_events: Vec<vst::event::MidiEvent> = vec![];
        for e in events.events() {
            match e {
                Event::Midi(mut midi_event) => {
                    log::info!("got midi event: {:?}", midi_event.data);
                    self.midi_producer.push(midi_event.data).unwrap_or(());
                    midi_event.data[1] += 12; // pictch notes up an octive
                    output_midi_events.push(midi_event);
                }
                _ => (),
            }
        }
        self.send_buffer
            .send_events(&output_midi_events, &mut self.host);
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

plugin_main!(TestPlugin);
