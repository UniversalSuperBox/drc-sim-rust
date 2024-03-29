// Copyright 2024 Dalton Durst
//
// Permission is hereby granted, free of charge, to any person obtaining
// a copy of this software and associated documentation files (the
// “Software”), to deal in the Software without restriction, including
// without limitation the rights to use, copy, modify, merge, publish,
// distribute, sublicense, and/or sell copies of the Software, and to
// permit persons to whom the Software is furnished to do so, subject to
// the following conditions:
//
// The above copyright notice and this permission notice shall be
// included in all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED “AS IS”, WITHOUT WARRANTY OF ANY KIND,
// EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF
// MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND
// NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS
// BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN
// ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN
// CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.


use core::str;
use std::{fs::File, io::{BufReader, BufWriter, Read, Write}, net::UdpSocket};
use bitter::{BigEndianReader, BitReader};
use log::{ error, info, max_level, trace, LevelFilter};
use simple_logger;

// const PORT_WII_MSG: u16 = 50010;
const PORT_WUP_VID: u16 = 50120;
const PORT_WUP_AUD: u16 = 50121;
// const PORT_WII_HID: u16 = 50122;
// const PORT_WII_CMD: u16 = 50123;

// const WII_VIDEO_WIDTH: u16 = 848;
// const WII_VIDEO_HEIGHT: u16 = 480;

// binary_layout!(video_packet, BigEndian, {
//     // magic: [u8; 4],
//     // packet_type: [u8; 2],
//     // seq_id: [u8; 10],
//     header_1: u16,
//     // init: bool,
//     // frame_begin: bool,
//     // chunk_end: bool,
//     // frame_end: bool,
//     // has_timestamp: bool,
//     // payload_size: 11 bit unsigned,
//     header_2: [u8; 2],
//     timestamp: u32,
//     extended_header: u64,
//     payload: [u8],
// });

#[derive(Debug)]
struct WiiUVideoPacket {
    magic: u8,
    packet_type: u8,
    seq_id: u16,
    init: bool,
    frame_begin: bool,
    chunk_end: bool,
    frame_end: bool,
    has_timestamp: bool,
    payload_size: u16,
    timestamp: u32,
    extended_header: [u8; 8],
    payload: Vec<u8>
}

fn get_socket(dest_ip: &str, port: u16) -> Result<UdpSocket, std::io::Error> {
    let addr = format!("{}:{}", dest_ip, port);
    return UdpSocket::bind(addr);
}

fn get_vid_socket(dest_ip: &str) -> Result<UdpSocket, std::io::Error> {
    return get_socket(dest_ip, PORT_WUP_VID);
}

fn get_aud_socket(dest_ip: &str) -> Result<UdpSocket, std::io::Error> {
    return get_socket(dest_ip, PORT_WUP_AUD);
}

