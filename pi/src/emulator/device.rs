use crate::gpib;

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
    fn reset(&mut self);

    fn process_bytes(&mut self, buffer: &[u8]) -> ServiceRequest;

    fn talk(&mut self, talker: gpib::Talker);
}
