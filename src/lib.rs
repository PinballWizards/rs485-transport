#![cfg_attr(not(test), no_std)]

use heapless::{consts::*, Vec};

use crc::crc16;

pub mod parser;

type Address = u8;

pub const BROADCAST_ADDRESS: Address = 0xff;

#[derive(Debug)]
pub struct DataFrame {
    address: Address,
    data: Vec<u8, U256>,
    crc: u16,
}

impl DataFrame {
    pub fn crc_valid(&self) -> bool {
        crc16::checksum_usb(&self.data) == self.crc
    }

    pub fn is_broadcast(&self) -> bool {
        self.address == BROADCAST_ADDRESS
    }
}

pub struct Response;

pub struct Transport {
    address: Address,
    data_buf: Vec<u8, U512>,
    frame_buf: Vec<DataFrame, U8>,
}

impl Transport {
    pub fn new(address: Address) -> Self {
        Transport {
            address,
            data_buf: Vec::new(),
            frame_buf: Vec::new(),
        }
    }

    fn is_address_byte(&self, byte: u16) -> bool {
        byte & (1 << 9) == 1
    }

    fn address_match(&self, address: Address) -> bool {
        (self.address & address == self.address)
            || (BROADCAST_ADDRESS & address == BROADCAST_ADDRESS)
    }

    fn parse_address(&self, byte: u16) -> Address {
        (byte & 0x00_ff) as Address
    }

    pub fn ingest(&mut self, byte: u16) -> Option<Response> {
        if self.is_address_byte(byte) {
            let address = self.parse_address(byte);
            if self.address_match(address) {
                match self.data_buf.push(byte as u8) {
                    Err(val) => {
                        // Errors are only thrown here if we've run out of space in the buffer.
                        // In this case we just want to empty the buffer and start over.
                        self.data_buf.clear();
                        self.data_buf.push(val).unwrap();
                    }
                    _ => (),
                }
            } else {
                // Got an address byte that we don't gaf about, need to dump buffer since
                // we *should* not be in a state to continue receiving data.
                self.data_buf.clear();
            }
        }
        match parser::parse(&self.data_buf) {
            Ok((i, o)) => {
                self.frame_buf.push(o).unwrap();
                Some(Response)
            }
            _ => None,
        }
    }
}
