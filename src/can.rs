use std::result::Result as StdResult;
use std::{io, fmt};
use byteorder::{ReadBytesExt, LittleEndian, WriteBytesExt};
use std::str::FromStr;
use crate::error::{self, Error, Result};

#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub struct CanAddr(u8, u8);

impl CanAddr {
    pub fn new(addr: u8, port: u8) -> StdResult<CanAddr, error::InvalidCanPort> {
        if port > 0x3f {
            return Err(error::InvalidCanPort);
        }

        Ok(CanAddr(addr, port))
    }

    pub fn addr(&self) -> u8 {
        self.0
    }

    pub fn port(&self) -> u8 {
        self.1
    }
}

impl fmt::Display for CanAddr {
    fn fmt(&self, f: &mut fmt::Formatter) -> StdResult<(), fmt::Error> {
        write!(f, "{:02x}:{:02x}", self.0, self.1)
    }
}

impl FromStr for CanAddr {
    type Err = error::CanAddrParseError;

    fn from_str(s: &str) -> StdResult<Self, Self::Err> {
        let mut split = s.split(':');
        if let (Some(addr), Some(port), None) = (split.next(), split.next(), split.next()) {
            let num_addr = u8::from_str_radix(addr, 16).
                map_err(|_| error::CanAddrParseError)?;
            let num_port = u8::from_str_radix(port, 16).
                map_err(|_| error::CanAddrParseError)?;
            Ok(CanAddr(num_addr, num_port))
        } else {
            Err(error::CanAddrParseError)
        }
    }
}

#[derive(Debug, Eq, PartialEq, Clone)]
pub struct CanPacket {
    pub src: CanAddr,
    pub dest: CanAddr,
    pub payload: Vec<u8>
}

impl CanPacket {

    pub fn new(src: CanAddr, dest: CanAddr, payload: Vec<u8>) -> CanPacket {
        CanPacket {
            src,
            dest,
            payload
        }
    }

    pub fn write<W>(&self, write: &mut W) -> Result<()> where W: io::Write {
        let can_id = can_id_from_tuple(self.src, self.dest);

        write.write_u32::<LittleEndian>(can_id)?;
        write.write_u8(self.payload.len() as u8)?;
        write.write_all(&self.payload)?;

        Ok(())
    }

    pub fn read<R>(read: &mut R) -> Result<CanPacket> where R: io::Read {

        let can_id = read.read_u32::<LittleEndian>()?;

        let (src, dest) = can_id_to_tuple(can_id)?;

        let mut payload = Vec::new();

        let dlc = read.read_u8()?;
        read.read_to_end(&mut payload)?;

        if dlc as usize != payload.len() {
            return Err(Error::WrongLength);
        }


        Ok(CanPacket {
            src,
            dest,
            payload
        })
    }
}

pub fn can_id_from_tuple(src: CanAddr, dest: CanAddr) -> u32 {
    (((src.port() & 0x3f) as u32) << 23) |
        (((dest.port() & 0x30) as u32) << 17) |
        (((dest.port() & 0x0f) as u32) << 16) |
        ((src.addr() as u32) << 8) |
        (dest.addr() as u32)
}

/// # Errors
/// [[CanAddrParseError]] if the result id is not a valid 29 bit ID
pub fn can_id_to_tuple(id: u32) -> StdResult<(CanAddr, CanAddr), error::InvalidCanId> {
    if id > 0x1fffffff {
        return Err(error::InvalidCanId);
    }

    let src_addr = (id >> 8) as u8;
    let dest_addr = id as u8;
    let src_port = ((id >> 23) & 0x3f) as u8;
    let dest_port = (((id >> 16) & 0x0f) | ((id >> 17) & 0x30)) as u8;

    Ok((CanAddr(src_addr, src_port), CanAddr(dest_addr, dest_port)))
}

#[cfg(test)]
mod test {
    use std::io::{Cursor, Seek, SeekFrom};
    use crate::can::{CanAddr, CanPacket};

    #[test]
    fn test_recode() {
        let input = CanPacket {
            src: CanAddr::new(0x42, 0xaa).unwrap(),
            dest: CanAddr::new(0b101010, 0b110011).unwrap(),
            payload: vec![0x13, 0x37]
        };

        let mut cursor = Cursor::new(Vec::new());
        input.write(&mut cursor).unwrap();

        cursor.seek(SeekFrom::Start(0)).unwrap();

        let output = CanPacket::read(&mut cursor).unwrap();

        assert_eq!(input, output);
    }

    #[test]
    fn test_can_addr_display() {
        let addr = CanAddr(0x42, 0x3f);

        assert_eq!(format!("{}", addr), "42:3f");
    }
}