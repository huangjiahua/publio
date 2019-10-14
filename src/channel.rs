use std::io;
use std::sync::Arc;
use tokio::sync::mpsc::{Sender, Receiver, channel};
use std::collections::HashMap;

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

struct Launcher {
    sender: Sender<Arc<Message>>,
}

pub struct Channel {
    senders: HashMap<u64, Launcher>,
    msg_id: u64,
    cli_id: u64,
}

impl Channel {
    pub fn new() -> Self {
        Channel {
            senders: HashMap::new(),
            msg_id: 0,
            cli_id: 0,
        }
    }

    pub fn register(&mut self) -> Receiver<Arc<Message>> {
        let (sender, recevier) = channel(1000);
        let launcher = Launcher {
            sender,
        };
        let cli_id = self.cli_id;
        self.cli_id += 1;
        let r = self.senders.insert(cli_id, launcher).is_none();
        assert!(r);
        recevier
    }

    // protected by mutex
    pub async fn broadcast(&mut self, content: Arc<Message>) -> io::Result<u64> {
        let ret = self.msg_id;
        self.msg_id += 1;
        let mut freed = vec![];

        for (id, l) in self.senders.iter_mut() {
            if let Err(_) = l.sender.send(content.clone()).await {
                freed.push(*id);
                continue;
            }
        }

        for id in freed {
            debug!("subscriber closed");
            self.senders.remove(&id).unwrap();
        }

        Ok(ret)
    }
}

//pub struct Repeater {}
