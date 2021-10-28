use std::io;
use failure::Fail;
use std::result::Result as StdResult;

pub type Result<T> = StdResult<T, Error>;

#[derive(Fail, Debug)]
#[fail(display = "Could not parse CAN address")]
pub struct CanAddrParseError;

#[derive(Fail, Debug)]
#[fail(display = "CAN identifier malformed")]
pub struct InvalidCanId;

#[derive(Fail, Debug)]
#[fail(display = "CAN Port out of range")]
pub struct InvalidCanPort;

#[derive(Fail, Debug)]
pub enum Error {
    #[fail(display = "CAN Port out of range")]
    InvalidCanPort,

    #[fail(display = "CAN identifier malformed")]
    InvalidCanId,

    #[fail(display = "Inner and outer length mismatch")]
    WrongLength,

    #[fail(display = "IO Error: {}", _0)]
    IOError(#[cause] io::Error)
}

impl From<InvalidCanPort> for Error {
    fn from(_: InvalidCanPort) -> Self {
        Error::InvalidCanPort
    }
}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self {
        Error::IOError(e)
    }
}

impl From<InvalidCanId> for Error {
    fn from(_: InvalidCanId) -> Self {
        Error::InvalidCanId
    }
}