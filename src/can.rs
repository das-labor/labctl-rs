use std::{io, fmt};
use byteorder::{ReadBytesExt, LittleEndian, WriteBytesExt};
use std::io::{SeekFrom, Seek, Cursor, Read};
use std::str::FromStr;
use failure::Fail;

#[derive(Fail, Debug)]
#[fail(display = "Could not parse CAN address")]
pub struct CanAddrParseError;

#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub struct CanAddr(pub u8, pub u8);

impl fmt::Display for CanAddr {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "{:02x}:{:02x}", self.0, self.1)
    }
}

impl FromStr for CanAddr {
    type Err = CanAddrParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut split = s.split(':');
        if let (Some(addr), Some(port), None) = (split.next(), split.next(), split.next()) {
            let num_addr = u8::from_str_radix(addr, 16).
                map_err(|_| CanAddrParseError)?;
            let num_port = u8::from_str_radix(port, 16).
                map_err(|_| CanAddrParseError)?;
            Ok(CanAddr(num_addr, num_port))
        } else {
            Err(CanAddrParseError)
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct CanPacket {
    pub src_addr: u8,
    pub dest_addr: u8,
    pub src_port: u8,
    pub dest_port: u8,
    pub payload: Vec<u8>
}

impl CanPacket {
    pub fn write<W>(&self, write: &mut W) -> io::Result<()> where W: io::Write {
        self.sanity_check();

        let can_id =  (((self.src_port & 0x3f) as u32) << 23) |
            (((self.dest_port & 0x30) as u32) << 17) |
            (((self.dest_port & 0x0f) as u32) << 16) |
            ((self.src_addr as u32) << 8) |
            (self.dest_addr as u32);

        write.write_u32::<LittleEndian>(can_id)?;
        write.write_u8(self.payload.len() as u8)?;
        write.write_all(&self.payload)?;

        Ok(())
    }

    pub fn read<R>(read: &mut R) -> io::Result<CanPacket> where R: io::Read {

        let can_id = read.read_u32::<LittleEndian>()?;

        let src_addr = (can_id >> 8) as u8;
        let dest_addr = can_id as u8;
        let src_port = ((can_id >> 23) & 0x3f) as u8;
        let dest_port = (((can_id >> 16) & 0x0f) | ((can_id >> 17) & 0x30)) as u8;

        let mut payload = Vec::new();
        //let read = r.into_reader();
        let _dlc = read.read_u8()?;
        read.read_to_end(&mut payload)?;


        Ok(CanPacket {
            src_port,
            dest_port,
            src_addr,
            dest_addr,
            payload
        })
    }

    fn sanity_check(&self) {
        if self.src_port > 0x3f {
            panic!("Invalid Port - source port value cannot be larger than 0x3f (6 bit)");
        }
        if self.dest_port > 0x3f {
            panic!("Invalid Port - dest port value cannot be larger than 0x3f (6 bit)");
        }
    }
}

pub fn read_packet<R: Read>(read: &mut R) -> Result<CanPacket, io::Error> {
    let mut buf = Vec::new();
    let size = read.read_u8()?;
    let kind = read.read_u8()?;

    buf.resize(size as usize, 0);
    read.read_exact(&mut buf)?;
    let mut c = Cursor::new(buf);
    Ok(CanPacket::read(&mut c)?)
}

pub fn write_packet_to_cand<W: io::Write>(w: &mut W, p: &CanPacket) -> io::Result<()> {

    println!("Writing CAN packet: {:?}", p);

    let mut cur = Cursor::new(Vec::new());
    p.write(&mut cur)?;
    let buf = cur.into_inner();

    w.write_u8(buf.len() as u8)?;
    w.write_u8(0x11)?;
    w.write_all(&buf)?;
    Ok(())
}

#[test]
fn test_recode() {
    let input = CanPacket {
        src_addr: 0x42,
        dest_addr: 0xaa,
        src_port: 0b101010,
        dest_port: 0b110011,
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