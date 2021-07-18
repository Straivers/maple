use super::commands::Command;
use super::platform;
use super::window::Window;
use std::marker::PhantomPinned;
use std::process;

const MAX_OS_WINDOWS: usize = 16;

/// Stores state global to all GUI windows.
pub struct Context {
    frame_counter: usize,
    windows: [Option<Window>; MAX_OS_WINDOWS],
    _pin: PhantomPinned,
}

impl Context {
    pub fn new() -> Self {
        Context {
            frame_counter: 1,
            windows: [Default::default(); MAX_OS_WINDOWS],
            _pin: PhantomPinned,
        }
    }

    pub fn run<F>(&mut self, event_handler: F) -> !
    where
        F: 'static + FnMut(&mut Control),
    {
        let mut callback = event_handler;
        platform::WindowManager::new().run(|w, event| {
            // we need to process commands here
            /*
            match event {
                WindowEvent { window, event } => {

                }
                InputEvent { window, event } => {

                }
                Draw => {
                    // start of frame setup

                    let mut control = Control::new(self, wm);
                    callback(&mut control);

                    for command in &control.commands {
                        ...
                    }

                    // end of frame cleanup
                    // reset input accumulators
                },
            }
            */
        });

        process::exit(0);
    }
}

/// A control acts as the interface between the context and the event loop;
/// recording UI changes, and storing responses to user responses.
pub struct Control<'a> {
    context: &'a mut Context,
    commands: Vec<Command>,
}

impl<'a> Control<'a> {
    fn new(context: &'a mut Context) -> Self {
        Control {
            context,
            commands: Vec::new(),
        }
    }

    pub fn reset(&mut self) {
        self.commands.clear();
    }

    pub fn begin(&mut self, name: &str) -> bool {
        // let window_index = self.context.touch_window(name).expect("Too many windows");
        // self.commands.push(Command::BeginWindow(0));
        // !self.context.windows[window_index]
        //     .unwrap()
        //     .os_window
        //     .user_requested_close
        todo!()
    }

    pub fn end(&mut self) {
        // self.commands.push(Command::EndWindow);
        todo!()
    }
}
