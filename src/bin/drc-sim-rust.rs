// Copyright 2024 Dalton Durst and the drc-sim-rust contributors
// SPDX-License-Identifier: MPL-2.0

use drc_sim_rust_lib::incoming_packet_parser;
use drc_sim_rust_lib::sockets;

use drc_sim_rust_lib::WUP_VID_PACKET_BUFFER_SIZE;
use log::{error, trace};

fn main() -> std::io::Result<()> {
    simple_logger::init_with_env().unwrap();
    {
        //TODO: Bind to the appropriate IP address (It's usually
        //192.168.1.11 but could be different)
        let video_socket = sockets::get_vid_socket("0.0.0.0")?;

        loop {
            let mut buf = [0u8; WUP_VID_PACKET_BUFFER_SIZE];
            video_socket.recv_from(&mut buf)?;

            let packet = match incoming_packet_parser::process_video_packet(&buf) {
                None => {
                    error!("Didn't get a packet back from process");
                    continue;
                }
                Some(val) => val,
            };

            trace!("{packet:?}");
        }
        Ok(())
    }
}
