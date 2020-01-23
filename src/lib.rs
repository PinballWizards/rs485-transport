#![cfg_attr(not(test), no_std)]

use heapless::{consts::*, Vec};

use core::cmp::Ordering;
use crc::crc16;

pub mod parser;

pub type Address = u8;

pub const BROADCAST_ADDRESS: Address = 0xff;
pub const MASTER_ADDRESS: Address = 0x1;

const MAX_APPDATA_LEN: usize = 255;

pub type AppData = Vec<u8, U256>;
pub type RawDataFrame = Vec<u16, U260>;

#[derive(Debug)]
pub struct DataFrame {
    address: Address,
    data: AppData,
    crc: u16,
}

impl DataFrame {
    fn new(address: Address, app_data: &[u8]) -> Self {
        let mut ret = DataFrame {
            address: address,
            data: Vec::new(),
            crc: calculate_crc(app_data),
        };
        ret.data.extend_from_slice(app_data).unwrap();
        ret
    }

    pub fn is_broadcast(&self) -> bool {
        self.address == BROADCAST_ADDRESS
    }

    pub fn app_data(&self) -> &AppData {
        &self.data
    }

    fn to_bytes(self) -> RawDataFrame {
        let mut ret: RawDataFrame = Vec::new();
        // Pack address
        ret.push(((self.address as u16) << 4) | (1 << 8)).unwrap();
        // Pack data length
        ret.push(self.data.len() as u16).unwrap();
        // Pack application data
        for byte in self.data.iter() {
            ret.push((*byte) as u16).unwrap();
        }
        // Pack CRC value
        for byte in self.crc.to_le_bytes().iter() {
            ret.push((*byte) as u16).unwrap();
        }
        ret
    }
}

fn calculate_crc(data: &[u8]) -> u16 {
    crc16::checksum_usb(data)
}

fn crc_valid(data: &[u8], crc_value: &u16) -> Result<u16, u16> {
    let calculated_crc = calculate_crc(data);
    match calculated_crc.cmp(crc_value) {
        Ordering::Equal => Ok(calculated_crc),
        _ => Err(calculated_crc),
    }
}

pub type Response = [u8; 4];
pub const RESPONSE_ACK: Response = [0x11, 0x0, 0x0, 0x0];
pub const RESPONSE_NACK: Response = [0x12, 0x0, 0x0, 0x0];

enum Device {
    Master,
    Slave,
}

pub type DataBuffer = Vec<u8, U512>;

pub struct Transport {
    device: Device,
    address: Address,
    data_buf: DataBuffer,
}

impl Transport {
    pub fn new_slave(address: Address) -> Self {
        Transport {
            device: Device::Slave,
            address,
            data_buf: Vec::new(),
        }
    }

    pub fn new_master() -> Self {
        Transport {
            device: Device::Master,
            address: MASTER_ADDRESS,
            data_buf: Vec::new(),
        }
    }

    fn is_address_byte(&self, byte: u16) -> bool {
        byte & (1 << 8) != 0
    }

    fn address_match(&self, address: Address) -> bool {
        (self.address & address == self.address)
            || (BROADCAST_ADDRESS & address == BROADCAST_ADDRESS)
    }

    fn parse_address(&self, byte: u16) -> Address {
        ((byte & 0x00_ff) >> 4) as Address
    }

    /// This is a minimal ingester for data straight from SERCOM and should be called as soon
    /// as data is received over the bus. This function will return a response that must be sent
    /// directly to the master.
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
        } else if !self.data_buf.is_empty() {
            match self.data_buf.push(byte as u8) {
                Err(_) => {
                    // Error here means we've run out of space in the buffer...which
                    // definitely shouldn't happen.
                    self.data_buf.clear();
                    match self.device {
                        Device::Slave => return Some(RESPONSE_NACK),
                        _ => (),
                    }
                }
                _ => (),
            };

            match self.device {
                Device::Slave => {
                    match parser::parse_dataframe_noclone(&self.data_buf) {
                        Ok((_, (_, data, crc))) => {
                            // Full dataframe has been received, check the crc value and respond
                            // appropriately.
                            if crc_valid(data, &crc).is_ok() {
                                return Some(RESPONSE_ACK);
                            } else {
                                return Some(RESPONSE_NACK);
                            }
                        }
                        _ => (),
                    }
                }
                _ => (),
            }
        }
        None
    }

    /// This is a pretty costly function but should be called periodically by the CPU.
    /// Dataframes returned here should be passed to the application layer.
    pub fn parse_data_buffer(&mut self) -> Option<DataFrame> {
        match parser::parse_dataframe(&self.data_buf.clone()) {
            Ok((i, o)) => {
                self.data_buf.clear();
                self.data_buf.extend_from_slice(i).unwrap();
                Some(o)
            }
            _ => None,
        }
    }

    pub fn send(&self, app_data: &[u8]) -> Result<RawDataFrame, usize> {
        match app_data.len().cmp(&MAX_APPDATA_LEN) {
            Ordering::Greater | Ordering::Equal => Err(app_data.len()),
            _ => Ok(DataFrame::new(self.address, app_data).to_bytes()),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    const ADDRESS: Address = 0x1;
    const SPECIFIC_DATAFRAME_RAW_VALID_CRC: [u16; 5] = [0x01_10, 0x1, 0xff, 0x00, 0xff];
    const SPECIFIC_DATAFRAME_RAW_INVALID_CRC: [u16; 5] = [0x01_10, 0x1, 0xff, 0x12, 0x34];

    #[test]
    fn test_is_address_pass() {
        let t = Transport::new_slave(ADDRESS);
        let address_byte: u16 = 0x01_10;

        assert_eq!(t.is_address_byte(address_byte), true);
    }

    #[test]
    fn test_is_address_fail() {
        let t = Transport::new_slave(ADDRESS);
        let address_byte: u16 = 0;

        assert_eq!(t.is_address_byte(address_byte), false);
    }

    #[test]
    fn test_ingest() {
        let mut transport = Transport::new_slave(ADDRESS);
        for byte in SPECIFIC_DATAFRAME_RAW_VALID_CRC.iter() {
            match transport.ingest(*byte) {
                Some(resp) => {
                    assert_eq!(resp, RESPONSE_ACK);
                    return;
                }
                _ => (),
            }
        }
        println!("frame is some? {}", transport.parse_data_buffer().is_some());
        panic!("did not receive ACK with good CRC");
    }

    #[test]
    fn test_ingest_master() {
        let mut transport = Transport::new_master();
        for byte in SPECIFIC_DATAFRAME_RAW_VALID_CRC.iter() {
            match transport.ingest(*byte) {
                None => (),
                _ => panic!("got response on master ingest"),
            }
        }
    }
}
