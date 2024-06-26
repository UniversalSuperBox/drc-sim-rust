use core::fmt;
use std::{cmp::Ordering, collections::HashMap};

use arbitrary_int::{u10, Number};

use crate::incoming_packet_parser::WUPVideoPacket;

pub struct FrameAccumulator {
    timestamp_: u32,
    packets: HashMap<u10, WUPVideoPacket>,
    begin_packet_: Option<u10>,
    end_packet_: Option<u10>,
}

#[derive(Debug)]
pub enum PacketRejectReason {
    /// The given packet indicates it does not have a timestamp.
    NoTimestamp,
    /// The given packet's timestamp does not match this
    /// FrameAccumulator.
    WrongTimestamp,
    /// This FrameAccumulator already has a packet with 'frame_begin'.
    AlreadyHaveBegin,
    /// This FrameAccumulator already has a packet with 'frame_end'.
    AlreadyHaveEnd,
    /// This FrameAccumulator already has a packet with that sequence
    /// ID.
    AlreadyHaveSeq,
}

pub struct IncompleteReason {
    pub kind: IncompleteReasonKind,
    pub text: String,
}

impl IncompleteReason {
    fn new(kind: IncompleteReasonKind, text: String) -> IncompleteReason {
        return IncompleteReason { kind, text };
    }
    fn format_error(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}: {}", self.kind, self.text)
    }
}

impl fmt::Display for IncompleteReason {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        return self.format_error(f);
    }
}

impl fmt::Debug for IncompleteReason {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        return self.format_error(f);
    }
}

#[derive(Debug)]
pub enum IncompleteReasonKind {
    /// This FrameAccumulator does not have the begin or end packet.
    NoBeginEndPacket,
    /// This FrameAccumulator hs the end packet, but not the begin
    /// packet.
    NoBeginPacket,
    /// This FrameAccumulator has the begin packet, but not the end
    /// packet.
    NoEndPacket,
    /// This FrameAccumulator has the begin and end packet, but not
    /// enough packets in between.
    TooFewPackets,
    /// This FrameAccumulator has more packets than it expected.
    TooManyPackets,
    /// This FrameAccumulator has the begin and end packet, but one or
    /// more of the seq_ids between the begin and end packet is missing
    /// (meaning we've received a packet outside the range)
    Corrupt,
}

impl FrameAccumulator {
    pub fn new(timestamp: u32) -> FrameAccumulator {
        return FrameAccumulator {
            timestamp_: timestamp,
            packets: HashMap::new(),
            begin_packet_: None,
            end_packet_: None,
        };
    }

    pub fn timestamp(&self) -> &u32 {
        return &self.timestamp_;
    }

    pub fn add_packet(&mut self, packet: WUPVideoPacket) -> Result<(), PacketRejectReason> {
        if !packet.has_timestamp {
            return Err(PacketRejectReason::NoTimestamp);
        }
        if !packet.timestamp == self.timestamp_ {
            return Err(PacketRejectReason::WrongTimestamp);
        }
        let incoming_seq_id = packet.seq_id.clone();
        if packet.frame_begin {
            if self.begin_packet_ != None {
                return Err(PacketRejectReason::AlreadyHaveBegin);
            }
            self.begin_packet_ = Some(incoming_seq_id);
        }
        if packet.frame_end {
            if self.end_packet_ != None {
                return Err(PacketRejectReason::AlreadyHaveEnd);
            }
            self.end_packet_ = Some(incoming_seq_id);
        }
        // This could be replaced with self.packets.try_insert if that
        // ever makes it into Rust.
        // https://github.com/rust-lang/rust/issues/82766
        if self.packets.contains_key(&incoming_seq_id) {
            return Err(PacketRejectReason::AlreadyHaveSeq);
        }
        let existing = self.packets.insert(incoming_seq_id, packet);
        if existing != None {
            panic!(
                "Clobbered a packet in FrameAccumulator with timestamp {} seq_id {}",
                self.timestamp_, incoming_seq_id
            );
        }
        return Ok(());
    }

    pub fn complete(&self) -> Result<Vec<&WUPVideoPacket>, IncompleteReason> {
        if self.begin_packet_ == None && self.end_packet_ == None {
            return Err(IncompleteReason::new(
                IncompleteReasonKind::NoBeginEndPacket,
                "have neither begin nor end packet.".to_string(),
            ));
        } else if self.begin_packet_ == None {
            return Err(IncompleteReason::new(
                IncompleteReasonKind::NoBeginPacket,
                "have end packet but not begin packet.".to_string(),
            ));
        } else if self.end_packet_ == None {
            return Err(IncompleteReason::new(
                IncompleteReasonKind::NoEndPacket,
                "have begin packet but not end packet.".to_string(),
            ));
        }

        let begin_packet: u16 = self.begin_packet_.unwrap().into();
        let end_packet: u16 = self.end_packet_.unwrap().into();
        let end_packet_absolute: u16 = match end_packet.cmp(&begin_packet) {
            Ordering::Less => end_packet + u16::from(u10::MAX) + 1,
            _ => end_packet,
        };

        let expected_num_packets = (end_packet_absolute + 1) - begin_packet;

        let have_packets = self.packets.len();
        match have_packets.cmp(&expected_num_packets.into()) {
            std::cmp::Ordering::Equal => (),
            comparison => {
                let reason = format!("Have {} dgrams want {}", have_packets, expected_num_packets);
                if comparison == Ordering::Less {
                    return Err(IncompleteReason::new(
                        IncompleteReasonKind::TooFewPackets,
                        reason,
                    ));
                }
                return Err(IncompleteReason::new(
                    IncompleteReasonKind::TooManyPackets,
                    reason,
                ));
            }
        }

        // We have the correct number of packets, but do we have the
        // correct packets within that stride?
        let mut sorted_packets: Vec<&WUPVideoPacket> = Vec::new();
        for i in begin_packet..=end_packet_absolute {
            let wrapped_i = u10::new(i % 1024);
            let packet = match self.packets.get(&wrapped_i) {
                None => {
                    return Err(IncompleteReason::new(
                        IncompleteReasonKind::Corrupt,
                        format!(
                            "FrameAccumulator has correct number of packets but is missing at least packet {} (searching {}..={})",
                            wrapped_i,
                            begin_packet,
                            end_packet_absolute
                        )
                    )
                );
                }
                Some(p) => p,
            };
            sorted_packets.push(packet);
        }

        return Ok(sorted_packets);
    }
}
