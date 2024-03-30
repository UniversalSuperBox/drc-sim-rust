// Copyright 2024 Dalton Durst and the drc-sim-rust contributors
// SPDX-License-Identifier: MPL-2.0

use std::{
    fs::File,
    io::{BufReader, Read},
};

use drc_sim_rust_lib::incoming_packet_parser;
use log::{error, trace};

fn main() -> std::io::Result<()> {
    simple_logger::init_with_env().unwrap();
    {
        let mut file_reader = BufReader::new(File::open("video_packets")?);

        for _n in 1..=1000 {
            // record_video_packets saves all packets as 2048 bytes.
            let mut buf = [0u8; 2048];
            file_reader.read(&mut buf)?;

            let packet = match incoming_packet_parser::process_video_packet(&buf) {
                None => {
                    error!("Didn't get a packet back from process");
                    continue;
                }
                Some(val) => val,
            };

            trace!("{packet:?}");
        }
    }
    Ok(())
}
