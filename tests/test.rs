#[cfg(test)]
#[macro_use]
extern crate assert_matches;

use drc_sim_rust_lib::incoming_packet_parser::{process_video_packet, WUPVideoPacket};

#[test]
fn christmas_tree_video_packet() {
    let packet: [u8; 17] = [
        0xF3, 0xFF, // magic, packet_type, seq_id
        0xFF, //init, frame_begin, chunk_end, frame_end, has_timestamp, first 3 of payload_size
        0xFF, // other 8 of payload_size
        0xFF, 0xFF, 0xFF, 0xFF, //timestamp
        0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, //extended_header
        0xFF, // payload
    ];
    assert_matches!(
        process_video_packet(&packet),
        Some(WUPVideoPacket {
            magic: 0xF,
            packet_type: 0,
            seq_id: 0x3FF,
            init: true,
            frame_begin: true,
            chunk_end: true,
            frame_end: true,
            has_timestamp: true,
            payload_size: 0x7FF,
            timestamp: 0xFFFFFFFF,
            extended_header: [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF],
            payload: &[0xFF],
        })
    );
}
