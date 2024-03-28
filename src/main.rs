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


use std::{net::UdpSocket};
use bitter::{BigEndianReader, BitReader};
use log::{debug, error, info, max_level, trace, LevelFilter};
use simple_logger;

// const PORT_WII_MSG: u16 = 50010;
const PORT_WII_VID: u16 = 50120;
// const PORT_WII_AUD: u16 = 50121;
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
//     extended_header: u8,
//     payload: [u8],
// });

fn main() -> std::io::Result<()>{
    simple_logger::init_with_env().unwrap();
    {
        //TODO: Bind to the appropriate IP address (It's usually 192.168.1.11 but could be different)
        let bindaddr = format!("0.0.0.0:{}", PORT_WII_VID);

        let video_socket = UdpSocket::bind(bindaddr)?;

        let mut processed = 0;

        loop {
            processed += 1;
            // The max size is really 2017 bytes, but bitter prefers
            // reading 64 bit chunks so we'll fill all the way to 2048
            let mut buf = [0u8; 2048];
            let (amt, src) = video_socket.recv_from(&mut buf)?;

            // The WUP datagram has a 16 byte header and should always
            // have at least a single-byte payload.
            if amt < 17 {
                info!("Packet from {src} was too short at only {amt} bytes");
                continue
            }

            let mut bits = BigEndianReader::new(&buf);

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
            if init {
                debug!("init!");
            }
            bits.consume(1);
            let frame_begin = bits.peek(1) != 0;
            if frame_begin {
                debug!("frame_begin!");
            }
            bits.consume(1);
            let chunk_end = bits.peek(1) != 0;
            if chunk_end {
                debug!("chunk_end!");
            }
            bits.consume(1);
            let frame_end = bits.peek(1) != 0;
            if frame_end {
                debug!("frame_end!");
            }
            bits.consume(1);
            let has_timestamp = bits.peek(1) != 0;
            assert!(has_timestamp, "Packet with no timestamp");
            bits.consume(1);
            let expected_payload_size_bytes = bits.peek(11) as usize;
            bits.consume(11);

            // The next 64 bits have <something else> that isn't the
            // payload
            let mut extended_header = [0u8; 8];
            if ! bits.read_bytes(&mut extended_header) {
                error!("Packet did not have an extended header");
                continue;
            }

            let mut payload: Vec<u8> = Vec::with_capacity(expected_payload_size_bytes);
            let remaining_dgram_bits = bits.bytes_remaining();
            if remaining_dgram_bits >= expected_payload_size_bytes {
                loop {
                    let payload_len = payload.len();
                    let still_unread = expected_payload_size_bytes - payload_len;

                    if still_unread == 0 {
                        break;
                    }

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
                continue;
            }
            if max_level() >= LevelFilter::Trace {
                let payload_len = payload.len();
                trace!("Good: Got a payload of {payload_len:?} bytes, expected {expected_payload_size_bytes}");
            }
            if processed > 1000 {
                info!("Finished {processed} packets");
                break;
            }
        }
    }
    Ok(())
}
