// Copyright 2024 Dalton Durst and the drc-sim-rust contributors
// SPDX-License-Identifier: MPL-2.0

use std::{
    collections::HashMap,
    fs::File,
    io::{BufReader, Read},
};

use drc_sim_rust_lib::{
    incoming_packet_parser,
    packet_organizer::{self, FrameAccumulator},
    WUP_VID_PACKET_BUFFER_SIZE,
};
use log::{debug, error, trace};

fn main() -> std::io::Result<()> {
    simple_logger::init_with_env().unwrap();
    {
        let mut file_reader = BufReader::new(File::open("video_packets")?);

        let mut i = 0;
        let mut dgrams_since_frame_begin = 0;
        let mut payload_bytes: u32 = 0;
        let mut largest_payload: u32 = 0;

        let mut frame_accumulators: HashMap<u32, packet_organizer::FrameAccumulator> =
            HashMap::new();
        loop {
            i += 1;
            let mut buf = [0u8; WUP_VID_PACKET_BUFFER_SIZE];
            let len = file_reader.read(&mut buf);
            match len {
                Ok(len) => {
                    if len == 0 {
                        break;
                    }
                    assert!(
                        len == WUP_VID_PACKET_BUFFER_SIZE,
                        "Read length was only {len}"
                    )
                }
                Err(err) => {
                    return std::io::Result::Err(err);
                }
            }

            let packet = match incoming_packet_parser::process_video_packet(buf) {
                None => {
                    error!("Didn't get a packet back from process");
                    continue;
                }
                Some(val) => val,
            };

            if packet.frame_begin {
                debug!("Begin! {} {}", packet.seq_id, packet.timestamp);
                dgrams_since_frame_begin = 0;
                payload_bytes = 0;
            }
            if packet.frame_end {
                debug!(
                    "End! {} {} {} {}",
                    packet.seq_id, packet.timestamp, dgrams_since_frame_begin, payload_bytes
                );
                if payload_bytes > largest_payload {
                    largest_payload = payload_bytes;
                }
            }

            dgrams_since_frame_begin += 1;
            payload_bytes += packet.payload_size as u32;

            trace!("Packet {i}: {packet:?}");

            let frame_accumulator = frame_accumulators
                .entry(packet.timestamp)
                .or_insert(FrameAccumulator::new(packet.timestamp));

            frame_accumulator.add_packet(packet);

            debug!("{:?}", frame_accumulator.complete())
        }
        debug!("Largest payload we saw was {}", largest_payload);
    }
    Ok(())
}
