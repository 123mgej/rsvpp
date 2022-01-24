mod socket;
mod util;
mod vpp_types;

use std::ffi::CStr;

use libc::{c_char, c_void};
use socket::Socket;

use crate::{Error, Result};

#[derive(Debug)]
pub struct InterfaceStats {
    pub name: &'static str,
    pub tx_bytes: u64,
    pub tx_pkts: u64,
    pub rx_bytes: u64,
    pub rx_pkts: u64,
}

pub struct Stats {
    header: &'static vpp_types::StatSegmentSharedHeader,
}

impl Stats {
    pub async fn connect(path: &str) -> Result<Self> {
        let sock = Socket::connect(path)?;
        let header = sock.get_mmap_header()?;

        if header.version != 2 {
            return Err(Error::vpp_api("Stats version mismatch"));
        }

        Ok(Self { header })
    }

    pub fn interface(&self) -> Vec<InterfaceStats> {
        let mut names_vector = 0usize;
        let mut rx_ptr = 0usize;
        let mut tx_ptr = 0usize;

        for entry in self.header.segments() {
            if true || entry.name_starts_with("/if/") {
                if entry.name_starts_with("/if/names") {
                    names_vector = unsafe { entry.u.data };
                }
                if entry.name_starts_with("/if/rx") && !entry.name_starts_with("/if/rx-") {
                    rx_ptr = unsafe { entry.u.data };
                }
                if entry.name_starts_with("/if/tx") && !entry.name_starts_with("/if/tx-") {
                    tx_ptr = unsafe { entry.u.data };
                }
            }
        }

        if names_vector == 0 || rx_ptr == 0 || tx_ptr == 0 {
            return vec![];
        }

        let names_vector =
            self.header.adjust_ptr(names_vector as *const c_void) as *const *const c_void;
        let rx_ptr = unsafe {
            self.header.adjust_ptr(
                *(self.header.adjust_ptr(rx_ptr as *const c_void) as *const *const c_void),
            )
        } as *const VlibCounterT;
        let tx_ptr = unsafe {
            self.header.adjust_ptr(
                *(self.header.adjust_ptr(tx_ptr as *const c_void) as *const *const c_void),
            )
        } as *const VlibCounterT;

        let iface_count = unsafe { util::vec_len(names_vector as *const c_void) };
        (0..iface_count)
            .map(|index| {
                let iface_name_ptr =
                    unsafe { self.header.adjust_ptr(*names_vector.offset(index as isize)) };
                let iface_name_c_str = unsafe { CStr::from_ptr(iface_name_ptr as *const c_char) };
                let iface_name = iface_name_c_str.to_str().unwrap_or("Name utf8 error");
                let rx = unsafe { &*rx_ptr.offset(index as isize) };
                let tx = unsafe { &*tx_ptr.offset(index as isize) };

                InterfaceStats {
                    name: iface_name,
                    tx_bytes: tx.bytes,
                    rx_bytes: rx.bytes,
                    tx_pkts: tx.packets,
                    rx_pkts: rx.packets,
                }
            })
            .collect()
    }
}

#[repr(C)]
#[derive(Debug)]
struct VlibCounterT {
    packets: u64,
    bytes: u64,
}
