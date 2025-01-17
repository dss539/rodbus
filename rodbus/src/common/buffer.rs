use crate::common::phys::PhysLayer;

#[cfg(feature = "no-panic")]
use no_panic::no_panic;

use crate::error::InternalError;

pub(crate) struct ReadBuffer {
    buffer: Vec<u8>,
    begin: usize,
    end: usize,
}

impl ReadBuffer {
    pub(crate) fn new(capacity: usize) -> Self {
        ReadBuffer {
            buffer: vec![0; capacity],
            begin: 0,
            end: 0,
        }
    }

    #[cfg_attr(feature = "no-panic", no_panic)]
    pub(crate) fn len(&self) -> usize {
        self.end - self.begin
    }

    #[cfg_attr(feature = "no-panic", no_panic)]
    pub(crate) fn is_empty(&self) -> bool {
        self.begin == self.end
    }

    #[cfg_attr(feature = "no-panic", no_panic)]
    pub(crate) fn read(&mut self, count: usize) -> Result<&[u8], InternalError> {
        if self.len() < count {
            return Err(InternalError::InsufficientBytesForRead(count, self.len()));
        }

        match self.buffer.get(self.begin..(self.begin + count)) {
            Some(ret) => {
                self.begin += count;
                Ok(ret)
            }
            None => Err(InternalError::InsufficientBytesForRead(count, self.len())),
        }
    }

    #[cfg_attr(feature = "no-panic", no_panic)]
    pub(crate) fn read_u8(&mut self) -> Result<u8, InternalError> {
        if self.is_empty() {
            return Err(InternalError::InsufficientBytesForRead(1, 0));
        }
        match self.buffer.get(self.begin) {
            Some(ret) => {
                self.begin += 1;
                Ok(*ret)
            }
            None => Err(InternalError::InsufficientBytesForRead(1, 0)),
        }
    }

    #[cfg_attr(feature = "no-panic", no_panic)]
    pub(crate) fn read_u16_be(&mut self) -> Result<u16, InternalError> {
        let b1 = self.read_u8()? as u16;
        let b2 = self.read_u8()? as u16;
        Ok((b1 << 8) | b2)
    }

    pub(crate) async fn read_some(&mut self, io: &mut PhysLayer) -> Result<usize, std::io::Error> {
        // before we read any data, check to see if the buffer is empty and adjust the indices
        // this allows use to make the biggest read possible, and avoids subsequent buffer shifting later
        if self.is_empty() {
            self.begin = 0;
            self.end = 0;
        }

        // if we've reached capacity, but still need more data we have to shift
        if self.end == self.buffer.capacity() {
            let length = self.len();
            self.buffer.copy_within(self.begin..self.end, 0);
            self.begin = 0;
            self.end = length;
        }

        let count = io.read(&mut self.buffer[self.end..]).await?;

        if count == 0 {
            return Err(std::io::Error::from(std::io::ErrorKind::UnexpectedEof));
        }
        self.end += count;
        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decode::PhysDecodeLevel;
    use crate::tokio::test::*;

    #[test]
    fn errors_when_reading_to_many_bytes() {
        let mut buffer = ReadBuffer::new(10);
        assert_eq!(
            buffer.read_u8(),
            Err(InternalError::InsufficientBytesForRead(1, 0))
        );
        assert_eq!(
            buffer.read(1),
            Err(InternalError::InsufficientBytesForRead(1, 0))
        );
    }

    #[test]
    fn shifts_contents_when_buffer_at_capacity() {
        let mut buffer = ReadBuffer::new(3);

        let (io, mut io_handle) = io::mock();
        let mut phys = PhysLayer::new_mock(io, PhysDecodeLevel::Nothing);

        {
            let buf_ref = &mut buffer;
            let mut task = spawn(async { buf_ref.read_some(&mut phys).await.unwrap() });
            assert_pending!(task.poll());
        }

        {
            let buf_ref = &mut buffer;
            let mut task = spawn(async { buf_ref.read_some(&mut phys).await.unwrap() });
            io_handle.read(&[0x01, 0x02, 0x03]);
            assert_ready_eq!(task.poll(), 3);
        }

        assert!(io_handle.all_done());
        assert_eq!(buffer.read(2).unwrap(), &[0x01, 0x02]);

        {
            let buf_ref = &mut buffer;
            let mut task = spawn(async { buf_ref.read_some(&mut phys).await.unwrap() });
            io_handle.read(&[0x04, 0x05]);
            assert_ready_eq!(task.poll(), 2);
        }

        assert_eq!(buffer.read(3).unwrap(), &[0x03, 0x04, 0x05]);
    }
}
