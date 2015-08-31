use std::path::PathBuf;
use std::sync::{Mutex, Arc};
use std::collections::VecDeque;
use std::env::home_dir;
use std::fs::File;
use std::io::Read;

use rustc_serialize::json::{Object, Json};

use input::Input;
use keyboard::Key;
use view::View;
use frontends::{Frontend, EditorEvent};
use modes::{Mode, ModeType, InsertMode, NormalMode};
use overlay::{Overlay, OverlayEvent};
use buffer::Buffer;
use command::Command;
use command::{Action, BuilderEvent, Operation, Instruction};
use keymap::KeyMap;
use keymap::KeyMapState;

use textobject::TextObject;
use buffer::Mark;
use textobject::Kind;
use textobject::Offset;


#[derive(Copy, Clone, Debug)]
struct Event {
    name: &'static str,
}

impl Event {
    pub fn new(name: &'static str) -> Event {
        Event {
            name: name,
        }
    }

    pub fn get_name(&self) -> &'static str {
        self.name
    }
}


/// The main Editor structure
///
/// This is the top-most structure in Iota.
pub struct Editor<'e, T: Frontend> {
    buffers: Vec<Arc<Mutex<Buffer>>>,
    view: View,
    running: bool,
    frontend: T,
    mode: Box<Mode + 'e>,
    events_queue: VecDeque<Event>,
    keymap: KeyMap<Event>,
}