fn process_video_packet(packet: &[u8]) -> Option<WiiUVideoPacket> {
    let mut bits = BigEndianReader::new(&packet);

    let len = bits.refill_lookahead();
    assert!(len >= 32);

    // first 32 bits of the header tell us what type it is and
    // where it fits in order
    let magic = bits.peek(4) as u8;
    assert!(magic == 15, "Unknown magic {magic}");
    bits.consume(4);
    let packet_type = bits.peek(2) as u8;
    assert!(packet_type == 0, "Unknown packet type {packet_type}");
    bits.consume(2);
    let seq_id = bits.peek(10) as u16;
    bits.consume(10);

    let len = bits.refill_lookahead();
    assert!(len >= 32);

    // next 32 bits regard what this video packet looks like
    let init = bits.peek(1) != 0;
    bits.consume(1);
    let frame_begin = bits.peek(1) != 0;
    bits.consume(1);
    let chunk_end = bits.peek(1) != 0;
    bits.consume(1);
    let frame_end = bits.peek(1) != 0;
    bits.consume(1);
    let has_timestamp = bits.peek(1) != 0;
    assert!(has_timestamp, "Packet with no timestamp");
    bits.consume(1);
    let expected_payload_size_bytes = bits.peek(11) as u16;
    bits.consume(11);

    let timestamp = bits.read_u32()?;

    // The next 64 bits have <something else> that isn't the
    // payload
    let mut extended_header = [0u8; 8];
    if ! bits.read_bytes(&mut extended_header) {
        error!("Packet did not have an extended header");
        return None
    }

    let mut payload: Vec<u8> = Vec::with_capacity(expected_payload_size_bytes as usize);
    let remaining_dgram_bits = bits.bytes_remaining();
    if remaining_dgram_bits >= expected_payload_size_bytes as usize {
        loop {
            let still_unread = expected_payload_size_bytes as usize - payload.len();

            if still_unread == 0 {
                trace!("Finished reading dgram");
                break;
            }

            trace!("Have {still_unread} left to read");

            let first_byte = if still_unread > 8 {
                1
            } else {
                8 - still_unread
            };

            //TODO: If this is too slow, we could speed it up
            // with bitter's unsafe API. We just need to make
            // sure we only consume the bytes we need and only
            // take the appropriate number of bytes from the u64
            let consumable = bits.refill_lookahead();
            let piece = bits.peek(consumable);
            bits.consume(consumable);
            let split_piece = &piece.to_be_bytes()[first_byte..8];
            payload.extend(split_piece);
        }
    } else {
        error!("Somehow we don't have {expected_payload_size_bytes} bits remaining, we have {remaining_dgram_bits:?}");
        return None
    }
    if max_level() >= LevelFilter::Trace {
        let payload_len = payload.len();
        trace!("Read {payload_len:?} bytes, expected {expected_payload_size_bytes}");
    }

    return Some(WiiUVideoPacket{
        magic: magic,
        packet_type: packet_type,
        seq_id: seq_id,
        init: init,
        frame_begin: frame_begin,
        chunk_end: chunk_end,
        frame_end: frame_end,
        has_timestamp: has_timestamp,
        payload_size: expected_payload_size_bytes,
        timestamp: timestamp,
        extended_header: extended_header,
        payload: payload
    })
}

fn writer_main() -> std::io::Result<()>{
    {
        //TODO: Bind to the appropriate IP address (It's usually 192.168.1.11 but could be different)
        let video_socket = get_vid_socket("0.0.0.0")?;

        let mut file_writer = BufWriter::new(File::create("video_packets")?);

        for _n in 1..=10000 {
            // The max size is really 2017 bytes, but bitter prefers
            // reading 64 bit chunks so we'll fill all the way to 2048
            let mut buf = [0u8; 2048];
            let (amt, src) = video_socket.recv_from(&mut buf)?;

            // The WUP datagram has a 16 byte header and should always
            // have at least a single-byte payload.
            if amt < 17 {
                info!("Packet from {src} was too short at only {amt} bytes");
                continue;
            }

            file_writer.write(&buf)?;
        }
    Ok(())
    }
}

fn main() -> std::io::Result<()>{
    simple_logger::init_with_env().unwrap();
    {
        //TODO: Bind to the appropriate IP address (It's usually 192.168.1.11 but could be different)
        // let video_socket = get_vid_socket("0.0.0.0")?;

        let mut file_reader = BufReader::new(File::open("video_packets")?);

        for _n in 1..=1000 {
            // The max size is really 2017 bytes, but bitter prefers
            // reading 64 bit chunks so we'll fill all the way to 2048
            let mut buf = [0u8; 2048];
            let amt = file_reader.read(&mut buf)?;

            // The WUP datagram has a 16 byte header and should always
            // have at least a single-byte payload.
            if amt < 17 {
                info!("Packet was too short at only {amt} bytes");
                continue;
            }

            let packet = match process_video_packet(&buf) {
                None => {
                    error!("Didn't get a packet back from process");
                    continue
                }
                Some(val) => val
            };

            trace!("{packet:?}");
        }
    }
    Ok(())
}
