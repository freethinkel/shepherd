//! Shared-memory frame ring (reader side).
//!
//! Layout (little-endian, contract with the ObjC writer):
//!   file header, 64 bytes: [magic u32 = 0x31464453][version u32 = 1][slots u32][pad u32][cap u64]
//!   slot i at 64 + i * (64 + cap):
//!     slot header, 64 bytes: [seq u64][width u32][height u32][source_width u32][source_height u32][bpp u32 = 4]
//!     pixels, `cap` bytes
//! `cap` must be a multiple of 64 (writer rounds w*h*4 up to 64); the reader rejects cap % 8 != 0
//! so every slot's seq word stays 8-byte aligned.
//! Seqlock: writer stores seq = 2k+1 (Relaxed), then fence(Release), then writes header+pixels,
//! then stores 2k+2 (Release). Reader: load seq (Acquire), copy, fence(Acquire), reload seq.
use crate::error::Error;
use std::ffi::CString;
use std::sync::atomic::{AtomicU64, Ordering, fence};

pub(crate) const MAGIC: u32 = 0x3146_4453;
pub(crate) const VERSION: u32 = 1;
pub(crate) const FILE_HEADER: usize = 64;
pub(crate) const SLOT_HEADER: usize = 64;

/// A decoded BGRA frame. Public API.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Frame {
    pub width: u32,
    pub height: u32,
    pub source_width: u32,
    pub source_height: u32,
    pub seq: u64,
    pub data: Vec<u8>,
}

fn last_os(what: &str) -> Error {
    Error::Shm(format!("{what}: {}", std::io::Error::last_os_error()))
}

fn total_len(slots: u32, cap: u64) -> Result<usize, Error> {
    let overflow = || Error::Shm("segment size overflow".into());
    let cap = usize::try_from(cap).map_err(|_| overflow())?;
    let stride = SLOT_HEADER.checked_add(cap).ok_or_else(overflow)?;
    let body = stride.checked_mul(slots as usize).ok_or_else(overflow)?;
    FILE_HEADER.checked_add(body).ok_or_else(overflow)
}

pub struct ShmReader {
    base: *const u8,
    len: usize,
    slots: u32,
    cap: u64,
}

// The mapping is read-only and only accessed through atomics / plain reads validated by the seqlock.
unsafe impl Send for ShmReader {}
unsafe impl Sync for ShmReader {}

impl ShmReader {
    pub fn open(name: &str, slots: u32, cap: u64) -> Result<ShmReader, Error> {
        if slots == 0 {
            return Err(Error::Shm("zero slots".into()));
        }
        let cname = CString::new(name).map_err(|_| Error::Shm("bad name".into()))?;
        if !cap.is_multiple_of(8) {
            return Err(Error::Shm(format!("cap {cap} not a multiple of 8")));
        }
        let len = total_len(slots, cap)?;
        unsafe {
            let fd = libc::shm_open(cname.as_ptr(), libc::O_RDONLY, 0);
            if fd < 0 {
                return Err(last_os("shm_open"));
            }
            let mut st: libc::stat = std::mem::zeroed();
            if libc::fstat(fd, &mut st) != 0 {
                let e = last_os("fstat");
                libc::close(fd);
                return Err(e);
            }
            if (st.st_size as usize) < len {
                libc::close(fd);
                return Err(Error::Shm(format!(
                    "segment {} bytes, need {len}",
                    st.st_size
                )));
            }
            let p = libc::mmap(
                std::ptr::null_mut(),
                len,
                libc::PROT_READ,
                libc::MAP_SHARED,
                fd,
                0,
            );
            let mmap_err = (p == libc::MAP_FAILED).then(|| last_os("mmap"));
            libc::close(fd);
            if let Some(e) = mmap_err {
                return Err(e);
            }
            let base = p as *const u8;
            let rd32 = |o: usize| (base.add(o) as *const u32).read_unaligned();
            let ok = rd32(0) == MAGIC
                && rd32(4) == VERSION
                && rd32(8) == slots
                && (base.add(16) as *const u64).read_unaligned() == cap;
            if !ok {
                libc::munmap(p, len);
                return Err(Error::Shm("bad segment header".into()));
            }
            Ok(ShmReader {
                base,
                len,
                slots,
                cap,
            })
        }
    }

