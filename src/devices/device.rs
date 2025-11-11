use crate::talker::Talker;

/// This flag shows if a Service Request is needed after the byte is processed.
///
/// If [`ServiceRequest::Required`] is used, it will turn on Serial Poll mode
/// on the GRiD Compass. This is needed for very long requests like reading and writing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ServiceRequest {
    NotRequired,
    Required,
}

pub trait Device {
    /// Resets the device to default state.
    fn reset(&mut self);

    /// Processes a byte from the bus.
    ///
    /// Returns a flag indicating if a service request is needed.
    fn process_byte(&mut self, byte: u8, eoi: bool) -> ServiceRequest;

    /// Data transfer completed and the Unlisten command was received.
    fn process_complete(&mut self) {
        // Do nothing
    }

    /// Someone on the bus addressed you and told you "talk".
    fn talk(&mut self, talker: Talker);
}
