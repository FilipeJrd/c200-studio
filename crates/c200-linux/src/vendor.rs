//! C200 Extension Unit protocol, ported from Eran Sandler's MIT-licensed tools.
//! Query buffers must have the device-reported size (60 for the known FOV unit).
use std::{io, os::fd::RawFd};

pub const UNIT: u8 = 0x06;
pub const SET_CUR: u8 = 0x01;
pub const GET_CUR: u8 = 0x81;
pub const GET_LEN: u8 = 0x85;
pub const GET_INFO: u8 = 0x86;

#[derive(Debug, Clone, Copy)]
pub enum Encoding {
    Boolean,
    Byte,
    Fov,
}
#[derive(Debug, Clone, Copy)]
pub struct VendorControl {
    pub selector: u8,
    pub encoding: Encoding,
}

pub fn control(key: &str) -> Option<VendorControl> {
    let (selector, encoding) = match key {
        "fov" => (0x10, Encoding::Fov),
        "hdr" => (0x13, Encoding::Boolean),
        "horizontal_flip" => (0x11, Encoding::Boolean),
        "vertical_screen" => (0x0a, Encoding::Boolean),
        "anti_flicker" => (0x12, Encoding::Byte),
        _ => return None,
    };
    Some(VendorControl { selector, encoding })
}

pub trait Transport {
    fn query(&self, selector: u8, query: u8, data: &mut [u8]) -> io::Result<()>;
}

#[repr(C)]
struct XuQuery {
    unit: u8,
    selector: u8,
    query: u8,
    size: u16,
    data: *mut u8,
}
pub struct IoctlTransport(pub RawFd);

impl Transport for IoctlTransport {
    fn query(&self, selector: u8, query: u8, data: &mut [u8]) -> io::Result<()> {
        let mut request = XuQuery {
            unit: UNIT,
            selector,
            query,
            size: data
                .len()
                .try_into()
                .map_err(|_| invalid("Payload too large"))?,
            data: data.as_mut_ptr(),
        };
        // _IOWR('u', 0x21, struct uvc_xu_control_query), generated with the
        // target ABI's struct size; never hardcode the x86_64 ioctl number.
        let number = ((3_u32 << 30)
            | ((std::mem::size_of::<XuQuery>() as u32) << 16)
            | (u32::from(b'u') << 8)
            | 0x21) as libc::c_ulong;
        // SAFETY: fd is borrowed from a live Device. repr(C) matches Linux's
        // uvc_xu_control_query; its buffer is writable and lives for this call.
        if unsafe { libc::ioctl(self.0, number, &mut request) } < 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(())
    }
}

