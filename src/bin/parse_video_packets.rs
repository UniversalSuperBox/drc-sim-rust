// Copyright 2024 Dalton Durst and the drc-sim-rust contributors
// SPDX-License-Identifier: MPL-2.0

use std::{
    fs::File,
    io::{BufReader, Read},
};

use drc_sim_rust_lib::{incoming_packet_parser, WUP_VID_PACKET_BUFFER_SIZE};
use log::{debug, error, trace};

fn main() -> std::io::Result<()> {
    simple_logger::init_with_env().unwrap();
    {
        let mut file_reader = BufReader::new(File::open("video_packets")?);

        let mut i = 0;
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
        }
    }
    Ok(())
}
