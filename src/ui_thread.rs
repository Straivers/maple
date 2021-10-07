use std::{
    sync::mpsc::{channel, Receiver, Sender},
    thread::JoinHandle,
};

use sys::{window::EventLoopControl, window_event::WindowEvent};

use crate::win32_ui_thread as platform;

pub struct WindowControl {
    pub(crate) control: platform::WindowControl,
}

impl WindowControl {}

pub struct WindowManager {
    num_threads: u32,
    receiver: Receiver<u32>,
    base_sender: Sender<u32>,
}

impl WindowManager {
    pub fn new() -> Self {
        let (base_sender, receiver) = channel();

        Self {
            num_threads: 0,
            receiver,
            base_sender,
        }
    }

    pub fn spawn_window<Callback>(&mut self, title: &str, callback: Callback) -> JoinHandle<()>
    where
        Callback: Send + Sync + 'static + FnMut(&WindowControl, WindowEvent) -> EventLoopControl,
    {
        self.num_threads += 1;
        platform::spawn_window(self.base_sender.clone(), title, callback)
    }

    pub fn wait_idle(&mut self) {
        while self.num_threads > 0 {
            let _ = self.receiver.recv();
            self.num_threads -= 1;
        }
    }
}
