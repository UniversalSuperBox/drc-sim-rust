use drc_sim_rust_lib::incoming_packet_parser::{process_video_packet, WUPVideoPacket};

/** Converts a WUPVideoPacket into big-endian bytes as parsed by
process_video_packet.

I have no idea whether this will work on big-endian systems.
*/
fn data_from_wupvideopacket(input: WUPVideoPacket) -> Vec<u8> {
    assert!(
        input.seq_id <= 1023,
        "seq_id is only a 10-bit number on the wire."
    );
    assert!(
        input.payload_size <= 2047,
        "payload_size is only an 11-bit number on the wire."
    );

    let mut data: Vec<u8> = Vec::new();
    let seq_id = input.seq_id.to_be_bytes();

    let first_byte: u8 =
        (input.magic << 4) | (input.packet_type << 2) | (seq_id[0] >> 6) | seq_id[0];
    data.push(first_byte);

    data.push(seq_id[1]);

    let payload_size = input.payload_size.to_be_bytes();
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

    return data;
}

const ONES: WUPVideoPacket = WUPVideoPacket {
    magic: 15,
    packet_type: 0,
    seq_id: 1,
    init: false,
    frame_begin: false,
    chunk_end: false,
    frame_end: false,
    has_timestamp: true,
    payload_size: 1,
    timestamp: 1,
    extended_header: 0u64.to_be_bytes(),
    payload: &1u8.to_be_bytes(),
};

const CHRISTMAS_TREE_PKT: WUPVideoPacket = WUPVideoPacket {
    magic: 15,
    packet_type: 0,
    seq_id: 1023,
    init: true,
    frame_begin: true,
    chunk_end: true,
    frame_end: true,
    has_timestamp: true,
    payload_size: 2047,
    timestamp: 0xFFFFFFFF,
    extended_header: 0xFFFFFFFFFFFFFFFFu64.to_be_bytes(),
    payload: &0xFFu8.to_be_bytes(),
};

const CHRISTMAS_TREE_SLICE: [u8; 17] = [
    0xF3, 0xFF, // magic, packet_type, seq_id
    0xFF, //init, frame_begin, chunk_end, frame_end, has_timestamp, first 3 of payload_size
    0xFF, // other 8 of payload_size
    0xFF, 0xFF, 0xFF, 0xFF, //timestamp
    0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, //extended_header
    0xFF, // payload
];

#[test]
fn test_data_from_wupvideopacket_ones() {
    assert_eq!(
        data_from_wupvideopacket(ONES),
        [0xF0, 1, 8, 1, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 1]
    );
}

#[test]
fn test_data_from_wupvideopacket_christmastree() {
    assert_eq!(
        data_from_wupvideopacket(CHRISTMAS_TREE_PKT),
        [
            0xF3, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
            0xFF, 0xFF, 0xFF
        ]
    );
}

#[test]
fn christmas_tree_video_packet() {
    assert_eq!(
        process_video_packet(&CHRISTMAS_TREE_SLICE),
        Some(CHRISTMAS_TREE_PKT)
    );
}

#[test]
fn test_data_from_wupvideopacket_magic() {
    for i in 0..15 {
        let mut packet = ONES.clone();
        packet.magic = i;
        let first_byte = i << 4;
        assert_eq!(
            data_from_wupvideopacket(packet),
            [
                first_byte, 1, 8, 1, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 1
            ]
        )
    }
}
