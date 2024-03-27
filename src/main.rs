use std::net::UdpSocket;
use binary_layout::prelude::*;

// const PORT_WII_MSG: u16 = 50010;
const PORT_WII_VID: u16 = 50120;
// const PORT_WII_AUD: u16 = 50121;
// const PORT_WII_HID: u16 = 50122;
// const PORT_WII_CMD: u16 = 50123;

// const WII_VIDEO_WIDTH: u16 = 848;
// const WII_VIDEO_HEIGHT: u16 = 480;

binary_layout!(video_packet, BigEndian, {
    // magic: [u8; 4],
    // packet_type: [u8; 2],
    // seq_id: [u8; 10],
    header_1: u16,
    // init: bool,
    // frame_begin: bool,
    // chunk_end: bool,
    // frame_end: bool,
    // has_timestamp: bool,
    // payload_size: 11 bit unsigned,
    header_2: [u8; 2],
    timestamp: u32,
    extended_header: u8,
    payload: [u8],
});

fn main() -> std::io::Result<()>{
    {
        //TODO: Bind to the appropriate IP address (It's usually 192.168.1.11 but could be different)
        let bindaddr = format!("0.0.0.0:{}", PORT_WII_VID);

        let video_socket = UdpSocket::bind(bindaddr)?;

        loop {

            let mut buf = [0; 2048];
            let (amt, src) = video_socket.recv_from(&mut buf)?;

            println!("Got {} bytes from peer {}", amt, src);

            let view = video_packet::View::new(&mut buf);

            let header = view.header_1().read();

            let magic = ((header & 0xF000) >> 12) as u8;
            let packet_type = ((header & 0x0C00) >> 10) as u8;
            let seq_id: u16 = header & 0x03FF;

            println!("Magic: {}, Type: {}, Seq: {}", magic, packet_type, seq_id)

            // magic: [u8; 4],
            // packet_type: [u8; 2],
            // seq_id: [u8; 10],

        }

        // println!("{:x?}", buf)
    }

    Ok(())
}
