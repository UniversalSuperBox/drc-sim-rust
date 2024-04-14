use std::{cmp::Ordering, panic::catch_unwind};

use arbitrary_int::{u10, u11, u2, u4};
use drc_sim_rust_lib::incoming_packet_parser::{
    process_video_packet, u10_paws_compare, u32_paws_compare, WUPVideoPacket
};
use proptest::prelude::*;

/** Converts a WUPVideoPacket into big-endian bytes as parsed by
process_video_packet.

I have no idea whether this will work on big-endian systems.
*/
fn data_from_wupvideopacket(input: WUPVideoPacket) -> Result<Vec<u8>, &'static str> {
    let mut data: Vec<u8> = Vec::new();
    let seq_id = u16::from(input.seq_id).to_be_bytes();

    let first_byte: u8 = (u8::from(input.magic) << 4)
        | (u8::from(input.packet_type) << 2)
        | (u8::from(seq_id[0]) >> 6)
        | seq_id[0];
    data.push(first_byte);

    data.push(seq_id[1]);

    let payload_size = u16::from(input.payload_size).to_be_bytes();
    let third_byte = ((input.init as u8) << 7)
        | ((input.frame_begin as u8) << 6)
        | ((input.chunk_end as u8) << 5)
        | ((input.frame_end as u8) << 4)
        | ((input.has_timestamp as u8) << 3)
        | payload_size[0];
    data.push(third_byte);

    data.push(payload_size[1]);

    data.extend(input.timestamp.to_be_bytes());

    data.extend(input.extended_header);

    data.extend(input.payload);

    return Ok(data);
}

const ONES_SLICE: [u8; 17] = [
    0xF0, 0x1, // magic, packet_type, seq_id
    0x8, //init, frame_begin, chunk_end, frame_end, has_timestamp, first 3 of payload_size
    0x1, // other 8 of payload_size
    0x0, 0x0, 0x0, 0x1, //timestamp
    0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0,  //extended_header
    0x01, // payload
];

fn data_ones() -> WUPVideoPacket {
    return WUPVideoPacket {
        magic: u4::new(15),
        packet_type: u2::new(0),
        seq_id: u10::new(1),
        init: false,
        frame_begin: false,
        chunk_end: false,
        frame_end: false,
        has_timestamp: true,
        payload_size: u11::new(1),
        timestamp: 1,
        extended_header: 0u64.to_be_bytes(),
        payload: Vec::from([0x1]),
    };
}

const CHRISTMAS_TREE_SLICE: [u8; 17] = [
    0xF3, 0xFF, // magic, packet_type, seq_id
    0xF8, //init, frame_begin, chunk_end, frame_end, has_timestamp, first 3 of payload_size
    0x1,  // other 8 of payload_size
    0xFF, 0xFF, 0xFF, 0xFF, //timestamp
    0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, //extended_header
    0xFF, // payload
];

fn data_christmas_tree() -> WUPVideoPacket {
    return WUPVideoPacket {
        magic: u4::new(15),
        packet_type: u2::new(0),
        seq_id: u10::new(1023),
        init: true,
        frame_begin: true,
        chunk_end: true,
        frame_end: true,
        has_timestamp: true,
        payload_size: u11::new(1),
        timestamp: 0xFFFFFFFF,
        extended_header: 0xFFFFFFFFFFFFFFFFu64.to_be_bytes(),
        payload: Vec::from([0xFF]),
    };
}

#[test]
fn test_data_from_wupvideopacket_ones() {
    assert_eq!(
        data_from_wupvideopacket(data_ones()).unwrap(),
        [0xF0, 1, 8, 1, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 1]
    );
}

#[test]
fn test_data_from_wupvideopacket_christmastree() {
    assert_eq!(
        data_from_wupvideopacket(data_christmas_tree()).unwrap(),
        CHRISTMAS_TREE_SLICE
    );
}

#[test]
fn christmas_tree_video_packet() {
    assert_eq!(
        process_video_packet(&CHRISTMAS_TREE_SLICE),
        Some(data_christmas_tree())
    );
}

#[test]
fn ones_video_packet() {
    assert_eq!(process_video_packet(&ONES_SLICE), Some(data_ones()));
}

#[test]
fn fail_with_invalid_magic() {
    let mut packet = data_ones();
    packet.magic = u4::new(14);
    assert_eq!(
        process_video_packet(&data_from_wupvideopacket(packet).unwrap()),
        None
    );
}

#[test]
fn fail_with_invalid_type() {
    let mut packet = CHRISTMAS_TREE_SLICE.clone();
    packet[0] = 0xF8;
    assert_eq!(process_video_packet(&packet), None);
}

// I'm mostly using this to learn about property-based testing... given
// writing these tests requires reimplementing data_from_wupvideopacket,
// I don't think they're the best tests ever created.
proptest! {
    #[test]
    fn twiddle_first_two_bytes(magic in 0..15u8, packet_type in 0..3u8, seq_id in 0..1023u16) {
        do_first_bytes_test(magic, packet_type, seq_id);
    }
}

