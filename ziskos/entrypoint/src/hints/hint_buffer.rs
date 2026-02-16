use std::collections::VecDeque;
use std::fs::File;
use std::io::{self, Read, Write};
use std::sync::{Arc, Condvar, Mutex};

pub const MAX_WRITER_LEN: usize = 128 * 1024;

pub struct HintBuffer {
    inner: Mutex<HintBufferInner>,
    not_empty: Condvar,
}

struct RefVerify {
    file: File,
    off: u64,
    scratch: Vec<u8>,
}

struct HintBufferInner {
    cur: Vec<u8>,
    queue: VecDeque<Vec<u8>>,
    closed: bool,
    paused: bool,

    // Reference verification state (DEBUG_HINTS_REF)
    ref_verify: Option<RefVerify>,
}

pub fn build_hint_buffer() -> Arc<HintBuffer> {
    Arc::new(HintBuffer {
        inner: Mutex::new(HintBufferInner {
            cur: Vec::new(),
            queue: VecDeque::new(),
            closed: true,
            paused: true,
            ref_verify: None,
        }),
        not_empty: Condvar::new(),
    })
}

impl HintBuffer {
    #[inline(always)]
    fn build_header_u64(hint_id: u32, len: usize, is_result: bool) -> u64 {
        let len32: u32 = len
            .try_into()
            .expect("hint len exceeds u32::MAX (protocol uses 32-bit len)");

        let hi: u32 = (hint_id & 0x7FFF_FFFF) | (if is_result { 0x8000_0000 } else { 0 });
        ((hi as u64) << 32) | (len32 as u64)
    }

    #[inline(always)]
    fn pad8(len: usize) -> usize {
        (8 - (len & 7)) & 7
    }

    /// Lazily init ref verification (DEBUG_HINTS_REF) if present.
    /// Also verifies START marker immediately on init.
    fn ensure_ref_verify(g: &mut HintBufferInner) {
        if g.ref_verify.is_some() {
            return;
        }

        let file_name = match std::env::var("DEBUG_HINTS_REF") {
            Ok(s) if !s.is_empty() => s,
            _ => return, // ref verification disabled
        };

        println!("DEBUG_HINTS_REF: opening reference file '{}'", file_name);
        let mut rv = RefVerify {
            file: File::open(&file_name).unwrap_or_else(|e| {
                panic!("Failed to open DEBUG_HINTS_REF '{}': {}", file_name, e)
            }),
            off: 0,
            scratch: Vec::new(),
        };

        // Verify START marker (u64 = 0) at the beginning of the stream
        let start = 0u64.to_le_bytes();
        Self::verify_against_ref_impl(&mut rv, &start);

        g.ref_verify = Some(rv);
    }

    fn verify_against_ref(g: &mut HintBufferInner, data: &[u8]) {
        if g.ref_verify.is_none() {
            return;
        }
        let rv = g.ref_verify.as_mut().unwrap();
        Self::verify_against_ref_impl(rv, data);
    }

    fn verify_against_ref_impl(rv: &mut RefVerify, data: &[u8]) {
        // Reuse scratch to avoid reallocations
        rv.scratch.resize(data.len(), 0u8);
        rv.file
            .read_exact(&mut rv.scratch)
            .unwrap_or_else(|e| {
                let tid = std::thread::current().id();
                panic!(
                    "DEBUG_HINTS_REF mismatch: failed to read {} bytes at offset {}, threadid: {:?}: {}",
                    data.len(),
                    rv.off,
                    tid,
                    e
                )
            });

        if rv.scratch != data {
            let mut i = 0usize;
            while i < data.len() && rv.scratch[i] == data[i] {
                i += 1;
            }
            let got = data[i];
            let exp = rv.scratch[i];
            let tid = std::thread::current().id();
            panic!(
                "DEBUG_HINTS_REF mismatch at ref stream offset {} (chunk idx {}): expected 0x{:02x}, got 0x{:02x}, threadid: {:?}",
                rv.off + i as u64,
                i,
                exp,
                got,
                tid
            );
        }

        rv.off += data.len() as u64;
    }

