extern crate bitstream_io;
extern crate byteorder;
#[macro_use]
extern crate clap;

extern crate labctl;

use byteorder::ReadBytesExt;
use std::net::TcpStream;
use std::io::{Read, Cursor, Write};
use labctl::can::CanAddr;
use labctl::lap::{BorgMode, LapPacket};
use std::thread;
use std::time::Duration;
use labctl::cand::Message;

fn args<'a, 'b>() -> clap::App<'a, 'b> {
    clap_app!{labctl =>
        (version: "0.1")
        (author: "kilobyte22")
        (about: "Controls the Lab")
        (setting: clap::AppSettings::SubcommandRequiredElseHelp)
        (@arg host: -h +required +takes_value "The host to connect to")
        (@arg port: -p +takes_value "The port the cand listens on")
        (@subcommand monitor =>
            (@arg decode: -d)
        )
        (@subcommand borg =>
            (@subcommand text =>
                (@arg now: --now "Instantly show mode 1")
                (@arg DEST: +required "The destination address")
                (@arg TEXT: +required "The Text to display in Fucky Borg Script")
            )
            (@subcommand mode =>
                (@arg DEST: +required "The destination address")
                (@arg MODE: +required "The mode to set")
            )
            (setting: clap::AppSettings::SubcommandRequiredElseHelp)
        )
        (@subcommand power => )
    }
}

fn monitor<R: Read>(sock: &mut R) -> Result<(), failure::Error> {
    while let Some(message) = labctl::cand::read_packet(sock)? {
        match message {
            Message::Frame(can_packet) => {
                println!(
                    "    {} -> {} {}",
                    can_packet.src,
                    can_packet.dest,
                    can_packet.payload
                        .iter()
                        .map(|b| format!("{:02x}", b))
                        .collect::<Vec<_>>()
                        .join(" ")
                );
            }
            Message::Reset { cause } => {
                println!("[*] Gateway Reset, cause: {}", cause)
            }
            Message::Ping => {
                println!("[*] Ping Response")
            }
            Message::Resync => {
                // Will usually not be transmitted to clients
                println!("[?] Ping Response")
            }
            Message::VersionRequest => {
                // Will usually not be transmitted to clients
                println!("[?] Version Request")
            }
            Message::VersionReply { major, minor } => {
                println!("[*] Gateway has version {}.{}", major, minor)
            }
            Message::FirmwareIdRequest => {
                println!("[?] Firmware ID Request")
            }
            Message::FirmwareIdResponse(fwid) => {
                println!("[*] Firmware ID String is {:?}", fwid);
            }
            Message::Unknown { kind, payload } => {
                println!("[!] Unknown Packet (Type {}): {}", kind, hex::encode(&payload))
            }
            Message::BusPowerRequest => {
                println!("[?] Bus Power Request")
            }
            Message::BusPowerResponse { v, i, reference, gnd } => {
                println!("[*] Bus Power: U: {}, I: {}, ref: {}, gnd: {}", v, i, reference, gnd)
            }
        }
    }
    Ok(())
}

fn borg_text<W: Write>(write: &mut W, text: &str, dst: CanAddr) -> Result<(), failure::Error> {
    for p in labctl::lap::set_scroll_text(text, CanAddr::new(0, 0x23)?, dst) {
        labctl::cand::write_packet_to_cand(write, &Message::Frame(p))?;
        write.flush()?;
        thread::sleep(Duration::from_millis(30));
    }
    Ok(())
}

fn borg_mode<W: Write>(write: &mut W, mode: u8, dst: CanAddr) -> Result<(), failure::Error> {
    let p = labctl::lap::BorgMode(mode)
        .to_can(CanAddr::new(0, 0x23)?, dst);
    labctl::cand::write_packet_to_cand(write, &Message::Frame(p))?;
    Ok(())
}

fn bus_power<W: Write, R: Read>(write: &mut W, read: &mut R) -> Result<(), failure::Error> {
    labctl::cand::write_packet_to_cand(write, &Message::BusPowerRequest)?;
    while let Some(msg) = labctl::cand::read_packet(read)? {
        match msg {
            Message::BusPowerResponse { v, i, reference, gnd } => {
                let uadc = v as f64 * (5f64 / 1023f64);
                let ubus = (uadc * 3_700f64) / 1_000f64;
                let iadc = i as f64 * (5f64 / 1023f64);
                let ibus = iadc / (10f64 * 0.01);

                println!("Bus Power:");
                println!("  U:   {:.02} V", ubus);
                println!("  I:   {:.02} A", ibus);
                println!("    => {:.02} W", ubus * ibus);
                println!("  ref: {}", reference);
                println!("  gnd: {}", gnd);
                break;
            },
            _ => {}
        }
    }
    Ok(())
}

fn main() -> Result<(), failure::Error> {

    let matches = args().get_matches();

    let host = matches.value_of("host").unwrap();
    let port = matches.value_of("port").unwrap_or("2342").parse()?;

    let mut s = TcpStream::connect((host, port)).unwrap();

    match matches.subcommand() {
        ("monitor", _args) => {
            monitor(&mut s)?;
        }
        ("borg", Some(borg_args)) => {
            match borg_args.subcommand() {
                ("text", Some(text_args)) => {
                    let text = text_args.value_of("TEXT").unwrap();
                    let dst = text_args.value_of("DEST")
                        .unwrap()
                        .parse()
                        .unwrap();
                    let now = text_args.is_present("now");
                    borg_text(&mut s, text, dst)?;
                    if now {
                        borg_mode(&mut s, 1, dst)?;
                    }
                },
                ("mode", Some(mode_args)) => {
                    let dst = mode_args.value_of("DEST")
                        .unwrap()
                        .parse()
                        .unwrap();
                    let mode = mode_args.value_of("MODE")
                        .unwrap()
                        .parse()
                        .unwrap();
                    borg_mode(&mut s, mode, dst)?;
                },
                _ => unreachable!()
            }
        },
        ("power", _) => {
            bus_power(&mut s.try_clone().unwrap(), &mut s)?;
        }
        _ => unreachable!()
    }

    s.flush()?;
    // Because cand is a piece of crap we need to sleep a tiny bit before closing the socket
    thread::sleep(Duration::from_millis(10));
    //let mut s = TcpStream::connect("10.0.1.4:2342").unwrap();

    /*let p = lap::SetLampPacket {
        mode: LampMode::Toggle,
        lamp_id: 6,
        value: 255
    };*/
    //let pac = p.to_can(CanAddr(0, 2), CanAddr(60, 2));

    
    /*let pac = CanPacket {
        src_addr: 0,
        dest_addr: 0x24,
        src_port: 0x23,
        dest_port: 0x23,
        payload: vec![1, 4]
    };*/

    Ok(())
}
