use std::marker::PhantomData;

use crate::error::Error;
use crate::peer;

pub struct FrameBuffer {
    _marker: PhantomData<Vec<()>>,
}

impl FrameBuffer {
    /// Create a new instance of FrameBuffer.
    ///
    /// ```
    /// let fb = FrameBuffer::new()
    /// ```
    pub fn new() -> FrameBuffer {
        FrameBuffer {
            _marker: PhantomData {},
        }
    }

    pub fn set_title(&mut self, text: &[u8]) -> Result<(), Error> {
        let ok = unsafe { peer::set_title(text.as_ptr(), text.len()) };
        if ok { Ok(()) } else { Err(Error::GENERIC) }
    }

    pub fn resize(&mut self, width: u16, height: u16) -> Result<(), Error> {
        let ok = unsafe { peer::resize(width, height) };
        if ok { Ok(()) } else { Err(Error::GENERIC) }
    }

    pub fn fill_rect(
        &mut self,
        data: u32,
        x: u16,
        y: u16,
        width: u16,
        height: u16,
    ) -> Result<(), Error> {
        let ok = unsafe { peer::fill_rect(data, x, y, width, height) };
        if ok { Ok(()) } else { Err(Error::GENERIC) }
    }

    pub fn update_rect(
        &mut self,
        data: &[u8],
        x: u16,
        y: u16,
        width: u16,
        height: u16,
    ) -> Result<(), Error> {
        let ptr = data.as_ptr();
        let ok = unsafe { peer::update_rect(ptr, x, y, width, height) };
        if ok { Ok(()) } else { Err(Error::GENERIC) }
    }

    pub fn copy_rect(
        &mut self,
        sx: u16,
        sy: u16,
        dx: u16,
        dy: u16,
        width: u16,
        height: u16,
    ) -> Result<(), Error> {
        let ok = unsafe { peer::copy_rect(sx, sy, dx, dy, width, height) };
        if ok { Ok(()) } else { Err(Error::GENERIC) }
    }

    pub fn bell(&mut self) -> Result<(), Error> {
        let ok = unsafe { peer::bell() };
        if ok { Ok(()) } else { Err(Error::GENERIC) }
    }

    pub fn clipboard_text(&mut self, text: &[u8]) -> Result<(), Error> {
        let ptr = text.as_ptr();
        let len = text.len();
        let ok = unsafe { peer::clipboard_text(ptr, len) };
        if ok { Ok(()) } else { Err(Error::GENERIC) }
    }

    pub fn led(&mut self, state: u8) -> Result<(), Error> {
        let ok = unsafe { peer::led(state) };
        if ok { Ok(()) } else { Err(Error::GENERIC) }
    }

    pub fn pointer_mode(&mut self, mode: u8) -> Result<(), Error> {
        let ok = unsafe { peer::pointer_mode(mode) };
        if ok { Ok(()) } else { Err(Error::GENERIC) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_frame_buffer() {
        let fb = FrameBuffer::new();
    }
}
