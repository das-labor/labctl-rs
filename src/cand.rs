use std::{fmt, io};
use std::fmt::Formatter;
use std::io::{Cursor, Read, Write};
use byteorder::{BigEndian, LittleEndian, ReadBytesExt, WriteBytesExt};
use crate::can::CanPacket;
use crate::error::Result;
#[cfg(feature = "async")]
use tokio::io::{AsyncRead, AsyncWrite, AsyncReadExt, AsyncWriteExt};

#[derive(Clone)]
pub enum Message {
    Frame(CanPacket),
    Reset { cause: u8 },
    Ping,
    Resync,
    VersionRequest,
    VersionReply { major: u8, minor: u8 },
    FirmwareIdRequest,
    FirmwareIdResponse(String),
    BusPowerRequest,
    BusPowerResponse { v: u16, i: u16, reference: u16, gnd: u16 },
    Unknown { kind: u8, payload:  Vec<u8> }
}

impl Message {
    pub fn read(kind: u8, buf: &[u8]) -> Result<Message> {
        let len = buf.len();
        let mut cur = Cursor::new(buf);
        match (kind, len) {
            (0x11, _) => Ok(Message::Frame(CanPacket::read(&mut cur)?)),
            (0x14, 1) => Ok(Message::Reset { cause: ReadBytesExt::read_u8(&mut cur)? }),
            (0x15, 0) => Ok(Message::Ping),
            (0x16, 0) => Ok(Message::Resync),
            (0x17, 0) => Ok(Message::VersionRequest),
            (0x17, 2) => Ok(Message::VersionReply {
                major: ReadBytesExt::read_u8(&mut cur)?,
                minor: ReadBytesExt::read_u8(&mut cur)?
            }),
            (0x18, 0) => Ok(Message::FirmwareIdRequest),
            (0x18, _) => Ok(Message::FirmwareIdResponse(String::from_utf8_lossy(buf).into_owned())),
            (0x1b, 0) => Ok(Message::BusPowerRequest),
            (0x1b, 8) => Ok(Message::BusPowerResponse {
                v: ReadBytesExt::read_u16::<LittleEndian>(&mut cur)?,
                i: ReadBytesExt::read_u16::<LittleEndian>(&mut cur)?,
                reference: ReadBytesExt::read_u16::<LittleEndian>(&mut cur)?,
                gnd: ReadBytesExt::read_u16::<LittleEndian>(&mut cur)?
            }),
            (_, _) => Ok(Message::Unknown {kind, payload: Vec::from(buf)})
        }
    }

    pub fn kind(&self) -> u8 {
        match self {
            Message::Frame(_) => 0x11,
            Message::Reset { .. } => 0x14,
            Message::Ping => 0x15,
            Message::Resync => 0x16,
            Message::VersionRequest => 0x17,
            Message::VersionReply { .. } => 0x17,
            Message::FirmwareIdRequest => 0x18,
            Message::FirmwareIdResponse(_) => 0x18,
            Message::Unknown { kind, .. } => *kind,
            Message::BusPowerRequest => 0x1b,
            Message::BusPowerResponse { .. } => 0x1b
        }
    }

    pub fn write<W: Write>(&self, write: &mut W) -> Result<()> {
        match self {
            Message::Frame(frame) => {
                frame.write(write)?;
            }
            Message::Reset { cause } => {
                write.write_u8(*cause)?;
            }
            Message::Ping => {} // Nothing to do
            Message::Resync => {} // Nothing to do
            Message::VersionRequest => {} // Nothing to do
            Message::VersionReply { major, minor } => {
                write.write_u8(*major)?;
                write.write_u8(*minor)?;
            }
            Message::FirmwareIdRequest => {} // Nothing to do
            Message::FirmwareIdResponse(id) => {
                write.write_all(id.as_bytes())?;
            }
            Message::Unknown { payload, .. } => {
                write.write_all(&payload)?;
            }
            Message::BusPowerRequest => {}
            Message::BusPowerResponse { v, i, reference, gnd } => {
                write.write_u16::<LittleEndian>(*v)?;
                write.write_u16::<LittleEndian>(*i)?;
                write.write_u16::<LittleEndian>(*reference)?;
                write.write_u16::<LittleEndian>(*gnd)?;
            }
        }
        Ok(())
    }
}

