use std::sync::Arc;
use std::sync::Mutex;

use ringbuf::Consumer;
use vst::plugin::PluginParameters;
use vst::util::AtomicFloat;

pub struct EditorState {
    pub params: Arc<DawParameters>,
    pub midi_consumer: Arc<Mutex<Consumer<[u8; 3]>>>,
    pub last_note: Arc<Mutex<[u8; 3]>>,
}

impl EditorState {
    pub fn new(params: &Arc<DawParameters>, midi_consumer: Consumer<[u8; 3]>) -> Self {
        EditorState {
            params: params.clone(),
            midi_consumer: Arc::new(Mutex::new(midi_consumer)),
            last_note: Arc::new(Mutex::new([0, 0, 0])),
        }
    }
}

pub struct DawParameters {
    pub amplitude: AtomicFloat,
}

impl Default for DawParameters {
    fn default() -> DawParameters {
        DawParameters {
            amplitude: AtomicFloat::new(0.5),
        }
    }
}

impl PluginParameters for DawParameters {
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
