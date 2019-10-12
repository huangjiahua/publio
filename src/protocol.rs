#![allow(unused)]

use tokio::net::TcpStream;
use tokio::prelude::*;
use std::io;
use tokio::io::ErrorKind;

pub enum Action {
    Read,
    Write,
}

pub struct Command {
    act: Action,
    channel: usize,
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

    fn try_build(&self) -> Option<Command> {
        if self.cnt < COMMAND_PARAM_NUM {
            None
        } else if self.cnt == COMMAND_PARAM_NUM {
            Some(Command {
                act: self.act.unwrap(),
                channel: self.channel.unwrap(),
            })
        } else {
            unreachable!()
        }
    }
}


pub struct AsyncCommandParser<'a>(&'a mut TcpStream);

impl<'a> AsyncCommandParser<'a> {
    pub async fn parse_command(&mut self) -> io::Result<Command> {
        let stream = self.0;
        let mut buf = [0u8; 1024];
        let mut builder = CommandBuilder::new();


        unimplemented!()
    }

    async fn read_u8(&mut self) -> io::Result<u8> {
        let mut buf = [0u8; 1];
        self.0.read_exact(&mut buf).await?;
        Ok(buf[0])
    }

    async fn read_usize_end_line(&mut self) -> io::Result<usize> {
        let mut buf = [0u8; 20];
        let mut total: usize = 0;
        let mut n_read: usize = 0;

        loop {
            let buf_ref = &mut buf[total..];
            n_read = self.0.read(&mut buf).await?;
            if n_read == 0 {
                return Err(io::Error::new(ErrorKind::BrokenPipe, "client closed"));
            }
            total += n_read;

            if buf_ref[n_read - 1] == b'\n' {
                if n_read == 1 || buf_ref[n_read - 2] != b'\r' {
                    return Err(io::Error::new(ErrorKind::Other, "protocol error"));
                } else {
                    total -= 2;
                    break;
                }
            }
            // TODO
        }


        Ok(1)
    }

}