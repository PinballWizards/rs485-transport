use nom::{
    multi::length_data,
    number::streaming::{le_u16, le_u8},
    sequence::tuple,
    IResult,
};

use crate::DataFrame;

pub fn parse_address(i: &[u8]) -> IResult<&[u8], u8> {
    let (input, val) = le_u8(i)?;
    Ok((input, val >> 4))
}

pub fn parse(i: &[u8]) -> IResult<&[u8], DataFrame> {
    let app_data = length_data(le_u8);
    let crc = le_u16;
    let (input, (addr, data, crcval)) = tuple((parse_address, app_data, crc))(i)?;
    Ok((
        input,
        DataFrame {
            address: addr,
            data: data.iter().cloned().collect(),
            crc: crcval,
        },
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn address_test() {
        let data = [0x10u8];
        let address = parse_address(&data);

        match address {
            Ok((_, o)) => {
                println!("parsed address: {}", o);
            }
            _ => {
                println!("failed to parse address!");
                panic!("cannot parse address");
            }
        }
    }

    #[test]
    fn data_frame_test() {
        let data = [0x10u8, 0x2, 0xff, 0xfe, 0x12, 0x34];
        let frame = parse(&data);

        match frame {
            Ok((_, o)) => {
                println!("parsed data frame!");
                println!(
                    "addr: {}\ndata len: {}\ndata: {:x?}\ncrc: {}",
                    o.address,
                    o.data.len(),
                    o.data,
                    o.crc
                );
            }
            _ => {
                println!("failed to parse data frame");
                panic!("could not parse data frame");
            }
        }
    }

    #[test]
    fn data_frame_fail() {
        let data = [0x0u8];
        let frame = parse(&data);
        match frame {
            Err(e) => println!("test failed successfully: {:?}", e),
            _ => {
                println!("test didn't fail");
                panic!("test should have failed");
            }
        }
    }

    #[test]
    fn crc_check() {
        let data = [0x10u8, 0x2, 0xff, 0xfe, 0x12, 0x34];
        let frame = parse(&data);

        match frame {
            Ok((_, o)) => {
                assert_eq!(o.crc_valid(), true);
            }
            _ => {
                panic!("failed to parse data frame");
            }
        }
    }
}
