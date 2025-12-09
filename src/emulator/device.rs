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
    fn process_byte(&mut self, byte: u8, eoi: bool);

    /// Data transfer completed and the UNL command was received.
    ///
    /// Returns a flag indicating if a service request is needed.
    fn unlisten(&mut self) -> ServiceRequest;

    /// Someone on the bus addressed you and told you "talk".
    fn talk(&mut self, talker: Talker);
}
