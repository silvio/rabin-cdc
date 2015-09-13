
#![feature(test)]

use std::io::{stdin, Read};
use std::fmt;

const POLYNOMIAL: u64 = 0x3DA3358B4DC173;
const POLYNOMIAL_DEGREE: usize = 53;
const WINSIZE: usize = 64;
const AVERAGE_BITS: usize = 20;
const MINSIZE: usize = 512*1024;
const MAXSIZE: usize = 8 * 1024 * 1024;

const MASK: u64 = (1<<AVERAGE_BITS)-1;
const POLSHIFT: u64 = (POLYNOMIAL_DEGREE-8) as u64;

struct Table {
    modt: [u64;256],
    outt: [u64;256],
}

impl Table {
    fn new() -> Table {
        let mut t = Table {
            modt: [0u64;256],
            outt: [0u64;256],
        };

        t.outt = Table::generate_outt(POLYNOMIAL, WINSIZE);
        t.modt = Table::generate_modt(POLYNOMIAL);

        return t;
    }

    fn generate_outt(pol: u64, winsize: usize) -> [u64;256] {
        let mut outt = [0u64;256];

        for b in 0usize .. 256 {
            let mut hash = 0u64;

            hash = Table::append_byte(hash, b as u8, pol);
            for _ in 0 .. (winsize-1) {
                hash = Table::append_byte(hash, 0, pol);
            }
            outt[b as usize] = hash;
        }

        return outt;
    }

    fn generate_modt(pol: u64) -> [u64;256] {
        let mut modt = [0u64;256];

        let k = Table::deg(pol);

        for b in 0usize .. 256 {
            modt[b] = Table::modulo(((b << k) as u64), pol);
            modt[b] |= (b << k) as u64;
        }

        return modt;
    }

    fn deg(p: u64) -> i64 {
        let mut mask = 0x8000000000000000u64;
        for i in 0 .. 64 {
            if (mask & p) > 0 {
                return 63-i;
            }

            mask >>= 1;
        }

        return -1;
    }

    fn modulo(x: u64, p: u64) -> u64 {
        let mut out = x;
        while Table::deg(out) >= Table::deg(p) {
            let shift = Table::deg(out) - Table::deg(p);
            out = out ^ (p << shift);
        }

        return out;
    }

    fn append_byte(hash: u64, b: u8, pol: u64) -> u64 {
        let mut out = hash.clone();

        out <<= 8 as u64;
        out |= b as u64;

        return Table::modulo(out, pol);
    }
}

struct Chunk {
    start: usize,
    length: usize,
    cutfp: u64,
}

impl Chunk {
    fn new() -> Chunk {
        Chunk {
            start: 0,
            length: 0,
            cutfp: 0,
        }
    }
}

impl fmt::Display for Chunk {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Chunk(s:{:010} l:{:08} c:{:016x})", self.start, self.length, self.cutfp)
    }
}

struct Rabin<'a> {
    window: [u8; WINSIZE],
    wpos: usize,
    count: usize,
    pos: usize,
    start: usize,
    digest: u64,
    table: &'a Table,
}

impl<'a> Rabin<'a> {
    fn new(table: &'a Table) -> Rabin {
        let mut r = Rabin {
            window: [0u8; WINSIZE],
            wpos: 0,
            count: 0,
            pos: 0,
            start: 0,
            digest: 0,
            table: table,
        };

        r.reset();
        return r;
    }

    fn reset(&mut self) {
        for el in self.window.iter_mut() {
            *el = 0;
        }
        self.wpos = 0;
        self.count = 0;
        self.digest = 0;

        self.rabin_slide(1);
    }

    #[inline]
    fn rabin_slide(&mut self, b: u8) {
        let out = self.window[self.wpos];
        self.window[self.wpos] = b;
        self.digest ^= self.table.outt[out as usize];
        self.wpos = (self.wpos+1) % WINSIZE;
        self.rabin_append(b);
    }

    #[inline]
    fn rabin_append(&mut self, b: u8) {
        let index: u64 = self.digest >> POLSHIFT;
        self.digest <<= 8;
        self.digest |= b as u64;
        self.digest ^= self.table.modt[index as usize];
    }

    fn rabin_next_chunk(&mut self, buf: &[u8], start: usize) -> (Chunk, i64) {
        for i in start .. buf.len() {
            let b = buf[i];

            self.rabin_slide(b);

            self.count += 1;
            self.pos += 1;

            if ((self.count >= MINSIZE) && (self.digest & MASK) == 0) || self.count >= MAXSIZE {
                let c = Chunk {
                    start: self.start,
                    length: self.count,
                    cutfp: self.digest,
                };

                let pos = self.pos;
                self.reset();
                self.start = pos;
                self.pos = pos;

                return (c, (i+1) as i64);
            }
        }

        return (Chunk::new(), -1);
    }