fn invalid(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

pub fn capabilities(io: &impl Transport, control: VendorControl) -> io::Result<(bool, bool)> {
    let mut info = [0];
    io.query(control.selector, GET_INFO, &mut info)?;
    Ok((info[0] & 1 != 0, info[0] & 2 != 0))
}

fn payload(io: &impl Transport, control: VendorControl) -> io::Result<Vec<u8>> {
    let mut bytes = [0; 2];
    io.query(control.selector, GET_LEN, &mut bytes)?;
    let len = usize::from(u16::from_le_bytes(bytes));
    let minimum = match control.encoding {
        Encoding::Fov => 4,
        _ => 1,
    };
    if !(minimum..=64).contains(&len) {
        return Err(invalid("Unexpected vendor control length"));
    }
    let mut data = vec![0; len];
    io.query(control.selector, GET_CUR, &mut data)?;
    Ok(data)
}

pub fn read(io: &impl Transport, control: VendorControl) -> io::Result<i64> {
    let data = payload(io, control)?;
    Ok(match control.encoding {
        Encoding::Fov => i64::from(u16::from_le_bytes([data[2], data[3]])),
        Encoding::Byte => i64::from(data[0]),
        Encoding::Boolean => {
            if data[0] > 1 {
                return Err(invalid("Unknown vendor boolean value"));
            }
            i64::from(data[0])
        }
    })
}

pub fn write(io: &impl Transport, control: VendorControl, value: i64) -> io::Result<()> {
    let maximum = match control.encoding {
        Encoding::Boolean => 1,
        Encoding::Byte => 255,
        Encoding::Fov => 65535,
    };
    if !(0..=maximum).contains(&value) {
        return Err(invalid("Vendor value out of range"));
    }
    // Preserve the upstream request sequence, including GET_CUR before SET_CUR.
    let mut data = payload(io, control)?;
    data.fill(0);
    match control.encoding {
        Encoding::Fov => {
            data[1] = 1;
            data[2..4].copy_from_slice(&(value as u16).to_le_bytes());
        }
        _ => data[0] = value as u8,
    }
    io.query(control.selector, SET_CUR, &mut data)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    struct Fake {
        len: u16,
        calls: RefCell<Vec<(u8, u8, Vec<u8>)>>,
        fail: bool,
    }
    impl Fake {
        fn new(len: u16) -> Self {
            Self {
                len,
                calls: RefCell::default(),
                fail: false,
            }
        }
    }
    impl Transport for Fake {
        fn query(&self, selector: u8, query: u8, data: &mut [u8]) -> io::Result<()> {
            if self.fail {
                return Err(io::Error::from_raw_os_error(libc::ENODEV));
            }
            match query {
                GET_LEN => data.copy_from_slice(&self.len.to_le_bytes()),
                GET_INFO => data[0] = 3,
                GET_CUR if data.len() >= 4 => data[2] = 78,
                _ => (),
            }
            self.calls
                .borrow_mut()
                .push((selector, query, data.to_vec()));
            Ok(())
        }
    }
    #[test]
    fn real_write_path_uses_60_bytes_and_little_endian() {
        let io = Fake::new(60);
        let fov = control("fov").unwrap();
        assert_eq!(read(&io, fov).unwrap(), 78);
        write(&io, fov, 95).unwrap();
        let calls = io.calls.borrow();
        let (selector, query, data) = calls.last().unwrap();
        assert_eq!((*selector, *query, data.len()), (0x10, SET_CUR, 60));
        assert_eq!(&data[..5], &[0, 1, 95, 0, 0]);
        assert!(data[4..].iter().all(|b| *b == 0));
        assert_eq!(
            calls.iter().map(|c| c.1).collect::<Vec<_>>(),
            [GET_LEN, GET_CUR, GET_LEN, GET_CUR, SET_CUR]
        );
    }
    #[test]
    fn all_vendor_selectors_use_the_right_encoding() {
        for (key, selector, value) in [
            ("hdr", 0x13, 1),
            ("horizontal_flip", 0x11, 0),
            ("vertical_screen", 0x0a, 1),
            ("anti_flicker", 0x12, 200),
        ] {
            let io = Fake::new(8);
            write(&io, control(key).unwrap(), value).unwrap();
            let calls = io.calls.borrow();
            let last = calls.last().unwrap();
            assert_eq!(last.0, selector);
            assert_eq!(last.2[0], value as u8);
            assert!(last.2[1..].iter().all(|b| *b == 0));
        }
    }
    #[test]
    fn malformed_lengths_and_values_never_reach_set_cur() {
        for len in [0, 1, 3, 65, 65535] {
            let io = Fake::new(len);
            assert!(write(&io, control("fov").unwrap(), 95).is_err());
            assert!(!io.calls.borrow().iter().any(|c| c.1 == SET_CUR));
        }
        assert!(write(&Fake::new(60), control("hdr").unwrap(), 2).is_err());
        assert!(write(&Fake::new(60), control("anti_flicker").unwrap(), -1).is_err());
    }
    #[test]
    fn info_and_ioctl_errors_are_propagated() {
        let mut io = Fake::new(60);
        assert_eq!(
            capabilities(&io, control("fov").unwrap()).unwrap(),
            (true, true)
        );
        io.fail = true;
        assert_eq!(
            read(&io, control("fov").unwrap())
                .unwrap_err()
                .raw_os_error(),
            Some(libc::ENODEV)
        );
    }
}
