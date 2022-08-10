use crate::Object;
use std::{
    fs::File,
    io::{self, Stdin, Stdout, Write},
};

pub enum Port {
    Stdin(Stdin),
    Stdout(Stdout),
    File(File),
}

impl Clone for Port {
    fn clone(&self) -> Self {
        match self {
            Self::Stdin(_) => Port::Stdin(std::io::stdin()),
            Self::Stdout(_) => Port::Stdout(std::io::stdout()),
            Self::File(f) => Port::File(f.try_clone().expect("couldn't clone fd")),
        }
    }
}

impl Write for Port {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        match self {
            Port::Stdin(_) => Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "can't write to stdin",
            )),
            Port::Stdout(s) => s.write(buf),
            Port::File(f) => f.write(buf),
        }
    }
    fn flush(&mut self) -> std::io::Result<()> {
        match self {
            Port::Stdin(_) => Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "can't flush stdin",
            )),
            Port::Stdout(s) => s.flush(),
            Port::File(f) => f.flush(),
        }
    }
}

pub fn current_output_port() -> Object {
    Object::Port(Port::Stdout(std::io::stdout()))
}
