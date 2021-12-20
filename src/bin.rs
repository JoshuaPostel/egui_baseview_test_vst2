mod state;
mod ui;
use state::{DawParameters, EditorState};

use std::sync::Arc;

use egui::CtxRef;
use egui_baseview::{EguiWindow, Queue};
use ringbuf::RingBuffer;

fn main() {
    let params = Arc::new(DawParameters::default());
    let midi_ring = RingBuffer::<[u8; 3]>::new(10);
    let (_, midi_consumer) = midi_ring.split();
    let state = Arc::new(EditorState::new(&params, midi_consumer));

    let _window_handle = EguiWindow::open_blocking(
        ui::settings(),
        state.clone(),
        |_egui_ctx: &CtxRef, _queue: &mut Queue, _state: &mut Arc<EditorState>| {},
        ui::update(),
    );
}
