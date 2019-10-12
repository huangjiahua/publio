#[macro_use]
extern crate log;
extern crate env_logger;

use std::io;
use std::sync::Arc;
use tokio::prelude::*;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc::{Sender, Receiver, channel};
use tokio::sync::Mutex;

struct Channel {
    senders: Vec<Sender<Arc<Vec<u8>>>>,
}

impl Channel {
    fn new() -> Self {
        Channel {
            senders: vec![],
        }
    }

    fn register(&mut self) -> Receiver<Arc<Vec<u8>>> {
        let (sender, recevier) = channel(1000);
        self.senders.push(sender);
        recevier
    }

    async fn broadcast(&mut self, content: Arc<Vec<u8>>) -> io::Result<()> {
        for s in self.senders.iter_mut() {
            if let Err(_) = s.send(content.clone()).await {
                continue;
            }
        }
        Ok(())
    }
}

async fn subscribe(socket: &mut TcpStream, mut r: Receiver<Arc<Vec<u8>>>) -> io::Result<()> {
    while let Some(msg) = r.recv().await {
        socket.write_all(msg.as_ref()).await?;
    }
    Ok(())
}

async fn publish(socket: &mut TcpStream, chan: Arc<Mutex<Channel>>) -> io::Result<()> {
    let mut buf = [0u8; 1024];
    loop {
        let n = socket.read(&mut buf).await?;
        if n == 0 {
            break;
        }
        let msg = Arc::new(buf[0..n].to_vec());
        chan.lock().await.broadcast(msg.clone()).await?;
    }
    Ok(())
}

async fn serve_client(mut socket: TcpStream, channels: Arc<Vec<Arc<Mutex<Channel>>>>) {
    let mut buf = [0u8; 1024];
    while let Ok(n) = socket.read(&mut buf).await {
        if n < 2 {
            break;
        }
        let (rw, num) = buf[0..n].split_at(1);
        assert_eq!(rw.len(), 1);

        let chan_num = (num[0] - b'0') as usize;

        match rw[0] {
            b'r' => {
                info!("get subscriber");
                let r = channels[chan_num].lock().await.register();
                subscribe(&mut socket, r).await.unwrap();
            }
            b'w' => {
                info!("get publisher");
                publish(&mut socket, channels[chan_num].clone()).await.unwrap();
            }
            _ => {}
        }
        break;
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    let mut listener = TcpListener::bind("127.0.0.1:9090").await?;
    info!("running");
    let mut vec = vec![];
    for _ in 0..10 {
        vec.push(Arc::new(Mutex::new(Channel::new())));
    }
    let channels = Arc::new(vec);

    while let Ok((socket, _)) = listener.accept().await {
        tokio::spawn(serve_client(socket, channels.clone()));
    }
    Ok(())
}