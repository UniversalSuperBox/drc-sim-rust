pub mod incoming_packet_parser;
pub mod packet_organizer;
pub mod sockets;

/// The largest dgram that we expect to receive from the WUP.
/// 2063 is the maximum theoretical size of the WUP video packet. I've
/// never seen a packet larger than 1688, but allocating the full 2048
/// doesn't hurt _much_
pub const WUP_VID_PACKET_BUFFER_SIZE: usize = 2048;

/// The amount of time, according to dgram timestamps, after which a
/// frame is considered no longer completeable.
pub const STALE_FRAME_THRESHOLD: u32 = 16683 * 5; // 5 frames at ~16ms per frame
