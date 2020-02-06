pub trait LapPacket {
    fn to_can(&self, src: CanAddr, dst: CanAddr) -> CanPacket;
}

use crate::can::{CanPacket, CanAddr};

//#[derive(FromPrimitive)]
#[derive(Clone, Copy)]
pub enum LampMode {
    Toggle = 0,
    Dim = 1
}

#[derive(Clone, Copy)]
pub enum BorgMessage {
    Info = 0,
    Mode = 1,
    ScrollReset = 2,
    ScrollAppend = 3
}

pub struct SetLampPacket {
    pub mode: LampMode,
    pub lamp_id: u8,
    pub value: u8
}

impl LapPacket for SetLampPacket {
    fn to_can(&self, src: CanAddr, dst: CanAddr) -> CanPacket {
        CanPacket {
            src_addr: src.0,
            dest_addr: dst.0,
            src_port: src.1,
            dest_port: dst.1,
            payload: vec![self.mode as u8, self.lamp_id, self.value]
        }
    }
}