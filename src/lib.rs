#![no_std]

use heapless::{
    consts::*,
    spsc::{Queue, SingleCore},
    Vec,
};

pub enum Error {
    CannotStoreData,
    Other,
}

type Address = u8;
const BROADCAST_ADDRESS: Address = 0xff;

type BufSize = U512;
type FrameBufsize = U16;

pub struct DataFrame {
    addr: Address,
    data: Vec<u8, U256>,
}

pub struct Transport {
    address: Address,
    buf: Vec<u8, BufSize>,
    frame_queue: Queue<DataFrame, FrameBufsize, u8, SingleCore>,
    curr_frame: Cell<DataFrame>,
    active_message: bool,
}

impl Transport {
    pub fn new(address: Address) -> Self {
        Transport {
            address,
            buf: Vec::new(),
            frame_queue: unsafe { Queue::u8_sc() },
            active_message: false,
        }
    }

    pub fn message_ready(&self) -> bool {
        // self.parse_message();
        !self.frame_queue.is_empty()
    }

    fn has_address_match(&self, data: u16) -> bool {
        let is_address = (data & (1 << 9)) == 1;
        let shifted_data = data as u8;
        match is_address {
            true => {
                let mut has_address_match: bool = shifted_data & self.address == self.address;
                has_address_match |= shifted_data & BROADCAST_ADDRESS == BROADCAST_ADDRESS;
                has_address_match
            }
            _ => is_address,
        }
    }

    pub fn ingest(&mut self, data: u16) -> Result<(), Error> {
        // if we have an active message then store
        // data, we don't care if we receive information when it's not our turn.
        if self.has_address_match(data) {
            if !self.active_message {
                self.active_message = true;
                match self.buf.push(data as u8) {
                    Ok(_) => (),
                    Err(_) => return Err(Error::CannotStoreData),
                };
            } else {
                // We have an already active message but received a new address byte
                // so we're gonna drop all known data.
                // TODO: at some point, push this into a frame queue
                self.buf.clear()
            }
        }
        Ok(())
    }

    fn parse_buffer(&mut self) {
        if self.buf.len() > 2 {
            if self.buf.len() >= (2 + self.buf[1]) as usize {
                // Here we have a message
            }
        }
    }
}