impl<'e, T: Frontend> Editor<'e, T> {
    /// Create a new Editor instance from the given source
    pub fn new(source: Input, mode: Box<Mode + 'e>, frontend: T) -> Editor<'e, T> {
        let height = frontend.get_window_height();
        let width = frontend.get_window_width();

        let mut buffers = Vec::new();
        let buffer = Buffer::from(source);

        buffers.push(Arc::new(Mutex::new(buffer)));

        let view = View::new(buffers[0].clone(), width, height);
        Editor {
            buffers: buffers,
            view: view,
            running: true,
            frontend: frontend,
            mode: mode,
            events_queue: VecDeque::new(),
            keymap: KeyMap::new(),
        }
    }

    /// Handle key events
    ///
    /// Key events can be handled in an Overlay, OR in the current Mode.
    ///
    /// If there is an active Overlay, the key event is sent there, which gives
    /// back an OverlayEvent. We then parse this OverlayEvent and determine if
    /// the Overlay is finished and can be cleared. The response from the
    /// Overlay is then converted to a Command and sent off to be handled.
    ///
    /// If there is no active Overlay, the key event is sent to the current
    /// Mode, which returns a Command which we dispatch to handle_command.
    fn handle_key_event(&mut self, key: Option<Key>) {
        let key = match key {
            Some(k) => k,
            None => return
        };

        // look up KeyMap
        match self.keymap.check_key(key) {
            KeyMapState::Match(c) => {
                // found a match!
                self.fire_event(c);
            },
            KeyMapState::Continue => {
                // possibly the start of a match...
                // not sure what to do here...
            }
            KeyMapState::None => {
                // no match at all :(
                //
                // lets try insert it into the buffer
                // TODO: use an event for this instead
                if let Key::Char(ch) = key {
                    self.view.insert_char(ch);
                }
            }
        }
    }

    /// Translate the response from an Overlay to a Command wrapped in a BuilderEvent
    ///
    /// In most cases, we will just want to convert the response directly to
    /// a Command, however in some cases we will want to perform other actions
    /// first, such as in the case of Overlay::SavePrompt.
    // fn handle_overlay_response(&mut self, response: Option<String>) -> BuilderEvent {
    //     // FIXME: This entire method neext to be updated
    //     match response {
    //         Some(data) => {
    //             match self.view.overlay {
    //
    //                 // FIXME: this is just a temporary fix
    //                 Overlay::Prompt { ref data, .. } => {
    //                     match &**data {
    //                         // FIXME: need to find a better system for these commands
    //                         //        They should be chainable
    //                         //          ie: wq - save & quit
    //                         //        They should also take arguments
    //                         //          ie w file.txt - write buffer to file.txt
    //                         "q" | "quit" => BuilderEvent::Complete(Command::exit_editor()),
    //                         "w" | "write" => BuilderEvent::Complete(Command::save_buffer()),
    //
    //                         _ => BuilderEvent::Incomplete
    //                     }
    //                 }
    //
    //                 Overlay::SavePrompt { .. } => {
    //                     let path = PathBuf::from(&*data);
    //                     self.view.buffer.lock().unwrap().file_path = Some(path);
    //                     BuilderEvent::Complete(Command::save_buffer())
    //                 }
    //
    //                 Overlay::SelectFile { .. } => {
    //                     let path = PathBuf::from(data);
    //                     let buffer = Arc::new(Mutex::new(Buffer::from(path)));
    //                     self.buffers.push(buffer.clone());
    //                     self.view.set_buffer(buffer.clone());
    //                     self.view.clear(&mut self.frontend);
    //                     BuilderEvent::Complete(Command::noop())
    //                 }
    //
    //                 _ => BuilderEvent::Incomplete,
    //             }
    //         }
    //         None => BuilderEvent::Incomplete
    //     }
    // }

    /// Handle resize events
    ///
    /// width and height represent the new height of the window.
    fn handle_resize_event(&mut self, width: usize, height: usize) {
        self.view.resize(width, height);
    }

    /// Draw the current view to the frontend
    fn draw(&mut self) {
        self.view.draw(&mut self.frontend);
    }

    /// Handle the given command, performing the associated action
    // fn handle_command(&mut self, command: Command) {
    //     let repeat = if command.number > 0 {
    //         command.number
    //     } else { 1 };
    //     for _ in 0..repeat {
    //         match command.action {
    //             Action::Instruction(i) => self.handle_instruction(i, command),
    //             Action::Operation(o) => self.handle_operation(o, command),
    //         }
    //     }
    // }


    // fn handle_instruction(&mut self, instruction: Instruction, command: Command) {
    //     match instruction {
    //         Instruction::SaveBuffer => { self.view.try_save_buffer() }
    //         Instruction::ExitEditor => { self.running = false; }
    //         Instruction::SetMark(mark) => {
    //             if let Some(object) = command.object {
    //                 self.view.move_mark(mark, object)
    //             }
    //         }
    //         Instruction::SetOverlay(overlay_type) => {
    //             self.view.set_overlay(overlay_type)
    //         }
    //         Instruction::SetMode(mode) => {
    //             match mode {
    //                 ModeType::Insert => { self.mode = Box::new(InsertMode::new()) }
    //                 ModeType::Normal => { self.mode = Box::new(NormalMode::new()) }
    //             }
    //         }
    //         Instruction::SwitchToLastBuffer => {
    //             self.view.switch_last_buffer();
    //             self.view.clear(&mut self.frontend);
    //         }
    //
    //         _ => {}
    //     }
    // }

    // fn handle_operation(&mut self, operation: Operation, command: Command) {
    //     match operation {
    //         Operation::Insert(c) => {
    //             for _ in 0..command.number {
    //                 self.view.insert_char(c)
    //             }
    //         }
    //         Operation::DeleteObject => {
    //             if let Some(obj) = command.object {
    //                 self.view.delete_object(obj);
    //             }
    //         }
    //         Operation::DeleteFromMark(m) => {
    //             if command.object.is_some() {
    //                 self.view.delete_from_mark_to_object(m, command.object.unwrap())
    //             }
    //         }
    //         Operation::Undo => { self.view.undo() }
    //         Operation::Redo => { self.view.redo() }
    //     }
    // }

    fn register_key_bindings(&mut self) {
        // TODO:
        //   Load these default from a JSON file of some sort
        self.bind_keys("up", "iota.move_up");
        self.bind_keys("down", "iota.move_down");
        self.bind_keys("left", "iota.move_left");
        self.bind_keys("right", "iota.move_right");
        self.bind_keys("ctrl-q", "iota.quit");

        self.bind_keys("backspace", "iota.delete_backwards");
        self.bind_keys("delete", "iota.delete_forwards");
        self.bind_keys("enter", "iota.newline");

        self.bind_keys("ctrl-z", "iota.undo");
        self.bind_keys("ctrl-r", "iota.redo");
        self.bind_keys("ctrl-s", "iota.save");
    }

    pub fn bind_keys(&mut self, key_str: &'static str, event: &'static str) {
        // TODO:
        //   it would be nice in the future to be able to store multiple events
        //   for each key. So for instance if an extension was to override a core
        //   keybinding, it would still store the core binding, but mark it as "inactive"
        //   This would allow us to visualize what order event/key bindings are being
        //   stored internally, and potentially disable/enable bindings at will.
        //   As it is now, binding an event to an already bound key will just override
        //   the binding.

        let bits: Vec<&str> = key_str.split(' ').collect();
        let mut keys: Vec<Key> = Vec::new();
        for part in bits {
            keys.push(Key::from(part));
        }
        self.keymap.bind_keys(&*keys, Event::new(event));
    }

    fn fire_event(&mut self, event: Event) {
        self.events_queue.push_back(event);
    }

    fn process_event(&mut self, event: Event) {
        // TODO:
        //   try process event in extensions first
        //   fall back here as a default
        //
        // NOTE:
        //   Extensions should be able to specify in their return
        //   type whether we should also perform the default Action
        //   for an event. For example, if the extension handles the "iota.save"
        //   event, they should be able to tell iota to perform the save,
        //   after whatever custom work they have done. This could be
        //   linting the file, for example.

        match event.get_name() {
            "iota.quit" => { self.running = false; }

            "iota.undo" => { self.view.undo(); }
            "iota.redo" => { self.view.redo(); }
            "iota.save" => { self.view.try_save_buffer(); }

            "iota.newline" => { self.view.insert_char('\n'); }

            "iota.delete_backwards" => {
                self.view.delete_from_mark_to_object(Mark::Cursor(0), TextObject{
                    kind: Kind::Char,
                    offset: Offset::Backward(1, Mark::Cursor(0))
                })
            }
            "iota.delete_forwards" => {
                self.view.delete_from_mark_to_object(Mark::Cursor(0), TextObject{
                    kind: Kind::Char,
                    offset: Offset::Forward(1, Mark::Cursor(0))
                })
            }

            "iota.move_up" => { self.view.move_up() }
            "iota.move_down" => { self.view.move_down() }
            "iota.move_left" => { self.view.move_left() }
            "iota.move_right" => { self.view.move_right() }

            _ => {}
        }
    }

    /// Start Iota!
    pub fn start(&mut self) {
        // self.register_extensions();
        self.register_key_bindings();

        while self.running {
            self.draw();
            self.frontend.present();
            let event = self.frontend.poll_event();

            match event {
                EditorEvent::KeyEvent(key)         => self.handle_key_event(key),
                EditorEvent::Resize(width, height) => self.handle_resize_event(width, height),

                _ => {}
            }

            while let Some(event) = self.events_queue.pop_front() {
                self.process_event(event);
            }
        }
    }
}
