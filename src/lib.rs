#![cfg_attr(not(test), no_std)]

extern crate alloc;

use alloc::vec::Vec;

use crc::crc16;

pub mod parser;

pub struct DataFrame {
    address: u8,
    data: Vec<u8>,
    crc: u16,
}

impl DataFrame {
    pub fn crc_valid(&self) -> bool {
        crc16::checksum_usb(&self.data) == self.crc
    }
}

pub struct Transport {
    address: u8,
    data_buf: Vec<u8>,
    frame_buf: Vec<DataFrame>,
}

// impl Transport {}
