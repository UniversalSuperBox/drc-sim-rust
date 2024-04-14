use arbitrary_int::u10;
use drc_sim_rust_lib::{
    incoming_packet_parser::WUPVideoPacket, packet_organizer::FrameAccumulator,
};
use proptest::proptest;

mod common;

#[test]
fn test_single_dgram_accumulator() {
    let mut packet = common::data_ones();
    packet.seq_id = u10::new(284);
    packet.frame_begin = true;
    packet.frame_end = true;
    packet.timestamp = 127384127;

    let mut accumulator = FrameAccumulator::new(packet.timestamp);
    assert!(accumulator.add_packet(packet).is_ok());
    let completed = accumulator.complete();
    assert!(completed.is_ok(), "complete() returned {:?}", completed);
}

#[test]
fn test_two_dgram_accumulator() {
    let mut packet1 = common::data_ones();
    packet1.seq_id = u10::new(284);
    packet1.frame_begin = true;
    packet1.frame_end = false;
    packet1.timestamp = 127384127;

    let mut packet2 = common::data_ones();
    packet2.seq_id = u10::new(285);
    packet2.frame_begin = false;
    packet2.frame_end = true;
    packet2.timestamp = 127384127;

    let mut accumulator = FrameAccumulator::new(packet1.timestamp);
    assert!(accumulator.add_packet(packet1.clone()).is_ok());
    assert!(accumulator.add_packet(packet2.clone()).is_ok());
    let completed = accumulator.complete();
    assert!(completed.is_ok(), "complete() returned {:?}", completed);
    let completed = completed.unwrap();
    assert_eq!(completed[0..], [&packet1, &packet2]);
}

#[test]
fn test_three_dgram_accumulator() {
    let mut packet1 = common::data_ones();
    packet1.seq_id = u10::new(284);
    packet1.frame_begin = true;
    packet1.frame_end = false;
    packet1.timestamp = 127384127;

    let mut packet2 = common::data_ones();
    packet2.seq_id = u10::new(285);
    packet2.frame_begin = false;
    packet2.frame_end = false;
    packet2.timestamp = 127384127;

    let mut packet3 = common::data_ones();
    packet3.seq_id = u10::new(286);
    packet3.frame_begin = false;
    packet3.frame_end = true;
    packet3.timestamp = 127384127;

    let mut accumulator = FrameAccumulator::new(packet1.timestamp);
    assert!(accumulator.add_packet(packet1.clone()).is_ok());
    assert!(accumulator.add_packet(packet2.clone()).is_ok());
    assert!(accumulator.add_packet(packet3.clone()).is_ok());
    let completed = accumulator.complete();
    assert!(completed.is_ok(), "complete() returned {:?}", completed);
    let completed = completed.unwrap();
    assert_eq!(completed[0..], [&packet1, &packet2, &packet3]);
}

#[test]
fn test_three_dgram_wrap_accumulator() {
    let mut packet1 = common::data_ones();
    packet1.seq_id = u10::new(1023);
    packet1.frame_begin = true;
    packet1.frame_end = false;
    packet1.timestamp = 127384127;

    let mut packet2 = common::data_ones();
    packet2.seq_id = u10::new(0);
    packet2.frame_begin = false;
    packet2.frame_end = false;
    packet2.timestamp = 127384127;

    let mut packet3 = common::data_ones();
    packet3.seq_id = u10::new(1);
    packet3.frame_begin = false;
    packet3.frame_end = true;
    packet3.timestamp = 127384127;

    let mut accumulator = FrameAccumulator::new(packet1.timestamp);
    assert!(accumulator.add_packet(packet1.clone()).is_ok());
    assert!(accumulator.add_packet(packet2.clone()).is_ok());
    assert!(accumulator.add_packet(packet3.clone()).is_ok());
    let completed = accumulator.complete();
    assert!(completed.is_ok(), "complete() returned {:?}", completed);
    let completed = completed.unwrap();
    assert_eq!(completed[0..], [&packet1, &packet2, &packet3]);
}