    fn rabin_finalize(&self) -> Option<Chunk> {
        if self.count == 0 {
            return None;
        }

        Some(Chunk {
            start: self.start,
            length: self.count,
            cutfp: self.digest,
        })
    }
}

impl<'a> fmt::Display for Rabin<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Rabin(wp:{}, c:{}, p:{} s:{} d:{:x})", self.wpos, self.count, self.pos, self.start, self.digest)
    }
}


fn main() {

    let t = Table::new();
    let mut r = Rabin::new(&t);

    let mut chunks = 0usize;

    let mut buf_obj: Vec<u8> = Vec::new();

    let mut bytes = 0usize;
    let mut stdin = stdin();
    let mut calc_length = 0usize;

    let len = match stdin.read_to_end(&mut buf_obj) {
        Ok(t) => { t },
        Err(why) => { panic!(why) },
    };

    if len <= 0 {
        panic!("read problem");
    }

    for buf in buf_obj.chunks(1024*1024) {
        bytes += buf.len();

        let mut start = 0usize;

        loop {
            let (chunk, remaining) = r.rabin_next_chunk(buf, start);
            if remaining < 0 {
                break;
            }

            println!("{:7} {:016x}", chunk.length, chunk.cutfp);

            start = remaining as usize;

            calc_length += chunk.length;

            chunks += 1;
        }
    }

    let mut last_chunk = false;
    let chunk = match r.rabin_finalize() {
        Some(c) => { chunks += 1; last_chunk = true; c },
        None => { Chunk::new() },
    };

    if last_chunk {
        println!("{:7} {:016x}", chunk.length, chunk.cutfp);
        calc_length += chunk.length;
    }

    let mut avg = 0usize;
    if chunks > 0 {
        avg = bytes / chunks;
    }

    println!("{} chunks, average chunk size {}, sum(chunk.len)={}", chunks, avg, calc_length);
}

#[cfg(test)]
mod test {
    extern crate test;

    use super::*;
    use std::path::Path;
    use std::fs::File;
    use std::io::Read;

    #[bench]
    fn bench_refimpl_outt(b: &mut test::Bencher) {
        let mut x = [0u64;256];
        b.iter(|| {
            x = ::Table::generate_outt(::POLYNOMIAL, ::WINSIZE);
        })
    }

    #[bench]
    fn bench_refimpl_modt(b: &mut test::Bencher) {
        let mut x = [0u64;256];
        b.iter(|| {
            x = ::Table::generate_modt(::POLYNOMIAL);
        })
    }

    #[bench]
    fn bench_refimpl_a(b: &mut test::Bencher) {
        // better run this before: dd if=/dev/urandom of=test.data  count=50 bs=1M
        let path = Path::new("/tmp/test.data");
        let f = File::open(&path);
        let mut buf_obj: Vec<u8> = Vec::new();
        let len = match f.unwrap().read_to_end(&mut buf_obj) {
            Ok(t) => { t },
            Err(why) => { panic!(why) },
        };

        b.iter(|| {
            let t = ::Table::new();
            let mut r = ::Rabin::new(&t);
            let mut chunks = 0usize;
            let mut calc_length = 0usize;
            let bytes = buf_obj.len();
            let mut start = 0usize;

            loop {
                let (chunk, remaining) = r.rabin_next_chunk(&buf_obj, start);
                if remaining < 0 {
                    break;
                }

                start = remaining as usize;
                calc_length += chunk.length;

                chunks += 1;
            }

            let mut last_chunk = false;
            let chunk = match r.rabin_finalize() {
                Some(c) => { chunks += 1; last_chunk = true; c },
                None => { ::Chunk::new() },
            };

            // verify correctness
            //println!("{:7} {:016x}", chunk.length, chunk.cutfp);
        })
    }

    #[bench]
    fn bench_refimpl_b(b: &mut test::Bencher) {
        let path = Path::new("/tmp/test.data");
        let f = File::open(&path);
        let mut buf_obj: Vec<u8> = Vec::new();
        let len = match f.unwrap().read_to_end(&mut buf_obj) {
            Ok(t) => { t },
            Err(why) => { panic!(why) },
        };

        let t = ::Table::new();
        let mut r = ::Rabin::new(&t);
        let mut chunks = 0usize;
        let mut calc_length = 0usize;

        let bytes = buf_obj.len();
        let mut start = 0usize;

        b.iter(|| {
            loop {
                let (chunk, remaining) = r.rabin_next_chunk(&buf_obj, start);
                if remaining < 0 {
                    break;
                }

                start = remaining as usize;
                calc_length += chunk.length;

                chunks += 1;
            }

            let mut last_chunk = false;
            let chunk = match r.rabin_finalize() {
                Some(c) => { chunks += 1; last_chunk = true; c },
                None => { ::Chunk::new() },
            };
        })
    }

}