    fn slot_ptr(&self, slot: u32) -> *const u8 {
        let stride = SLOT_HEADER + self.cap as usize;
        unsafe { self.base.add(FILE_HEADER + slot as usize * stride) }
    }

    /// Copy out the frame in `slot` if it still carries `seq`; None if torn or overwritten.
    pub fn read(&self, slot: u32, seq: u64) -> Option<Frame> {
        if slot >= self.slots || !seq.is_multiple_of(2) {
            return None;
        }
        let p = self.slot_ptr(slot);
        unsafe {
            let seq_word = AtomicU64::from_ptr(p as *mut u64);
            if seq_word.load(Ordering::Acquire) != seq {
                return None;
            }
            let rd32 = |o: usize| (p.add(o) as *const u32).read_unaligned();
            let (width, height) = (rd32(8), rd32(12));
            let (source_width, source_height) = (rd32(16), rd32(20));
            if rd32(24) != 4 {
                return None;
            }
            let n = (width as u64).checked_mul(height as u64)?.checked_mul(4)?;
            if n > self.cap {
                return None;
            }
            let mut data = vec![0u8; n as usize];
            std::ptr::copy_nonoverlapping(p.add(SLOT_HEADER), data.as_mut_ptr(), n as usize);
            fence(Ordering::Acquire);
            if seq_word.load(Ordering::Relaxed) != seq {
                return None;
            }
            Some(Frame {
                width,
                height,
                source_width,
                source_height,
                seq,
                data,
            })
        }
    }
}

impl Drop for ShmReader {
    fn drop(&mut self) {
        unsafe {
            libc::munmap(self.base as *mut libc::c_void, self.len);
        }
    }
}

#[cfg(test)]
pub(crate) struct FrameHeader {
    pub width: u32,
    pub height: u32,
    pub source_width: u32,
    pub source_height: u32,
}

/// Test-only mirror of what the ObjC helper does.
#[cfg(test)]
pub(crate) struct ShmWriter {
    name: CString,
    base: *mut u8,
    len: usize,
    cap: u64,
}

#[cfg(test)]
impl ShmWriter {
    pub(crate) fn create(name: &str, slots: u32, cap: u64) -> Result<ShmWriter, Error> {
        let cname = CString::new(name).map_err(|_| Error::Shm("bad name".into()))?;
        let len = total_len(slots, cap)?;
        unsafe {
            let fd = libc::shm_open(
                cname.as_ptr(),
                libc::O_RDWR | libc::O_CREAT | libc::O_EXCL,
                0o600,
            );
            if fd < 0 {
                return Err(last_os("shm_open"));
            }
            if libc::ftruncate(fd, len as libc::off_t) != 0 {
                let e = last_os("ftruncate");
                libc::close(fd);
                libc::shm_unlink(cname.as_ptr());
                return Err(e);
            }
            let p = libc::mmap(
                std::ptr::null_mut(),
                len,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_SHARED,
                fd,
                0,
            );
            libc::close(fd);
            if p == libc::MAP_FAILED {
                let e = last_os("mmap");
                libc::shm_unlink(cname.as_ptr());
                return Err(e);
            }
            let base = p as *mut u8;
            (base as *mut u32).write(MAGIC);
            (base.add(4) as *mut u32).write(VERSION);
            (base.add(8) as *mut u32).write(slots);
            (base.add(12) as *mut u32).write(0);
            (base.add(16) as *mut u64).write(cap);
            Ok(ShmWriter {
                name: cname,
                base,
                len,
                cap,
            })
        }
    }

    fn slot_ptr(&self, slot: u32) -> *mut u8 {
        unsafe {
            self.base
                .add(FILE_HEADER + slot as usize * (SLOT_HEADER + self.cap as usize))
        }
    }

    /// Full seqlock write: odd, header+pixels, then `seq_even`.
    pub(crate) fn write(&self, slot: u32, seq_even: u64, h: &FrameHeader, pixels: &[u8]) {
        self.begin(slot, seq_even);
        unsafe {
            let p = self.slot_ptr(slot);
            (p.add(8) as *mut u32).write(h.width);
            (p.add(12) as *mut u32).write(h.height);
            (p.add(16) as *mut u32).write(h.source_width);
            (p.add(20) as *mut u32).write(h.source_height);
            (p.add(24) as *mut u32).write(4);
            std::ptr::copy_nonoverlapping(pixels.as_ptr(), p.add(SLOT_HEADER), pixels.len());
        }
        self.finish(slot, seq_even);
    }

