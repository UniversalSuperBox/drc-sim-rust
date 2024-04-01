pub mod incoming_packet_parser;
pub mod sockets;

// 2063 is the maximum theoretical size of the WUP video packet. I've
// never seen a packet larger than 1688, but allocating the full 2048
// doesn't hurt _much_
pub const WUP_VID_PACKET_BUFFER_SIZE: usize = 2048;
