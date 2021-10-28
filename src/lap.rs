pub trait LapPacket {
    fn to_can(&self, src: CanAddr, dst: CanAddr) -> CanPacket;
}

use crate::can::{CanPacket, CanAddr};
use std::cmp;

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
    fn to_can(&self, src: CanAddr, dest: CanAddr) -> CanPacket {
        CanPacket {
            src,
            dest,
            payload: vec![self.mode as u8, self.lamp_id, self.value]
        }
    }
}

pub struct ClearBorgText;

impl LapPacket for ClearBorgText {
    fn to_can(&self, src: CanAddr, dest: CanAddr) -> CanPacket {
        CanPacket {
            src,
            dest,
            payload: vec![0x02]
        }
    }
}

pub struct BorgMode(pub u8);

impl LapPacket for BorgMode {
    fn to_can(&self, src: CanAddr, dest: CanAddr) -> CanPacket {
        CanPacket {
            src,
            dest,
            payload: vec![0x01, self.0]
        }
    }
}

pub struct AppendBorgText {
    text: [u8; 7]
}

impl LapPacket for AppendBorgText {
    fn to_can(&self, src: CanAddr, dest: CanAddr) -> CanPacket {
        let payload = [0x03].iter()
            .chain(self.text.iter())
            .map(|i| *i)
            .collect();
        CanPacket {
            src,
            dest,
            payload
        }
    }
}

pub fn set_scroll_text(input: &str, src: CanAddr, dst: CanAddr) -> Vec<CanPacket> {
    let input_data = input.as_bytes();
    let mut buf = Vec::with_capacity(2 + input.len() / 7);
    buf.push(ClearBorgText.to_can(src, dst));
    let mut idx = 0;
    while idx < input.len() {
        let substr = &input_data[idx..];
        let sub = &substr[..cmp::min(substr.len(), 7)];
        let mut text = [0; 7];
        copy_data(&sub, &mut text);
        buf.push(AppendBorgText { text }.to_can(src, dst));
        idx += 7;
    }
    buf
}

fn copy_data(src: &[u8], dest: &mut [u8]) {
    for (idx, el) in src.iter().enumerate() {
        dest[idx] = *el;
    }
}