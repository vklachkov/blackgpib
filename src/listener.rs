use crate::messages;

/// Listener state machine, implements
/// 2.6.2 L Function State Diagram.
pub struct ListenerStateMachine {
    dev_address: u8,
    current_state: ListenerState,
    buffer: Vec<u8>,
}

#[derive(Clone, Copy, Debug)]
enum ListenerState {
    /// Wait for MLA.
    ///
    /// `LIDS` status in terms of the standard.
    Idle,

    /// Read bytes into the internal buffer until an UNL or MTA
    /// command is received.
    ///
    /// `LACS` status in terms of the standard.
    Active,
}

impl ListenerStateMachine {
    pub fn new(dev_address: u8) -> Self {
        Self {
            dev_address,
            current_state: ListenerState::Idle,
            buffer: Vec::with_capacity(16),
        }
    }

    pub fn process(&mut self, byte: u8, is_command: bool) -> Option<Vec<u8>> {
        match self.current_state {
            ListenerState::Idle => {
                if is_command && messages::is_mla(byte, self.dev_address) {
                    self.switch_to(ListenerState::Active);
                }
            }
            ListenerState::Active => {
                if is_command {
                    if messages::is_unl(byte) {
                        self.switch_to(ListenerState::Idle);

                        let read = self.buffer.clone();
                        self.buffer.clear();

                        return Some(read);
                    } else {
                        // Ignore other commands.
                    }
                } else {
                    println!("Add `{byte:#04x}` to buffer");
                    self.buffer.push(byte);
                }
            }
        };

        None
    }

    fn switch_to(&mut self, new_state: ListenerState) {
        println!("Switch state from `{:?}` to `{:?}`", self.current_state, new_state);
        self.current_state = new_state;
    }
}