    fn finalize_ref_verify(g: &mut HintBufferInner) {
        let Some(rv) = g.ref_verify.as_mut() else {
            return;
        };

        // Verify END marker
        let end_header: u64 = (1u64 << 32) | 0u64;
        let end = end_header.to_le_bytes();
        Self::verify_against_ref_impl(rv, &end);

        // Ensure fully consumed (no trailing bytes)
        let mut extra = [0u8; 1];
        if let Ok(()) = rv.file.read_exact(&mut extra) {
            let tid = std::thread::current().id();
            panic!(
                "DEBUG_HINTS_REF mismatch: reference file has extra trailing data starting at offset {} (next byte 0x{:02x}), threadid: {:?}",
                rv.off,
                extra[0],
                tid
            );
        }

        // Drop ref verifier
        g.ref_verify = None;
    }

    pub fn close(&self) {
        let mut g = self.inner.lock().unwrap();

        // If ref verification is enabled, we expect the stream to finish here.
        // (If you call close() before draining / before emitting all hints, this can legitimately panic.)
        Self::finalize_ref_verify(&mut g);

        g.cur.clear();
        g.queue.clear();
        g.closed = true;
        g.paused = true;
        g.count = 0;
        self.not_empty.notify_all();
    }

    pub fn reset(&self) {
        let mut g = self.inner.lock().unwrap();

        g.cur.clear();
        g.queue.clear();
        g.closed = false;
        g.paused = false;
        g.count = 0;

        // Re-init reference verification for a new stream and verify START marker.
        g.ref_verify = None;
        Self::ensure_ref_verify(&mut g);

        self.not_empty.notify_all();
    }

    #[inline(always)]
    pub fn pause(&self) {
        self.inner.lock().unwrap().paused = true;
    }

    #[inline(always)]
    pub fn resume(&self) {
        self.inner.lock().unwrap().paused = false;
    }

    #[inline(always)]
    pub fn is_paused(&self) -> bool {
        self.inner.lock().unwrap().paused
    }

    #[inline(always)]
    pub fn is_enabled(&self) -> bool {
        let g = self.inner.lock().unwrap();
        !g.paused && !g.closed
    }

    #[inline(always)]
    pub unsafe fn write_hint_segments(
        &self,
        hint_id: u32,
        segments: &[*const u8],
        lengths: &[usize],
        is_result: bool,
    ) {
        if !self.is_enabled() || !crate::hints::check_main_thread() {
            return;
        }
        debug_assert_eq!(segments.len(), lengths.len(), "segments/lengths mismatch");

        let mut total = 0usize;
        for (&p, &l) in segments.iter().zip(lengths.iter()) {
            debug_assert!(l == 0 || !p.is_null(), "null ptr with nonzero len");
            total = total.checked_add(l).expect("total len overflow");
        }

        let pad = Self::pad8(total);
        let header = Self::build_header_u64(hint_id, total, is_result).to_le_bytes();

        let mut g = self.inner.lock().unwrap();

        // Ensure reference verifier (and START) is ready, then compare this hint bytes right here.
        Self::ensure_ref_verify(&mut g);

        g.cur.reserve(8 + total + pad);
        g.cur.extend_from_slice(&header);

        for (&p, &l) in segments.iter().zip(lengths.iter()) {
            if l == 0 {
                continue;
            }
            let s = std::slice::from_raw_parts(p, l);
            g.cur.extend_from_slice(s);
        }

        if pad > 0 {
            const ZERO_PAD: [u8; 8] = [0; 8];
            g.cur.extend_from_slice(&ZERO_PAD[..pad]);
        }

        #[cfg(zisk_hints_metrics)]
        {
            crate::hints::metrics::inc_hint_count(hint_id);
        }

        let hint = std::mem::take(&mut g.cur);
        g.cur = Vec::new();

        Self::verify_against_ref(&mut g, &hint);

        g.queue.push_back(hint);
        drop(g);

        self.not_empty.notify_one();
    }

