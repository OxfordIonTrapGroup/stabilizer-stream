use anyhow::Result;
use clap::Parser;
use std::io::ErrorKind;
use std::time::Duration;
use std::{
    fs::File,
    io::{BufReader, Read, Seek},
};

/// Stabilizer stream source options
#[derive(Parser, Debug, Clone)]
pub struct SourceOpts {
    /// The local IP to receive streaming data on.
    #[arg(short, long, default_value = "0.0.0.0")]
    ip: std::net::Ipv4Addr,

    /// The UDP port to receive streaming data on.
    #[arg(short, long, default_value_t = 9293)]
    port: u16,

    /// Use frames from the given file
    #[arg(short, long)]
    file: Option<String>,

    /// Frame size in file (8 + n_batches*n_channel*batch_size)
    #[arg(short, long, default_value_t = 1400)]
    frame_size: usize,
}

#[derive(Debug)]
pub enum Source {
    Udp(std::net::UdpSocket),
    File(BufReader<File>, usize),
}

impl Source {
    pub fn new(opts: &SourceOpts) -> Result<Self> {
        Ok(if let Some(file) = &opts.file {
            Self::File(
                BufReader::with_capacity(1 << 20, File::open(file)?),
                opts.frame_size,
            )
        } else {
            log::info!("Binding to {}:{}", opts.ip, opts.port);
            let socket = std::net::UdpSocket::bind((opts.ip, opts.port))?;
            socket2::SockRef::from(&socket).set_recv_buffer_size(1 << 20)?;
            socket.set_read_timeout(Some(Duration::from_millis(1000)))?;
            Self::Udp(socket)
        })
    }

    pub fn get(&mut self, buf: &mut [u8]) -> Result<usize> {
        Ok(match self {
            Self::File(fil, n) => loop {
                match fil.read_exact(&mut buf[..*n]) {
                    Ok(()) => {
                        break *n;
                    }
                    Err(e) if e.kind() == ErrorKind::UnexpectedEof => {
                        fil.seek(std::io::SeekFrom::Start(0))?;
                    }
                    Err(e) => Err(e)?,
                }
            },
            Self::Udp(socket) => socket.recv(buf)?,
        })
    }
}
