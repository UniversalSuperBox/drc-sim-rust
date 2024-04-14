use arbitrary_int::{u10, u11, u2, u4};
use drc_sim_rust_lib::incoming_packet_parser::WUPVideoPacket;

pub fn data_ones() -> WUPVideoPacket {
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
