#![allow(unused)]

use std::sync::Arc;
use std::io::ErrorKind;
use std::io;
use tokio::net::TcpStream;
use tokio::prelude::*;
use tokio::io::{AsyncBufReadExt, BufReader};
use crate::util::*;
use crate::channel::Message;

#[derive(Clone)]
pub enum Action {
    SubStream,
    SubPacket,
    PubStream,
    PubPacket(Arc<Message>),
}

pub struct Command {
    act: Action,
    channel: usize,
}

impl Command {
    pub fn action(&self) -> &Action {
        &self.act
    }

    pub fn channel(&self) -> usize {
        self.channel
    }
}

const COMMAND_PARAM_NUM: usize = 2;

struct CommandBuilder {
    act: Option<Action>,
    channel: Option<usize>,
    cnt: usize,
}

impl CommandBuilder {
    fn new() -> CommandBuilder {
        CommandBuilder {
            act: None,
            channel: None,
            cnt: 0,
        }
    }
    fn set_act(&mut self, act: Action) {
        if let None = self.act {
            self.cnt += 1;
        }
        self.act = Some(act);
    }

    fn set_channel(&mut self, chan: usize) {
        if let None = self.channel {
            self.cnt += 1;
        }
        self.channel = Some(chan);
    }

    fn try_build(&mut self) -> Option<Command> {
        if self.cnt < COMMAND_PARAM_NUM {
            None
        } else if self.cnt == COMMAND_PARAM_NUM {
            Some(Command {
                act: self.act.take().unwrap(),
                channel: self.channel.take().unwrap(),
            })
        } else {
            unreachable!()
        }
    }
}


pub struct AsyncCommandParser<'a>(pub &'a mut BufReader<TcpStream>);

impl<'a> AsyncCommandParser<'a> {
    pub async fn parse_command(&mut self) -> io::Result<Command> {
        let mut builder = CommandBuilder::new();
        let ch = self.read_u8().await?;
        match ch {
            b'w' | b'W' => {
                builder.set_act(Action::PubStream);
            }
            b'r' | b'R' => {
                builder.set_act(Action::SubStream);
            }
            b'p' | b'P' => {
                return self.parse_packet_pub().await;
            }
            b's' | b'S' => {
                builder.set_act(Action::SubPacket);
            }
            _ => {
                return Err(io::Error::new(ErrorKind::Other, "protocol error"));
            }
        }

        let channel_no = self.read_usize_end_line().await?;
        builder.set_channel(channel_no);

        Ok(builder.try_build().unwrap())
    }

    async fn parse_packet_pub(&mut self) -> io::Result<Command> {
        let mut builder = CommandBuilder::new();
        let channel_no = self.read_usize_end_line().await?;

        builder.set_channel(channel_no);
        let msg_len = self.read_usize_end_line().await?;
        let mut buf: Vec<u8> = vec![0; msg_len];

        self.0.read_exact(&mut buf[..msg_len]).await?;
        let msg = Message::from_vec(buf);
        builder.set_act(Action::PubPacket(Arc::new(msg)));
        Ok(builder.try_build().unwrap())
    }

    async fn read_u8(&mut self) -> io::Result<u8> {
        let mut buf = [0u8; 1];
        self.0.read_exact(&mut buf).await?;
        Ok(buf[0])
    }

    async fn read_usize_end_line(&mut self) -> io::Result<usize> {
        let mut buf: Vec<u8> = Vec::with_capacity(3);

        self.0.read_until(b'\n', &mut buf).await?;
        if buf.len() <= 2 {
            return Err(io::Error::new(ErrorKind::Other, "protocol error"));
        }

        if buf[buf.len() - 1] == b'\n' {
            if buf[buf.len() - 2] != b'\r' {
                return Err(io::Error::new(ErrorKind::Other, "protocol error"));
            }
        }

        let size = bytes_to_usize(&buf[..buf.len() - 2]).map_err(|_| {
            io::Error::new(ErrorKind::Other, "protocol error")
        })?;
        Ok(size)
    }
}