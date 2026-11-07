use crate::qr::Qr;
use std::io::Write;

const QUIET: usize = 4; // marge blanche obligatoire, en modules

fn dark(qr: &Qr, x: i32, y: i32) -> bool {
    x >= 0 && y >= 0 && (x as usize) < qr.size && (y as usize) < qr.size && qr.get(x as usize, y as usize)
}

/// Deux modules par caractère grâce aux demi-blocs ▀ ▄ █.
pub fn terminal(qr: &Qr, invert: bool) -> String {
    let n = (qr.size + 2 * QUIET) as i32;
    let q = QUIET as i32;
    let mut out = String::new();
    for row in (0..n).step_by(2) {
        for x in 0..n {
            let top = dark(qr, x - q, row - q) ^ invert;
            let bottom = dark(qr, x - q, row + 1 - q) ^ invert;
            // terminal sombre : on dessine le blanc, le noir reste le fond
            out.push(match (top, bottom) {
                (false, false) => '█',
                (false, true) => '▀',
                (true, false) => '▄',
                (true, true) => ' ',
            });
        }
        out.push('\n');
    }
    out
}

pub fn svg(qr: &Qr) -> String {
    let n = qr.size + 2 * QUIET;
    let mut path = String::new();
    for y in 0..qr.size {
        let mut x = 0;
        while x < qr.size {
            if qr.get(x, y) {
                let start = x;
                while x < qr.size && qr.get(x, y) {
                    x += 1;
                }
                // une ligne horizontale de modules noirs = un seul rectangle
                path += &format!("M{},{}h{}v1h-{}z", start + QUIET, y + QUIET, x - start, x - start);
            } else {
                x += 1;
            }
        }
    }
    format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 {n} {n}\" shape-rendering=\"crispEdges\">\n\
         <rect width=\"{n}\" height=\"{n}\" fill=\"#fff\"/>\n<path d=\"{path}\" fill=\"#000\"/>\n</svg>\n"
    )
}

// PNG en niveaux de gris, deflate en blocs « stockés » : pas de compression,
// mais pas de dépendance non plus.
pub fn png(qr: &Qr, scale: usize) -> Vec<u8> {
    let n = (qr.size + 2 * QUIET) * scale;
    let mut raw = Vec::with_capacity((n + 1) * n);
    for py in 0..n {
        raw.push(0);
        for px in 0..n {
            let x = (px / scale) as i32 - QUIET as i32;
            let y = (py / scale) as i32 - QUIET as i32;
            raw.push(if dark(qr, x, y) { 0 } else { 255 });
        }
    }
    let mut z = vec![0x78, 0x01];
    let chunks: Vec<&[u8]> = raw.chunks(65535).collect();
    for (i, c) in chunks.iter().enumerate() {
        z.push((i + 1 == chunks.len()) as u8);
        let len = c.len() as u16;
        z.extend(len.to_le_bytes());
        z.extend((!len).to_le_bytes());
        z.extend_from_slice(c);
    }
    z.extend(adler32(&raw).to_be_bytes());

    let mut out = b"\x89PNG\r\n\x1a\n".to_vec();
    let mut ihdr = Vec::new();
    ihdr.extend((n as u32).to_be_bytes());
    ihdr.extend((n as u32).to_be_bytes());
    ihdr.extend([8, 0, 0, 0, 0]);
    chunk(&mut out, b"IHDR", &ihdr);
    chunk(&mut out, b"IDAT", &z);
    chunk(&mut out, b"IEND", &[]);
    out
}

fn chunk(out: &mut Vec<u8>, kind: &[u8; 4], data: &[u8]) {
    out.extend((data.len() as u32).to_be_bytes());
    let mut c = Vec::with_capacity(4 + data.len());
    c.extend_from_slice(kind);
    c.extend_from_slice(data);
    out.extend_from_slice(&c);
    out.extend(crc32(&c).to_be_bytes());
}

fn crc32(data: &[u8]) -> u32 {
    let mut crc = !0u32;
    for &b in data {
        crc ^= b as u32;
        for _ in 0..8 {
            crc = if crc & 1 != 0 { 0xEDB88320 ^ (crc >> 1) } else { crc >> 1 };
        }
    }
    !crc
}

fn adler32(data: &[u8]) -> u32 {
    let (mut a, mut b) = (1u32, 0u32);
    for &x in data {
        a = (a + x as u32) % 65521;
        b = (b + a) % 65521;
    }
    b << 16 | a
}

pub fn write_file(path: &str, bytes: &[u8]) -> std::io::Result<()> {
    std::fs::File::create(path)?.write_all(bytes)
}
