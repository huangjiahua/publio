use std::io;
use std::sync::Arc;
use tokio::sync::mpsc::{Sender, Receiver, channel};

pub struct Message {
    pub data: Vec<u8>,
    packet_len: usize,
    packet_off: usize,
}

impl Message {
    pub fn from_bytes(bytes: &[u8]) -> Message {
        Message {
            data: bytes.to_vec(),
            packet_len: 0,
            packet_off: 0,
        }
    }

    pub fn from_bytes_and_packet_info(bytes: &[u8], len: usize, off: usize) -> Message {
        Message {
            data: bytes.to_vec(),
            packet_len: len,
            packet_off: off,
        }
    }

    pub fn packet_len(&self) -> Option<usize> {
        if self.packet_len == 0 {
            None
        } else {
            Some(self.packet_len)
        }
    }

    pub fn packet_off(&self) -> Option<usize> {
        if self.packet_len == 0 {
            None
        } else {
            Some(self.packet_off)
        }
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }
}

pub struct Channel {
    senders: Vec<Sender<Arc<Message>>>,
}

impl Channel {
    pub fn new() -> Self {
        Channel {
            senders: vec![],
        }
    }

    pub fn register(&mut self) -> Receiver<Arc<Message>> {
        let (sender, recevier) = channel(1000);
        self.senders.push(sender);
        recevier
    }

    pub async fn broadcast(&mut self, content: Arc<Message>) -> io::Result<()> {
        for s in self.senders.iter_mut() {
            if let Err(_) = s.send(content.clone()).await {
                continue;
            }
        }
        Ok(())
    }
}

//pub struct Repeater {}
