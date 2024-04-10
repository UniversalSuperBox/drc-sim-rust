// Copyright 2024 Dalton Durst and the drc-sim-rust contributors
// SPDX-License-Identifier: MPL-2.0

use std::{
    collections::HashMap,
    fs::File,
    io::{BufReader, Read},
};

use drc_sim_rust_lib::{
    incoming_packet_parser::{self, timestamp_compare},
    packet_organizer::{self, FrameAccumulator},
    WUP_VID_PACKET_BUFFER_SIZE,
};
use log::{debug, error, info, trace};

fn main() -> std::io::Result<()> {
    simple_logger::init_with_env().unwrap();
    {
        let mut file_reader = BufReader::new(File::open("video_packets")?);

        let mut i = 0;
        let mut completed_frames = 0;
        let mut dropped_frames = 0;
        let mut most_queued_frames = 0;

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

            trace!("Packet {i}: {packet:?}");

            let timestamp = packet.timestamp;
            // This implementation holds the current frame and up to two
            // additional older frames under normal conditions. If
            // conditions got really bad, it would allocate and promptly
            // drop a very large number of frame accumulators.
            // Additionally, it doesn't make much sense to hold frames
            // from multiple seconds ago. It is very unlikely they will
            // be completed. It may be better to calculate a window of
            // acceptable values around the newest known frame, dropping
            // all dgrams which fall outside the acceptable window to
            // avoid that churn.
            let num_accumulators = frame_accumulators.len();
            if num_accumulators > most_queued_frames {
                most_queued_frames = num_accumulators;
            }
            if num_accumulators > 2 {
                let mut in_flight_accumulators: Vec<u32> =
                    Vec::from_iter(frame_accumulators.keys().cloned());
                in_flight_accumulators.sort_by(|a: &u32, b: &u32| timestamp_compare(*a, *b));
                debug!("{:?}", in_flight_accumulators);
                // also this might be an off-by-one error.
                for _ in 0..(num_accumulators - 2) {
                    let to_remove = match in_flight_accumulators.pop() {
                        None => {
                            error!(
                                "Tried to remove a FrameAccumulator, but there are none to remove"
                            );
                            continue;
                        }
                        Some(a) => a,
                    };
                    if to_remove == timestamp {
                        continue;
                    }
                    info!("Dropping frame {}", to_remove);
                    dropped_frames += 1;
                    frame_accumulators.remove(&to_remove);
                }
            }

            let frame_accumulator = frame_accumulators
                .entry(timestamp)
                .or_insert(FrameAccumulator::new(timestamp));

            frame_accumulator.add_packet(packet);

            let frame_dgrams = match frame_accumulator.complete() {
                Some(data) => {
                    completed_frames += 1;
                    data
                }
                None => {
                    continue;
                }
            };
            info!("Processed frame {:?}", frame_accumulator.timestamp);
            debug!("{:?}", frame_dgrams);

            frame_accumulators.remove(&timestamp);
        }
        info!(
            "{:?} frames were incomplete at time of exiting, completed {} dropped {}, had at most {} queued.",
            frame_accumulators.len(),
            completed_frames,
            dropped_frames,
            most_queued_frames,
        );
    }
    Ok(())
}