    #[inline(always)]
    pub unsafe fn write_hint_len_prefixed_segments(
        &self,
        hint_id: u32,
        segments: &[*const u8],
        lengths: &[usize],
        is_result: bool,
    ) {
        if !self.is_enabled() || !crate::hints::check_main_thread() {
            return;
        }
        debug_assert_eq!(segments.len(), lengths.len(), "segments/lengths mismatch");

        // total payload = sum(8 + len_i)
        let mut total = 0usize;
        for (&p, &l) in segments.iter().zip(lengths.iter()) {
            debug_assert!(l == 0 || !p.is_null(), "null ptr with nonzero len");
            total = total.checked_add(8 + l).expect("total len overflow");
        }

        let pad = Self::pad8(total);
        let header = Self::build_header_u64(hint_id, total, is_result).to_le_bytes();

        let mut g = self.inner.lock().unwrap();

        crate::hints::check_main_thread();
        debug_assert!(g.cur.is_empty(), "cur not empty at emit start");

        // Ensure reference verifier (and START) is ready, then compare this hint bytes right here.
        Self::ensure_ref_verify(&mut g);

        g.cur.reserve(8 + total + pad);
        g.cur.extend_from_slice(&header);

        for (&p, &l) in segments.iter().zip(lengths.iter()) {
            let len_bytes = (l as u64).to_le_bytes();
            g.cur.extend_from_slice(&len_bytes);

            if l != 0 {
                let s = std::slice::from_raw_parts(p, l);
                g.cur.extend_from_slice(s);
            }
        }

        if pad > 0 {
            const ZERO_PAD: [u8; 8] = [0; 8];
            g.cur.extend_from_slice(&ZERO_PAD[..pad]);
        }

        #[cfg(zisk_hints_metrics)]
        {
            crate::hints::metrics::inc_hint_count(hint_id);
        }

        let hint = std::mem::take(&mut g.cur);
        g.cur = Vec::new();

        Self::verify_against_ref(&mut g, &hint);

        g.queue.push_back(hint);
        drop(g);

        self.not_empty.notify_one();
    }

    pub fn drain_to_writer<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        let mut debug_file = match std::env::var("DEBUG_HINTS_FILE") {
            Ok(file_name) => {
                if !file_name.is_empty() {
                    println!("DEBUG_HINTS_FILE: opening debug output file '{}'", file_name);
                    match File::create(&file_name) {
                        Ok(f) => Some(f),
                        Err(e) => {
                            eprintln!("Failed to open DEBUG_HINTS_FILE '{}': {}", file_name, e);
                            None
                        }
                    }
                } else {
                    None
                }
            }
            _ => None,
        };

        // (Opcional) si quieres que DEBUG_HINTS_FILE incluya START:
        if let Some(f) = debug_file.as_mut() {
            f.write_all(&0u64.to_le_bytes())?;
        }

        let mut out_buf: Vec<u8> = Vec::with_capacity(MAX_WRITER_LEN);

        loop {
            let hint: Option<Vec<u8>> = {
                let mut g = self.inner.lock().unwrap();

                while g.queue.is_empty() && !g.closed {
                    g = self.not_empty.wait(g).unwrap();
                }

                if g.queue.is_empty() && g.closed {
                    None
                } else {
                    Some(g.queue.pop_front().unwrap())
                }
            };

            let Some(hint_bytes) = hint else {
                if !out_buf.is_empty() {
                    writer.write_all(&out_buf)?;
                    if let Some(f) = debug_file.as_mut() {
                        f.write_all(&out_buf)?;
                        f.flush()?;
                    }
                    out_buf.clear();
                }

                if let Some(f) = debug_file.as_mut() {
                    let end_header: u64 = (1u64 << 32) | 0u64;
                    f.write_all(&end_header.to_le_bytes())?;
                    f.flush()?;
                }

                return Ok(());
            };

            if !out_buf.is_empty() && out_buf.len() + hint_bytes.len() > MAX_WRITER_LEN {
                writer.write_all(&out_buf)?;
                if let Some(f) = debug_file.as_mut() {
                    f.write_all(&out_buf)?;
                }
                out_buf.clear();
            }

            if hint_bytes.len() > MAX_WRITER_LEN {
                let mut off = 0usize;
                while off < hint_bytes.len() {
                    let n = std::cmp::min(MAX_WRITER_LEN, hint_bytes.len() - off);
                    let part = &hint_bytes[off..off + n];

                    writer.write_all(part)?;
                    if let Some(f) = debug_file.as_mut() {
                        f.write_all(part)?;
                    }

                    off += n;
                }
                continue;
            }

            out_buf.extend_from_slice(&hint_bytes);
        }
    }
}
