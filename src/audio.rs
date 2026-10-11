use crate::error::Error;
use crate::peer;

pub struct AudioSink {}

impl AudioSink {
    pub fn new() -> Self {
        Self {}
    }

    pub fn start(&mut self) -> Result<(), Error> {
        let ok = unsafe { peer::audio_start() };
        if ok { Ok(()) } else { Err(Error::GENERIC) }
    }

    pub fn stop(&mut self) -> Result<(), Error> {
        let ok = unsafe { peer::audio_stop() };
        if ok { Ok(()) } else { Err(Error::GENERIC) }
    }

    pub fn data(&mut self, data: &[u8]) -> Result<(), Error> {
        let ok = unsafe { peer::audio_data(data.as_ptr(), data.len()) };
        if ok { Ok(()) } else { Err(Error::GENERIC) }
    }
}
