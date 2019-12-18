use crate::error::*;
use crate::service::traits::Serialize;
use crate::types::{AddressRange, CoilState, Indexed, RegisterValue};
use crate::util::cursor::WriteCursor;

impl Serialize for AddressRange {
    fn serialize(&self, cur: &mut WriteCursor) -> Result<(), Error> {
        cur.write_u16_be(self.start)?;
        cur.write_u16_be(self.count)?;
        Ok(())
    }
}

impl Serialize for details::ExceptionCode {
    fn serialize(&self, cursor: &mut WriteCursor) -> Result<(), Error> {
        cursor.write_u8(self.to_u8())?;
        Ok(())
    }
}

impl Serialize for Indexed<CoilState> {
    fn serialize(&self, cur: &mut WriteCursor) -> Result<(), Error> {
        cur.write_u16_be(self.index)?;
        cur.write_u16_be(self.value.to_u16())?;
        Ok(())
    }
}

impl Serialize for Indexed<RegisterValue> {
    fn serialize(&self, cursor: &mut WriteCursor) -> Result<(), Error> {
        cursor.write_u16_be(self.index)?;
        cursor.write_u16_be(self.value.value)?;
        Ok(())
    }
}

impl Serialize for &[bool] {
    fn serialize(&self, cursor: &mut WriteCursor) -> Result<(), Error> {

        // how many bytes should we have?
        let num_bytes : u8 = {
            let div_8 = self.len() / 8;

            if self.len() % 8 == 0 {
                div_8
            } else {
                div_8 + 1
            }
        } as u8; // TODO - validation!

        cursor.write_u8(num_bytes)?;

        for byte in self.chunks(8) {
            let mut acc : u8 = 0;
            let mut count : u8 = 0;
            for bit in byte {
                if *bit {
                    acc |= 1 << count;
                }
                count += 1;
            }
            cursor.write_u8(acc)?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serializes_address_range() {
        let range = AddressRange::new(3, 512);
        let mut buffer = [0u8; 4];
        let mut cursor = WriteCursor::new(&mut buffer);
        range.serialize(&mut cursor).unwrap();
        assert_eq!(buffer, [0x00, 0x03, 0x02, 0x00]);
    }
}