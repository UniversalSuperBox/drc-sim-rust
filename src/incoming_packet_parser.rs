// Copyright 2024 Dalton Durst and the drc-sim-rust contributors
// SPDX-License-Identifier: MPL-2.0

// const WII_VIDEO_WIDTH: u16 = 848;
// const WII_VIDEO_HEIGHT: u16 = 480;

use core::fmt;
use std::cmp::Ordering;

use arbitrary_int::{u10, u11, u2, u4};
use bitter::{BigEndianReader, BitReader};
use log::error;

#[derive(PartialEq, Clone)]
pub struct WUPVideoPacket {
    pub magic: u4,                // 4
    pub packet_type: u2,          // 2
    pub seq_id: u10,              // 10 (16b/2B)
    pub init: bool,               // 1
    pub frame_begin: bool,        // 1
    pub chunk_end: bool,          // 1
    pub frame_end: bool,          // 1
    pub has_timestamp: bool,      // 1
    pub payload_size: u11,        // 11 (32b/4B)
    pub timestamp: u32, // 32, counts in microseconds, overflows every ~1.19 hours (64b/8B)
    pub extended_header: [u8; 8], // 64 (128b/16B)
    pub payload: Vec<u8>, // up to 2047 bytes, I've never seen larger than 1672
                        // minimum 17B, maximum 2063B (but I don't think the WUP actually
                        // sends dgrams that large)
}

impl fmt::Debug for WUPVideoPacket {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Point")
            .field("magic", &self.magic)
            .field("packet_type", &self.packet_type)
            .field("seq_id", &self.seq_id)
            .field("init", &self.init)
            .field("frame_begin", &self.frame_begin)
            .field("chunk_end", &self.chunk_end)
            .field("frame_end", &self.frame_end)
            .field("has_timestamp", &self.has_timestamp)
            .field("payload_size", &self.payload_size)
            .field("timestamp", &self.timestamp)
            .field("extended_header", &self.extended_header)
            .field("payload", &format!("size {}", &self.payload.len()))
            .finish()
    }
}

/// Compares s against t with the RFC 1323 PAWS algorithm. Returns None
/// when s and t are exactly 0x80000000 apart as it is not possible to
/// know which is higher. Returns the appropriate ordering for wrapping
/// 32-bit values (where 0 is greater than 0xFFFFFFFF, for example) in
/// all other cases.
/// https://blog.theprogrammingjunkie.com/post/paws/
pub fn u32_paws_compare(s: u32, t: u32) -> Option<Ordering> {
    if s == t {
        return Some(Ordering::Equal);
    }
    return paws_comparison(t.wrapping_sub(s), 2u32.pow(31));
}

/// Compares s against t with the RFC 1323 PAWS algorithm. Returns None
/// when s and t are exactly 0x200 apart as it is not possible to
/// know which is higher. Returns the appropriate ordering for wrapping
/// 10-bit values (where 0 is greater than 0x3FF, for example) in
/// all other cases.
/// https://blog.theprogrammingjunkie.com/post/paws/
pub fn u10_paws_compare(s: u10, t: u10) -> Option<Ordering> {
    if s == t {
        return Some(Ordering::Equal);
    }
    return paws_comparison(t.wrapping_sub(s), u10::new(2u16.pow(9)));
}

pub fn paws_comparison<T: Eq + PartialOrd>(diff: T, half_space: T) -> Option<Ordering> {
    if diff == half_space {
        return None;
    }
    if diff < half_space {
        return Some(Ordering::Less);
    }
    return Some(Ordering::Greater);
}

pub fn process_video_packet(packet: &[u8]) -> Option<WUPVideoPacket> {
    let mut bits = BigEndianReader::new(packet);

    let packet_len = packet.len();
    if packet_len < 17 {
        error!("packet was too short to process as video");
    }

    let len = bits.refill_lookahead();
    assert!(len >= 16);

    // first 16 bits of the header tell us what type it is and
    // where it fits in order
    let magic = bits.peek(4) as u8;
    if magic != 15 {
        error!("Unknown magic {magic}");
        return None;
    }
    bits.consume(4);
    let packet_type = bits.peek(2) as u8;
    if packet_type != 0 {
        error!("Unknown packet type {packet_type}");
        return None;
    }
    bits.consume(2);
    let seq_id = bits.peek(10) as u16;
    bits.consume(10);

    let len = bits.refill_lookahead();
    assert!(len >= 16);

    // next 16 bits regard what this video packet looks like
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

    // This goes past the bitter manual lookahead but that's probably
    // fine.
    let timestamp = bits.read_u32()?;

    // The next 64 bits have <something else> that isn't the
    // payload
    let mut extended_header = [0u8; 8];
    if !bits.read_bytes(&mut extended_header) {
        error!("Packet did not have an extended header");
        return None;
    }

    let expected_packet_len = expected_payload_size_bytes as usize + 16;
    if packet_len < expected_packet_len {
        error!(
            "Packet data was only {} bytes, need {}",
            packet_len, expected_packet_len
        );
        return None;
    }

    return Some(WUPVideoPacket {
        magic: u4::new(magic),
        packet_type: u2::new(packet_type),
        seq_id: u10::new(seq_id),
        init: init,
        frame_begin: frame_begin,
        chunk_end: chunk_end,
        frame_end: frame_end,
        has_timestamp: has_timestamp,
        payload_size: u11::new(expected_payload_size_bytes),
        timestamp: timestamp,
        extended_header: extended_header,
        payload: packet[16..(expected_payload_size_bytes as usize + 16)].to_vec(),
    });
}
