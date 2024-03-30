// Copyright 2024 Dalton Durst and the drc-sim-rust contributors
// SPDX-License-Identifier: MPL-2.0

// This program records ten thousand packets to a file called
// video_packets in your current directory.

use drc_sim_rust_lib::sockets;

use std::{
    fs::File,
    io::{BufWriter, Write},
};

use log::info;

fn main() -> std::io::Result<()> {
    simple_logger::init_with_env().unwrap();
    {
        //TODO: Bind to the appropriate IP address (It's usually
        //192.168.1.11 but could be different)
        let video_socket = sockets::get_vid_socket("0.0.0.0")?;

        let mut file_writer = BufWriter::new(File::create_new("video_packets")?);

        for _n in 1..=10000 {
            // 2063 is the maximum theoretical size of the WUP video
            // packet
            let mut buf = [0u8; 2063];
            let (amt, src) = video_socket.recv_from(&mut buf)?;

            // The WUP video datagram has a 16 byte header and should
            // always have at least a single-byte payload.
            if amt < 17 {
                info!("Packet from {src} was too short at only {amt} bytes, skipping.");
                continue;
            }

            file_writer.write(&buf)?;
        }
        Ok(())
    }
}
