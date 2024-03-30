// Copyright 2024 Dalton Durst and the drc-sim-rust contributors
// SPDX-License-Identifier: MPL-2.0

// const WII_VIDEO_WIDTH: u16 = 848;
// const WII_VIDEO_HEIGHT: u16 = 480;

use bitter::{BigEndianReader, BitReader};
use log::{error, max_level, trace, LevelFilter};

#[derive(Debug)]
pub struct WiiUVideoPacket {
    magic: u8,                // 4
    packet_type: u8,          // 2
    seq_id: u16,              // 10 (16b/2B)
    init: bool,               // 1
    frame_begin: bool,        // 1
    chunk_end: bool,          // 1
    frame_end: bool,          // 1
    has_timestamp: bool,      // 1
    payload_size: u16,        // 11 (32b/4B)
    timestamp: u32,           // 32 (64b/8B)
    extended_header: [u8; 8], // 64 (128b/16B)
    payload: Vec<u8>,         // up to 2047 bytes
                              // minimum 17B, maximum 2063B (but I don't think the WUP actually
                              // sends dgrams that large)
}

pub fn process_video_packet(packet: &[u8]) -> Option<WiiUVideoPacket> {
    let mut bits = BigEndianReader::new(&packet);

    if packet.len() < 17 {
        error!("packet was too short to process as video");
    }

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
    if !bits.read_bytes(&mut extended_header) {
        error!("Packet did not have an extended header");
        return None;
    }

    let mut payload: Vec<u8> = Vec::with_capacity(expected_payload_size_bytes as usize);
    let remaining_dgram_bits = bits.bytes_remaining();
    if remaining_dgram_bits >= expected_payload_size_bytes as usize {
        payload.reserve(expected_payload_size_bytes as usize);
        loop {
            let still_unread = expected_payload_size_bytes as usize - payload.len();

            if still_unread == 0 {
                trace!("Finished reading dgram");
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
        return None;
    }
    if max_level() >= LevelFilter::Trace {
        let payload_len = payload.len();
        trace!("Read {payload_len:?} bytes, expected {expected_payload_size_bytes}");
    }

    return Some(WiiUVideoPacket {
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
        payload: payload,
    });
}
