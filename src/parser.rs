use nom::{
    multi::length_data,
    number::streaming::{le_u16, le_u8},
    sequence::tuple,
    IResult,
};

use crate::DataFrame;

fn parse_address(i: &[u8]) -> IResult<&[u8], u8> {
    let (input, val) = le_u8(i)?;
    Ok((input, val >> 4))
}

fn parse_datalength(i: &[u8]) -> IResult<&[u8], u8> {
    le_u8(i)
}

/// Helper method for Transport::ingest() to quickly determine data payload length
/// from front end of buffer.
pub fn parse_only_datalength(i: &[u8]) -> IResult<&[u8], u8> {
    let (input, (_, data_len)) = tuple((parse_address, parse_datalength))(i)?;
    Ok((input, data_len))
}

fn parse_app_data(i: &[u8]) -> IResult<&[u8], &[u8]> {
    length_data(parse_datalength)(i)
}

fn parse_crc(i: &[u8]) -> IResult<&[u8], u16> {
    le_u16(i)
}

pub fn parse_only_crc(i: &[u8]) -> IResult<&[u8], u16> {
    let (input, (_, _, crcval)) = tuple((parse_address, parse_app_data, parse_crc))(i)?;
    Ok((input, crcval))
}

/// Parses a complete data frame from a u8 slice.
pub fn parse_dataframe(i: &[u8]) -> IResult<&[u8], DataFrame> {
    let (input, (addr, data, crcval)) = tuple((parse_address, parse_app_data, parse_crc))(i)?;
    Ok((
        input,
        DataFrame {
            address: addr,
            data: data.iter().cloned().collect(),
            crc: crcval,
        },
    ))
}

pub fn parse_dataframe_noclone(i: &[u8]) -> IResult<&[u8], (u8, &[u8], u16)> {
    let (input, (addr, data, crcval)) = tuple((parse_address, parse_app_data, parse_crc))(i)?;
    Ok((input, (addr, data, crcval)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crc_valid;
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
        let frame = parse_dataframe(&data);

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
        let frame = parse_dataframe(&data);
        match frame {
            Err(e) => println!("test failed successfully: {:?}", e),
            _ => {
                println!("test didn't fail");
                panic!("test should have failed");
            }
        }
    }

    #[test]
    fn parse_crc_check() {
        let val = 0x670fu16;
        match parse_crc(&val.to_le_bytes()) {
            Ok((_, o)) => {
                assert_eq!(o, val);
            }
            _ => (),
        }
    }

    #[test]
    fn crc_check() {
        let data = [0x10u8, 0x1, 0xff, 0x00, 0xff];
        let frame = parse_dataframe(&data);

        match frame {
            Ok((_, o)) => {
                assert_eq!(crc_valid(&o.data, &o.crc).is_ok(), true);
            }
            _ => {
                panic!("failed to parse data frame");
            }
        }
    }
}