impl fmt::Debug for Message {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Message::Frame(frame) => {
                let mut tuple = f.debug_tuple("Message::Frame");
                tuple.field(frame);
                tuple.finish()
            }
            Message::Ping => {
                f.write_str("Message::Ping")
            }
            Message::Reset { cause } => {
                let mut tuple = f.debug_struct("Message::Reset");
                tuple.field("cause", cause);
                tuple.finish()
            }
            Message::Resync => {
                f.write_str("Message::Resync")
            }
            Message::VersionRequest => {
                f.write_str("Message::VersionRequest")
            }
            Message::VersionReply { major, minor } => {
                let mut tuple = f.debug_struct("Message::VersionReply");
                tuple.field("minor", minor);
                tuple.field("major", major);
                tuple.finish()
            }
            Message::FirmwareIdRequest => {
                f.write_str("Message::FirmwareIdRequest")
            }
            Message::FirmwareIdResponse(firmware_id) => {
                let mut tuple = f.debug_tuple("Message::FirmwareIdResponse");
                tuple.field(firmware_id);
                tuple.finish()
            }
            Message::Unknown { kind, payload } => {
                let mut s = f.debug_struct("Message::Unknown");
                s.field("kind", kind);
                s.field("payload", payload);
                s.finish()
            }
            Message::BusPowerRequest => {
                f.write_str("Message::BusPowerRequest")
            }
            Message::BusPowerResponse { v, i, reference, gnd } => {
                let mut tuple = f.debug_struct("Message::BusPowerResponse");
                tuple.field("v", v);
                tuple.field("i", i);
                tuple.field("ref", reference);
                tuple.field("gnd", gnd);
                tuple.finish()
            }
        }
    }
}

pub fn read_packet<R: Read>(read: &mut R) -> Result<Option<Message>> {
    let mut buf = Vec::new();
    let size = match read.read_u8() {
        Ok(size) => size,
        Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => {
            return Ok(None);
        },
        Err(e) => {
            return Err(e.into())
        }
    };
    let kind = read.read_u8()?;

    buf.resize(size as usize, 0);
    read.read_exact(&mut buf)?;

    Ok(Some(Message::read(kind, &buf)?))
}

#[cfg(feature = "async")]
pub async fn read_packet_async<R: AsyncRead + Unpin>(read: &mut R) -> Result<Option<Message>> {
    let mut buf = Vec::new();
    let size = match read.read_u8().await {
        Ok(size) => size,
        Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => {
            return Ok(None);
        },
        Err(e) => {
            return Err(e.into())
        }
    };
    let kind = read.read_u8().await?;

    buf.resize(size as usize, 0);
    read.read_exact(&mut buf).await?;

    Ok(Some(Message::read(kind, &buf)?))
}

pub fn write_packet_to_cand<W: io::Write>(w: &mut W, msg: &Message) -> Result<()> {
    let mut cur = Cursor::new(Vec::new());
    msg.write(&mut cur)?;
    let buf = cur.into_inner();

    w.write_u8(buf.len() as u8)?;
    w.write_u8(msg.kind())?;
    w.write_all(&buf)?;
    Ok(())
}

#[cfg(feature = "async")]
pub async fn write_packet_to_cand_async<W: AsyncWrite + Unpin>(w: &mut W, msg: &Message) -> Result<()> {
    let mut cur = Cursor::new(Vec::new());
    msg.write(&mut cur)?;
    let buf = cur.into_inner();

    w.write_u8(buf.len() as u8).await?;
    w.write_u8(msg.kind()).await?;
    w.write_all(&buf).await?;
    Ok(())
}