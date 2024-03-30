// Copyright 2024 Dalton Durst and the drc-sim-rust contributors
// SPDX-License-Identifier: MPL-2.0

use std::net::UdpSocket;

// const PORT_WII_MSG: u16 = 50010;
const PORT_WUP_VID: u16 = 50120;
const PORT_WUP_AUD: u16 = 50121;
// const PORT_WII_HID: u16 = 50122;
// const PORT_WII_CMD: u16 = 50123;

fn get_socket(dest_ip: &str, port: u16) -> Result<UdpSocket, std::io::Error> {
    let addr = format!("{}:{}", dest_ip, port);
    return UdpSocket::bind(addr);
}

pub fn get_vid_socket(dest_ip: &str) -> Result<UdpSocket, std::io::Error> {
    return get_socket(dest_ip, PORT_WUP_VID);
}

pub fn get_aud_socket(dest_ip: &str) -> Result<UdpSocket, std::io::Error> {
    return get_socket(dest_ip, PORT_WUP_AUD);
}
