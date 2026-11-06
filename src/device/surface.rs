//! A frame's pixels in an IOSurface-backed `CVPixelBuffer`, which the window draws with GPUI's `paint_surface`: the
//! GPU reads the surface itself, so a frame costs one copy into it and no texture of its own (drawn as an image, each
//! frame had the sprite atlas make, fill and free a Metal texture, which stuttered). The buffers come from a pool per
//! size, recycled once the window lets go of them.
use core_foundation::base::{CFType, TCFType};
use core_foundation::dictionary::CFDictionary;
use core_foundation::number::CFNumber;
use core_foundation::string::CFString;
use core_video::pixel_buffer::{CVPixelBuffer, CVPixelBufferKeys, kCVPixelFormatType_32BGRA};
use core_video::pixel_buffer_pool::CVPixelBufferPool;
use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};

/// A BGRA buffer of one frame. Core Video's objects may be shared between threads; the crate's wrappers just don't
/// say so.
pub struct Surface(CVPixelBuffer);
unsafe impl Send for Surface {}
unsafe impl Sync for Surface {}

struct Pool(CVPixelBufferPool);
unsafe impl Send for Pool {}

static POOLS: LazyLock<Mutex<HashMap<(u32, u32), Pool>>> = LazyLock::new(Mutex::default);

impl Surface {
    /// The buffer, for `paint_surface`.
    pub fn buffer(&self) -> CVPixelBuffer {
        self.0.clone()
    }

    /// A buffer someone else filled: VideoToolbox's, made to `attributes`.
    pub fn of(buffer: CVPixelBuffer) -> Surface {
        Surface(buffer)
    }

    /// `pixels` (BGRA, or RGB turned BGRA on the way) in a surface of `width` × `height` from that size's pool;
    /// `None` when they don't fill it or Core Video gives no buffer.
    // ponytail: the pool hands a buffer out once nothing holds it, and the window drops the last frame's as it paints
    // the next, so a frame written while the GPU still reads it would tear; it hasn't shown, a third buffer in
    // flight if it does
    pub fn new(width: u32, height: u32, pixels: &[u8], rgb: bool) -> Option<Surface> {
        let (w, h) = (width as usize, height as usize);
        let bpp = if rgb { 3 } else { 4 };
        if w == 0 || h == 0 || pixels.len() < w * h * bpp {
            return None;
        }
        let buffer = {
            let mut pools = POOLS.lock().ok()?;
            let pool = match pools.get(&(width, height)) {
                Some(pool) => pool,
                None => pools.entry((width, height)).or_insert(Pool(pool(width, height)?)),
            };
            pool.0.create_pixel_buffer().ok()?
        };
        if buffer.lock_base_address(0) != 0 {
            return None;
        }
        let stride = buffer.get_bytes_per_row();
        let base = unsafe { buffer.get_base_address() } as *mut u8;
        for y in 0..h {
            // the row is ours while the buffer's locked, and `stride` fits the buffer's own width
            let row = unsafe { std::slice::from_raw_parts_mut(base.add(y * stride), w * 4) };
            let src = &pixels[y * w * bpp..(y + 1) * w * bpp];
            if rgb {
                for (o, p) in row.as_chunks_mut::<4>().0.iter_mut().zip(src.as_chunks::<3>().0) {
                    (o[0], o[1], o[2], o[3]) = (p[2], p[1], p[0], 255);
                }
            } else {
                row.copy_from_slice(src);
            }
        }
        buffer.unlock_base_address(0);
        Some(Surface(buffer))
    }

    #[cfg(test)]
    fn bytes(&self) -> Vec<u8> {
        let (w, h, stride) = (self.0.get_width(), self.0.get_height(), self.0.get_bytes_per_row());
        self.0.lock_base_address(core_video::pixel_buffer::kCVPixelBufferLock_ReadOnly);
        let base = unsafe { self.0.get_base_address() } as *const u8;
        let bytes = (0..h).flat_map(|y| unsafe { std::slice::from_raw_parts(base.add(y * stride), w * 4) }.to_vec()).collect();
        self.0.unlock_base_address(core_video::pixel_buffer::kCVPixelBufferLock_ReadOnly);
        bytes
    }
}

/// A pool of BGRA IOSurface-backed buffers of one size.
fn pool(width: u32, height: u32) -> Option<CVPixelBufferPool> {
    CVPixelBufferPool::new(None, Some(&attributes(Some((width, height))))).ok()
}

/// A BGRA IOSurface-backed buffer's attributes, of a size or any.
pub fn attributes(size: Option<(u32, u32)>) -> CFDictionary<CFString, CFType> {
    let key = |k: CVPixelBufferKeys| CFString::from(k);
    let number = |n: u32| CFNumber::from(i64::from(n)).as_CFType();
    let io_surface = CFDictionary::<CFString, CFType>::from_CFType_pairs(&[]).as_CFType();
    let mut pairs = vec![(key(CVPixelBufferKeys::PixelFormatType), number(kCVPixelFormatType_32BGRA)), (key(CVPixelBufferKeys::IOSurfaceProperties), io_surface)];
    pairs.extend(size.into_iter().flat_map(|(w, h)| [(key(CVPixelBufferKeys::Width), number(w)), (key(CVPixelBufferKeys::Height), number(h))]));
    CFDictionary::from_CFType_pairs(&pairs)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn surfaces_hold_the_pixels_bgra() {
        let s = Surface::new(2, 1, &[1, 2, 3, 4, 5, 6], true).expect("a surface");
        assert_eq!(s.bytes(), [3, 2, 1, 255, 6, 5, 4, 255]);
        assert_eq!(s.0.get_pixel_format(), kCVPixelFormatType_32BGRA);
        let s = Surface::new(1, 2, &[1, 2, 3, 4, 5, 6, 7, 8], false).expect("a surface");
        assert_eq!(s.bytes(), [1, 2, 3, 4, 5, 6, 7, 8]);
        assert!(Surface::new(2, 2, &[0; 4], false).is_none());
    }
}