fn do_first_bytes_test(magic: u8, packet_type: u8, seq_id: u16) {
    let mut packet = data_ones();
    packet.magic = u4::new(magic);
    packet.packet_type = u2::new(packet_type);
    packet.seq_id = u10::new(seq_id);
    if magic > 15 || packet_type > 3 || seq_id > 1023 {
        let result = catch_unwind(|| data_from_wupvideopacket(packet));
        assert!(result.is_err());
    } else {
        let split_seq_id = seq_id.to_be_bytes();
        let first_byte = (magic << 4) | (packet_type << 2) | split_seq_id[0];
        assert_eq!(
            data_from_wupvideopacket(packet).unwrap(),
            [
                first_byte,
                split_seq_id[1],
                8,
                1,
                0,
                0,
                0,
                1,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                1
            ]
        );
    }
}

/// Ensures that the comparison of s and t equals comparison, and that
/// the inverse is also true.
fn do_u32_timestamp_compare_test(s: u32, t: u32, comparison: Ordering) {
    let result = u32_paws_compare(s, t);
    assert_eq!(
        result,
        Some(comparison),
        "{:#X} was not {:?} to {:#X} (it was {:?})",
        s,
        comparison,
        t,
        result
    );
    let comparison = comparison.reverse();
    let result = u32_paws_compare(t, s);
    assert_eq!(
        result,
        Some(comparison),
        "on inversion test, {:#X} was not {:?} to {:#X} (it was {:?})",
        s,
        comparison,
        t,
        result
    );
}

// This can be seen as a "sanity check" for the property-based test for
// u32_timestamp_compare: a few values that a human checked the answers for.
#[test]
fn test_u32_timestamp_compare() {
    do_u32_timestamp_compare_test(0xFFFFFFFF, 0x00000000, Ordering::Less);
    do_u32_timestamp_compare_test(0xFFFFFFFF, 0x00000001, Ordering::Less);
    do_u32_timestamp_compare_test(0xFFFFFFFF, 0x000000F1, Ordering::Less);
    do_u32_timestamp_compare_test(0xFFFFFFFF, 0x000000FF, Ordering::Less);
    do_u32_timestamp_compare_test(0xFFFFFFFF, 0x0000FFFE, Ordering::Less);
    do_u32_timestamp_compare_test(0xFFFFFFFF, 0xFFFFFFFE, Ordering::Greater);
    do_u32_timestamp_compare_test(0xFFFFFFFF, 0xFFFF0000, Ordering::Greater);
    do_u32_timestamp_compare_test(0xFFFFFFFF, 0x0FFFFFFF, Ordering::Less);
    do_u32_timestamp_compare_test(0xFFFFFFFF, 0x0FFFFFFF, Ordering::Less);
    do_u32_timestamp_compare_test(0xFFFFFFFF, 0x10000000, Ordering::Less);
    do_u32_timestamp_compare_test(0xFFFFFFFF, 0x70000000, Ordering::Less);
    do_u32_timestamp_compare_test(0xFFFFFFFF, 0x7FFFFFFE, Ordering::Less);
    do_u32_timestamp_compare_test(0xFFFFFFFF, 0x80000000, Ordering::Greater);
    // The comparison between 0x7FFFFFFF and 0xFFFFFFFF is difficult
    // because their difference is exactly 2**31.
    assert_eq!(u32_paws_compare(0xFFFFFFFF, 0x7FFFFFFF), None);
    assert_eq!(u32_paws_compare(0x7FFFFFFF, 0xFFFFFFFF), None);

    // Try the hard comparison again but with an arbitrarily chosen s
    assert_eq!(u32_paws_compare(0x9E911F8, 0x89E911F8), None);
    assert_eq!(u32_paws_compare(0x89E911F8, 0x9E911F8), None);
}

proptest! {
    #[test]
    fn test_u32_timestamp_compare_prop(s in 0u32..0xFFFFFFFFu32, difference in 0u32..0xFFFFFFFFu32) {
        do_u32_timestamp_compare_prop(s, difference);
    }
    #[test]
    fn test_u10_timestamp_compare_prop(s in 0u16..0x3FFu16, difference in 0u16..0x3FFu16) {
        do_u10_timestamp_compare_prop(s, difference);
    }
}

fn do_u32_timestamp_compare_prop(s: u32, difference: u32) {
    let (expected, inverse_expected) = match difference {
        0 => (Some(Ordering::Equal), Some(Ordering::Equal)),
        0x80000000 => (None, None),
        other => {
            let expected = other.cmp(&0x80000000u32);
            (Some(expected.reverse()), Some(expected))
        }
    };
    let t = s.wrapping_sub(difference);
    assert_eq!(u32_paws_compare(s, t), expected);
    assert_eq!(u32_paws_compare(t, s), inverse_expected);
}

fn do_u10_timestamp_compare_prop(s: u16, difference: u16) {
    let (expected, inverse_expected) = match difference {
        0 => (Some(Ordering::Equal), Some(Ordering::Equal)),
        0x200 => (None, None),
        other => {
            let expected = other.cmp(&0x200);
            (Some(expected.reverse()), Some(expected))
        }
    };
    let s = u10::new(s);
    let difference = u10::new(difference);
    let t = s.wrapping_sub(difference);
    assert_eq!(u10_paws_compare(s, t), expected);
    assert_eq!(u10_paws_compare(t, s), inverse_expected);
}
