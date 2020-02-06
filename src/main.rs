extern crate bitstream_io;
extern crate byteorder;

use byteorder::ReadBytesExt;
use std::net::TcpStream;
use std::io::{Read, Cursor};
use crate::can::{CanPacket, CanAddr};
use crate::lap::LapPacket;

mod can;
mod lap;

fn main() {
    let mut s = TcpStream::connect("weblab:2342").unwrap();

    /*
    let p = lap::SetLampPacket {
        mode: 1,
        lamp_id: 8,
        value: 0xaa
    };
    let pac = p.to_can(CanAddr(0, 2), CanAddr(96, 2));
    */
    
    let pac = CanPacket {
        src_addr: 0,
        dest_addr: 0x24,
        src_port: 0x23,
        dest_port: 0x23,
        payload: vec![1, 4]
    };

    can::write_packet_to_cand(&mut s, &pac).unwrap();

    loop {
        let mut buf = Vec::new();
        let size = s.read_u8().unwrap();
        let kind = s.read_u8().unwrap();
        println!("Foo: {}", kind);

        buf.resize(size as usize, 0);
        s.read_exact(&mut buf).unwrap();
        let mut c = Cursor::new(buf);
        let can_packet = can::CanPacket::read(&mut c).unwrap();
        println!("{:x?}", can_packet);
    }
}
