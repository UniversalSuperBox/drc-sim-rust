// Copyright 2024 Dalton Durst and the drc-sim-rust contributors
// SPDX-License-Identifier: MPL-2.0

use std::{
    cmp::Ordering,
    collections::HashMap,
    fs::File,
    io::{BufReader, Read},
};

use drc_sim_rust_lib::{
    incoming_packet_parser::{self, u32_paws_compare},
    packet_organizer::{self, FrameAccumulator, IncompleteReason},
    STALE_FRAME_THRESHOLD, WUP_VID_PACKET_BUFFER_SIZE,
};
use log::{debug, error, info, trace};

fn main() -> std::io::Result<()> {
    simple_logger::init_with_env().unwrap();
    {
        let mut file_reader = BufReader::new(File::open("video_packets")?);

        let mut i = 0;
        let mut completed_frames = 0;
        let mut dropped_frames = 0;

        // The newest timestamp that we have seen so far.
        let mut high_water_mark: Option<u32> = None;

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

            let packet = match incoming_packet_parser::process_video_packet(&buf) {
                None => {
                    error!("Didn't get a packet back from process");
                    continue;
                }
                Some(val) => val,
            };

            trace!("Packet {i}: {packet:?}");

            let timestamp = packet.timestamp;

            if high_water_mark == None {
                high_water_mark = Some(timestamp);
            }

            if u32_paws_compare(timestamp, high_water_mark.unwrap()) == Some(Ordering::Greater) {
                debug!("New high water mark is {}", timestamp);
                high_water_mark = Some(timestamp);
            }

            let mut accumulators_to_remove: Vec<u32> = Vec::new();
            if frame_accumulators.len() > 1 {
                for accu_timestamp in frame_accumulators.keys().cloned() {
                    // This is "timestamp is less than our lowest acceptable timestamp"
                    if u32_paws_compare(
                        accu_timestamp,
                        high_water_mark.unwrap() - STALE_FRAME_THRESHOLD,
                    ) == Some(Ordering::Less)
                    {
                        accumulators_to_remove.push(accu_timestamp);
                    }
                }
            }
            // I tried to avoid this separate loop by cloning the
            // HashMap's keys and performing the removal inside the
            // above loop, but the compiler always complained that I was
            // trying to mutate the HashMap after requesting an
            // immutable borrow of it at the declaration of the loop.
            for to_remove in accumulators_to_remove {
                dropped_frames += 1;
                frame_accumulators.remove(&to_remove);
            }

            let frame_accumulator = frame_accumulators
                .entry(timestamp)
                .or_insert(FrameAccumulator::new(timestamp));

            let _ = frame_accumulator.add_packet(packet);

            let frame_dgrams = match frame_accumulator.complete() {
                Ok(data) => {
                    completed_frames += 1;
                    data
                }
                Err(IncompleteReason::TooManyPackets) => {
                    error!("Frame accumulator has too many packets, dropping.");
                    frame_accumulators.remove(&timestamp);
                    continue;
                }
                // All other reasons will be handled by us dropping old
                // accumulators.
                _ => continue,
            };
            info!("Processed frame {:?}", frame_accumulator.timestamp());
            debug!("{:?}", frame_dgrams);

            frame_accumulators.remove(&timestamp);
        }
        info!(
            "{:?} frames were incomplete at time of exiting, completed {} dropped {}.",
            frame_accumulators.len(),
            completed_frames,
            dropped_frames,
        );
    }
    Ok(())
}
