// Copyright 2024 Dalton Durst and the drc-sim-rust contributors
// SPDX-License-Identifier: MPL-2.0

// This program records ten thousand packets to a file called
// video_packets in your current directory.

use drc_sim_rust_lib::{sockets, WUP_VID_PACKET_BUFFER_SIZE};

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

        for n in 0..10000 {
            let mut buf = [0u8; WUP_VID_PACKET_BUFFER_SIZE];
            video_socket.recv_from(&mut buf)?;

            let written = file_writer.write(&buf)?;
            assert!(written == WUP_VID_PACKET_BUFFER_SIZE);
            info!("{}", n);
        }
        Ok(())
    }
}
