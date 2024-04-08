use std::{cmp::Reverse, collections::BinaryHeap};

use log::{error, trace};

use crate::incoming_packet_parser::{process_video_packet, WUPVideoPacket};
use std::cmp::Ordering;

struct SortablePacket<'a> {
    sortable_seq_id: u32,
    data: &'a WUPVideoPacket,
}

pub struct FrameAccumulator {
    pub timestamp: u32,
    pub packets: Vec<WUPVideoPacket>,
}

impl FrameAccumulator {
    pub fn new(timestamp: u32) -> FrameAccumulator {
        return FrameAccumulator {
            timestamp: timestamp,
            packets: Vec::new(),
        };
    }

    pub fn add_packet(&mut self, packet: WUPVideoPacket) {
        assert!(packet.has_timestamp);
        assert!(packet.timestamp == self.timestamp);
        self.packets.push(packet);
    }

    pub fn complete(&self) -> Option<Vec<&WUPVideoPacket>> {
        // First, we need to figure out whether we have the start and
        // end dgrams
        let mut begin_seq_id: Option<u16> = None;
        let mut end_seq_id: Option<u16> = None;
        for packet in &self.packets {
            if packet.frame_begin {
                begin_seq_id = Some(packet.seq_id);
            } else if packet.frame_end {
                end_seq_id = Some(packet.seq_id);
            }
        }
        if begin_seq_id.is_none() {
            trace!("Don't have a start dgram");
            return None;
        }
        if end_seq_id.is_none() {
            trace!("Don't have an end dgram");
            return None;
        }

        let mut virt_end_seq_id = end_seq_id.unwrap();
        if end_seq_id < begin_seq_id {
            virt_end_seq_id = end_seq_id.unwrap() + begin_seq_id.unwrap()
        }
        let expected_num_dgrams = (begin_seq_id.unwrap()..virt_end_seq_id).len() + 1;

        let have_packets = self.packets.len();

        if have_packets != expected_num_dgrams {
            trace!("Have {} dgrams want {}", have_packets, expected_num_dgrams);
            return None;
        }

        // We have all of the dgrams we need, now we need to sort and
        // output them.
        let mut sorted_packets: Vec<SortablePacket> = Vec::new();
        for packet in &self.packets {
            let virt_seq_id = match packet.seq_id.cmp(&begin_seq_id.unwrap()) {
                Ordering::Less => packet.seq_id as u32 + begin_seq_id.unwrap() as u32,
                Ordering::Greater => packet.seq_id.into(),
                Ordering::Equal => packet.seq_id.into(),
            };
            let sortable_packet = SortablePacket {
                sortable_seq_id: virt_seq_id,
                data: packet,
            };
            sorted_packets.push(sortable_packet);
        }
        sorted_packets.sort_by_key(|packet| packet.sortable_seq_id);

        let mut returned_packets: Vec<&WUPVideoPacket> = Vec::new();
        for packet in sorted_packets {
            returned_packets.push(packet.data);
        }

        return Some(returned_packets);
    }
}
