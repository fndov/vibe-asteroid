use std::collections::HashMap;
use std::io;
use crossterm::event::{Event, KeyCode};

// --- SimulatedInput for debugging ---
pub struct SimulatedInput {
    events: HashMap<u64, Event>,
    current_frame: u64,
}

impl SimulatedInput {
    pub fn new(events: HashMap<u64, Event>) -> Self {
        SimulatedInput { events, current_frame: 0 }
    }

    pub fn poll(&mut self, frame_count: u64) -> io::Result<bool> {
        self.current_frame = frame_count;
        Ok(self.events.contains_key(&frame_count))
    }

    pub fn read(&mut self) -> io::Result<Event> {
        if let Some(event) = self.events.remove(&self.current_frame) {
            Ok(event)
        } else {
            Ok(Event::Key(KeyCode::Null.into()))
        }
    }
}