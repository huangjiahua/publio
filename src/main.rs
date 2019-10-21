#[macro_use]
extern crate log;
extern crate env_logger;

use std::io;
use std::io::ErrorKind;
use std::error::Error;
use std::sync::Arc;
use tokio::prelude::*;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc::Receiver;
use tokio::sync::Mutex;
use tokio::io::BufReader;

use publio::protocol::{AsyncCommandParser, Action};
use publio::channel::*;

async fn subscribe(mut socket: TcpStream, mut r: Receiver<Arc<Message>>) -> io::Result<()> {
    while let Some(msg) = r.recv().await {
        socket.write_all(&msg.data[..]).await?;
    }
    Ok(())
}

async fn subscribe_packet(
    mut socket: TcpStream,
    mut r: Receiver<Arc<Message>>,
) -> io::Result<()> {
    while let Some(msg) = r.recv().await {
        let len_str = format!(":{}\r\n", msg.data.len());
        socket.write_all(len_str.as_bytes()).await?;
        socket.write_all(&msg.data[..]).await?;
    }
    Ok(())
}

async fn publish(mut socket: TcpStream, chan: Arc<Mutex<Channel>>) -> io::Result<()> {
    let mut buf = [0u8; 1024];
    loop {
        let n = socket.read(&mut buf).await?;
        if n == 0 {
            break;
        }
        let msg = Arc::new(Message::from_bytes(&buf[..n]));
        chan.lock().await.broadcast(msg.clone()).await?;
    }
    Ok(())
}

async fn publish_packet(packet: &Arc<Message>, chan: Arc<Mutex<Channel>>) -> io::Result<()> {
    chan.lock().await.broadcast(packet.clone()).await?;
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
                        socket = Some(buf_reader.into_inner());
                        continue;
                    }
                    _ => {
                        debug!("parse command error: {}", e.description());
                        break;
                    }
                }
            }
        };
        let stream = buf_reader.into_inner();
        match cmd.action() {
            Action::SubStream => {
                let r = channels[cmd.channel()].lock().await.register();
                if let Err(e) = subscribe(stream, r).await {
                    debug!("subscribe error: {}", e.description());
                }
                break;
            }
            Action::SubPacket => {
                let r = channels[cmd.channel()].lock().await.register();
                if let Err(e) = subscribe_packet(stream, r).await {
                    debug!("subscribe packet error: {}", e.description());
                }
                break;
            }
            Action::PubStream => {
                if let Err(e) = publish(stream,
                                        channels[cmd.channel()].clone()).await {
                    debug!("publish error: {}", e.description());
                }
                break;
            }
            Action::PubPacket(p) => {
                if let Err(e) = publish_packet(p,
                                               channels[cmd.channel()].clone()).await {
                    debug!("publish packet error: {}", e.description());
                }
            }
        }
        socket = Some(stream);
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