    /// Start a write: Relaxed odd store then Release fence (leaves the slot "torn").
    pub(crate) fn begin(&self, slot: u32, seq_even: u64) {
        unsafe {
            AtomicU64::from_ptr(self.slot_ptr(slot) as *mut u64)
                .store(seq_even - 1, Ordering::Relaxed)
        }
        fence(Ordering::Release);
    }

    pub(crate) fn finish(&self, slot: u32, seq_even: u64) {
        unsafe {
            AtomicU64::from_ptr(self.slot_ptr(slot) as *mut u64).store(seq_even, Ordering::Release)
        }
    }
}

#[cfg(test)]
impl Drop for ShmWriter {
    fn drop(&mut self) {
        unsafe {
            libc::munmap(self.base as *mut libc::c_void, self.len);
            libc::shm_unlink(self.name.as_ptr());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn name(tag: &str) -> String {
        format!("/sdt-{}-{tag}", std::process::id())
    }

    fn hdr(w: u32, h: u32) -> FrameHeader {
        FrameHeader {
            width: w,
            height: h,
            source_width: 100,
            source_height: 200,
        }
    }

    #[test]
    fn writer_reader_roundtrip() {
        let n = name("rt");
        let w = ShmWriter::create(&n, 3, 16).unwrap();
        let px: Vec<u8> = (0..16).collect();
        w.write(1, 2, &hdr(2, 2), &px);
        let r = ShmReader::open(&n, 3, 16).unwrap();
        let f = r.read(1, 2).unwrap();
        assert_eq!(
            f,
            Frame {
                width: 2,
                height: 2,
                source_width: 100,
                source_height: 200,
                seq: 2,
                data: px
            }
        );
    }

    #[test]
    fn read_rejects_stale_seq() {
        let n = name("stale");
        let w = ShmWriter::create(&n, 3, 16).unwrap();
        w.write(0, 6, &hdr(2, 2), &[0; 16]);
        let r = ShmReader::open(&n, 3, 16).unwrap();
        assert!(r.read(0, 4).is_none());
        assert!(r.read(0, 6).is_some());
    }

    #[test]
    fn read_rejects_torn_write() {
        let n = name("torn");
        let w = ShmWriter::create(&n, 3, 16).unwrap();
        w.write(0, 2, &hdr(2, 2), &[0; 16]);
        w.begin(0, 4);
        let r = ShmReader::open(&n, 3, 16).unwrap();
        assert!(r.read(0, 2).is_none());
        assert!(r.read(0, 4).is_none());
        assert!(r.read(0, 3).is_none());
    }

    #[test]
    fn reader_follows_new_generation() {
        let (n1, n2) = (name("g1"), name("g2"));
        let w1 = ShmWriter::create(&n1, 3, 16).unwrap();
        w1.write(0, 2, &hdr(2, 2), &[1; 16]);
        let r1 = ShmReader::open(&n1, 3, 16).unwrap();
        assert_eq!(r1.read(0, 2).unwrap().width, 2);
        drop(r1);
        let w2 = ShmWriter::create(&n2, 3, 64).unwrap();
        w2.write(0, 2, &hdr(4, 4), &[2; 64]);
        let r2 = ShmReader::open(&n2, 3, 64).unwrap();
        let f = r2.read(0, 2).unwrap();
        assert_eq!((f.width, f.height, f.data.len()), (4, 4, 64));
    }

    #[test]
    fn open_rejects_wrong_geometry() {
        let n = name("geo");
        let _w = ShmWriter::create(&n, 3, 16).unwrap();
        assert!(ShmReader::open(&n, 3, 32).is_err());
        assert!(ShmReader::open(&n, 2, 16).is_err());
    }

    #[test]
    fn open_rejects_misaligned_cap() {
        let n = name("align");
        let _w = ShmWriter::create(&n, 3, 20).unwrap();
        assert!(matches!(ShmReader::open(&n, 3, 20), Err(Error::Shm(_))));
    }

    #[test]
    fn total_len_overflow_is_error() {
        assert!(total_len(u32::MAX, u64::MAX / 2).is_err());
        assert!(total_len(3, 16).is_ok());
    }
}
