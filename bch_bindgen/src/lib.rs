pub mod bcachefs;
pub mod bkey;
pub mod btree;
pub mod errcode;
pub mod fs;
pub mod keyutils;
pub mod opts;
pub mod sb_io;
pub use paste::paste;

pub mod c {
    pub use crate::bcachefs::*;
}

use c::bpos as Bpos;

pub const fn spos(inode: u64, offset: u64, snapshot: u32) -> Bpos {
    Bpos {
        inode,
        offset,
        snapshot,
    }
}

pub const fn pos(inode: u64, offset: u64) -> Bpos {
    spos(inode, offset, 0)
}

pub const POS_MIN: Bpos = spos(0, 0, 0);
pub const POS_MAX: Bpos = spos(u64::MAX, u64::MAX, 0);
pub const SPOS_MAX: Bpos = spos(u64::MAX, u64::MAX, u32::MAX);

use std::cmp::Ordering;

impl PartialEq for Bpos {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}

impl Eq for Bpos {}

impl PartialOrd for Bpos {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Bpos {
    fn cmp(&self, other: &Self) -> Ordering {
        let l_inode = self.inode;
        let r_inode = other.inode;
        let l_offset = self.offset;
        let r_offset = other.offset;
        let l_snapshot = self.snapshot;
        let r_snapshot = other.snapshot;

        l_inode
            .cmp(&r_inode)
            .then(l_offset.cmp(&r_offset))
            .then(l_snapshot.cmp(&r_snapshot))
    }
}

use std::ffi::CStr;
use std::fmt;

impl fmt::Display for c::btree_id {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let s = unsafe { CStr::from_ptr(c::bch2_btree_id_str(*self)) };
        let s = s.to_str().unwrap();
        write!(f, "{}", s)
    }
}

use std::ffi::CString;
use std::str::FromStr;
use std::{os::unix::ffi::OsStrExt, path::Path};

pub fn path_to_cstr<P: AsRef<Path>>(p: P) -> CString {
    CString::new(p.as_ref().as_os_str().as_bytes()).unwrap()
}

use std::error::Error;

#[derive(Debug)]
pub enum BchToolsErr {
    InvalidBtreeId,
    InvalidBkeyType,
    InvalidBpos,
}

impl fmt::Display for BchToolsErr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BchToolsErr::InvalidBtreeId => write!(f, "invalid btree id"),
            BchToolsErr::InvalidBkeyType => write!(f, "invalid bkey type"),
            BchToolsErr::InvalidBpos => write!(f, "invalid bpos"),
        }
    }
}

impl Error for BchToolsErr {}

impl FromStr for c::btree_id {
    type Err = BchToolsErr;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = CString::new(s).unwrap();
        let p = s.as_ptr();

        let v = unsafe {
            c::match_string(
                c::__bch2_btree_ids[..].as_ptr(),
                (-(1 as isize)) as usize,
                p,
            )
        };
        if v >= 0 {
            Ok(unsafe { std::mem::transmute(v) })
        } else {
            Err(BchToolsErr::InvalidBtreeId)
        }
    }
}

impl FromStr for c::bch_bkey_type {
    type Err = BchToolsErr;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = CString::new(s).unwrap();
        let p = s.as_ptr();

        let v = unsafe {
            c::match_string(c::bch2_bkey_types[..].as_ptr(), (-(1 as isize)) as usize, p)
        };
        if v >= 0 {
            Ok(unsafe { std::mem::transmute(v) })
        } else {
            Err(BchToolsErr::InvalidBkeyType)
        }
    }
}

impl c::printbuf {
    fn new() -> c::printbuf {
        let mut buf: c::printbuf = Default::default();

        buf.set_heap_allocated(true);
        buf
    }
}

impl Drop for c::printbuf {
    fn drop(&mut self) {
        unsafe { c::bch2_printbuf_exit(self) }
    }
}

impl fmt::Display for Bpos {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut buf = c::printbuf::new();

        unsafe { c::bch2_bpos_to_text(&mut buf, *self) };

        let s = unsafe { CStr::from_ptr(buf.buf) };
        let s = s.to_str().unwrap();
        write!(f, "{}", s)
    }
}

impl FromStr for c::bpos {
    type Err = BchToolsErr;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s == "POS_MIN" {
            return Ok(POS_MIN);
        }

        if s == "POS_MAX" {
            return Ok(POS_MAX);
        }

        if s == "SPOS_MAX" {
            return Ok(SPOS_MAX);
        }

        let mut fields = s.split(':');
        let ino_str = fields.next().ok_or(BchToolsErr::InvalidBpos)?;
        let off_str = fields.next().ok_or(BchToolsErr::InvalidBpos)?;
        let snp_str = fields.next();

        let ino: u64 = ino_str.parse().map_err(|_| BchToolsErr::InvalidBpos)?;
        let off: u64 = off_str.parse().map_err(|_| BchToolsErr::InvalidBpos)?;
        let snp: u32 = snp_str.map(|s| s.parse().ok()).flatten().unwrap_or(0);

        Ok(c::bpos {
            inode:    ino,
            offset:   off,
            snapshot: snp,
        })
    }
}

pub fn printbuf_to_formatter<F>(f: &mut fmt::Formatter<'_>, func: F) -> fmt::Result
where
    F: Fn(*mut c::printbuf),
{
    let mut buf = c::printbuf::new();

    func(&mut buf);

    let s = unsafe { CStr::from_ptr(buf.buf) };
    f.write_str(&s.to_string_lossy())
}
