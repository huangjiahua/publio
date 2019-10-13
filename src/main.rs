#[macro_use]
extern crate log;
extern crate env_logger;

use std::io;
use std::io::ErrorKind;
use std::error::Error;
use std::sync::Arc;
use tokio::prelude::*;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc::{Sender, Receiver, channel};
use tokio::sync::Mutex;
use tokio::io::BufReader;
use publio::protocol::{AsyncCommandParser, Action};

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

async fn serve_client(socket: TcpStream, channels: Arc<Vec<Arc<Mutex<Channel>>>>) {
    let mut socket = Some(socket);
    loop {
        let mut buf_reader = BufReader::new(socket.take().unwrap());
        let cmd;
        let mut parser = AsyncCommandParser(&mut buf_reader);
        cmd = match parser.parse_command().await {
            Ok(c) => c,
            Err(e) => {
                match e.kind() {
                    ErrorKind::Other => {
                        continue;
                    }
                    _ => {
                        debug!("parse command error: {}", e.description());
                        break;
                    }
                }
            }
        };
        let mut stream = buf_reader.into_inner();
        match cmd.action() {
            Action::Read => {
                let r = channels[cmd.channel()].lock().await.register();
                if let Err(e) = subscribe(&mut stream, r).await {
                    debug!("subscribe error: {}", e.description());
                }
            }
            Action::Write => {
                if let Err(e) = publish(&mut stream,
                                        channels[cmd.channel()].clone()).await {
                    debug!("publish error: {}", e.description());
                }
            }
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