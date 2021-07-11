use super::commands::Command;
use super::elements::*;
use super::window::*;
use super::platform;
use std::process;

const MAX_OS_WINDOWS: usize = 16;

/// Stores state global to all GUI windows.
pub struct Context {
    frame_counter: usize,
    platform_context: platform::Context,
    windows: [Option<Window>; MAX_OS_WINDOWS],
}

impl Context {
    pub fn new() -> Self {
        Context {
            frame_counter: 0,
            platform_context: platform::Context::new(),
            windows: [Default::default(); MAX_OS_WINDOWS],
        }
    }

    pub fn run<F>(&mut self, event_handler: F) -> !
    where
        F: 'static + FnMut(&mut Control),
    {
        let mut runner = Runner {
            context: self,
            event_handler,
        };
        runner.run();

        process::exit(0);
    }

    fn begin_frame(&mut self) {
        self.frame_counter += 1;
    }

    /// Performs cleanup of unused resources from previous frames.
    ///
    /// OS windows that were not touched this frame will be destroyed.
    fn end_frame(&mut self) {
        println!("end_frame");
        for slot in self.windows.iter_mut().filter(|x| x.is_some()) {
            let window = slot.as_ref().unwrap();
            if window.frame_last_touched < self.frame_counter {
                // destroy os window here!
                println!("Destroying windows: {}", window.get_title());
                self.platform_context.destroy_window(window.os_window);
                *slot = None;
            }
        }
    }

    /// Touches a window to keep it alive for the current frame. If the window
    /// name did not exist in the previous frame, a new window will be created
    /// with default parameters.
    ///
    /// Windows that are not touched in the current frame will be destroyed in
    /// the next frame (the next time `end_frame()` is called).
    fn touch_window(&mut self, name: &str) -> Option<usize> {
        let name_hash = Window::hash_title(name);

        let mut free_index = None;
        for (i, window) in self.windows.iter_mut().enumerate() {
            if let Some(h) = window {
                if h.name_hash == name_hash {
                    h.frame_last_touched += 1;
                    return Some(i);
                }
            } else {
                free_index = Some(i);
            }
        }

        if let Some(i) = free_index {
            let os_window = self.platform_context.create_window(name);
            self.windows[i] = Some(Window::new(name, name_hash, self.frame_counter, os_window));
        }
        free_index
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

    pub fn begin(&mut self, name: &str) -> ElementId {
        let window_index = self.context.touch_window(name);
        self.commands.push(Command::BeginWindow(0));
        ElementId(window_index.unwrap() as u64)
    }

    pub fn end(&mut self) {
        self.commands.push(Command::EndWindow);
    }
}

/// Takes the commands created in the event loop and turns them into visible
/// artifacts.
struct Runner<'a, F>
where
    F: 'static + FnMut(&mut Control),
{
    context: &'a mut Context,
    event_handler: F,
}

impl<'a, F> Runner<'a, F>
where
    F: 'static + FnMut(&mut Control),
{
    pub fn run(&mut self) {
        self.context.begin_frame();

        let mut control = Control::new(self.context);
        (self.event_handler)(&mut control);

        for command in control.commands {
            match command {
                Command::BeginWindow(_) => {}
                Command::EndWindow => {}
            }
        }

        self.context.end_frame();

        // call event_handler
        // every time a BeginWindow command is encountered
        // if an ElementId was provided,
        // fetch it from the window-map for the previous frame
        // move it into the current frame
        // if no ElementId was provided
        // create a new window
        // move it into the current frame
        // do nothing when an EndWindow command is encountered
    }
}
