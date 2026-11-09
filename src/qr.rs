use crate::gf::{self, Tables};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Ecc {
    L,
    M,
    Q,
    H,
}

impl Ecc {
    fn index(self) -> usize {
        self as usize
    }
    // bits de format : L=01, M=00, Q=11, H=10 (oui, pas dans l'ordre)
    fn format_bits(self) -> u32 {
        match self {
            Ecc::L => 1,
            Ecc::M => 0,
            Ecc::Q => 3,
            Ecc::H => 2,
        }
    }
    pub fn parse(s: &str) -> Option<Ecc> {
        match s.to_ascii_uppercase().as_str() {
            "L" => Some(Ecc::L),
            "M" => Some(Ecc::M),
            "Q" => Some(Ecc::Q),
            "H" => Some(Ecc::H),
            _ => None,
        }
    }
}

// Tables du standard (ISO 18004, table 9), indexées par [niveau][version]
const ECC_PER_BLOCK: [[i16; 41]; 4] = [
    [-1, 7, 10, 15, 20, 26, 18, 20, 24, 30, 18, 20, 24, 26, 30, 22, 24, 28, 30, 28, 28, 28, 28, 30, 30, 26, 28, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30],
    [-1, 10, 16, 26, 18, 24, 16, 18, 22, 22, 26, 30, 22, 22, 24, 24, 28, 28, 26, 26, 26, 26, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28],
    [-1, 13, 22, 18, 26, 18, 24, 18, 22, 20, 24, 28, 26, 24, 20, 30, 24, 28, 28, 26, 30, 28, 30, 30, 30, 30, 28, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30],
    [-1, 17, 28, 22, 16, 22, 28, 26, 26, 24, 28, 24, 28, 22, 24, 24, 30, 28, 28, 26, 28, 30, 24, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30],
];
const NUM_BLOCKS: [[i16; 41]; 4] = [
    [-1, 1, 1, 1, 1, 1, 2, 2, 2, 2, 4, 4, 4, 4, 4, 6, 6, 6, 6, 7, 8, 8, 9, 9, 10, 12, 12, 12, 13, 14, 15, 16, 17, 18, 19, 19, 20, 21, 22, 24, 25],
    [-1, 1, 1, 1, 2, 2, 4, 4, 4, 5, 5, 5, 8, 9, 9, 10, 10, 11, 13, 14, 16, 17, 17, 18, 20, 21, 23, 25, 26, 28, 29, 31, 33, 35, 37, 38, 40, 43, 45, 47, 49],
    [-1, 1, 1, 2, 2, 4, 4, 6, 6, 8, 8, 8, 10, 12, 16, 12, 17, 16, 18, 21, 20, 23, 23, 25, 27, 29, 34, 34, 35, 38, 40, 43, 45, 48, 51, 53, 56, 59, 62, 65, 68],
    [-1, 1, 1, 2, 4, 4, 4, 5, 6, 8, 8, 11, 11, 16, 16, 18, 16, 19, 21, 25, 25, 25, 34, 30, 32, 35, 37, 40, 42, 45, 48, 51, 54, 57, 60, 63, 66, 70, 74, 77, 81],
];

/// Nombre de modules disponibles pour les données (+ correction) dans une version.
fn raw_data_modules(ver: usize) -> usize {
    let mut n = (16 * ver + 128) * ver + 64;
    if ver >= 2 {
        let align = ver / 7 + 2;
        n -= (25 * align - 10) * align - 55;
        if ver >= 7 {
            n -= 36;
        }
    }
    n
}

pub fn data_codewords(ver: usize, ecc: Ecc) -> usize {
    let e = ecc.index();
    raw_data_modules(ver) / 8 - ECC_PER_BLOCK[e][ver] as usize * NUM_BLOCKS[e][ver] as usize
}

// ---------------------------------------------------------------- segments

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Mode {
    Numeric,
    Alnum,
    Byte,
}

const ALNUM: &[u8] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ $%*+-./:";

impl Mode {
    fn indicator(self) -> u32 {
        match self {
            Mode::Numeric => 0b0001,
            Mode::Alnum => 0b0010,
            Mode::Byte => 0b0100,
        }
    }
    fn count_bits(self, ver: usize) -> usize {
        let i = if ver <= 9 { 0 } else if ver <= 26 { 1 } else { 2 };
        match self {
            Mode::Numeric => [10, 12, 14][i],
            Mode::Alnum => [9, 11, 13][i],
            Mode::Byte => [8, 16, 16][i],
        }
    }
    fn accepts(self, b: u8) -> bool {
        match self {
            Mode::Numeric => b.is_ascii_digit(),
            Mode::Alnum => ALNUM.contains(&b),
            Mode::Byte => true,
        }
    }
    // bits de données pour n caractères (sans en-tête)
    fn data_bits(self, n: usize) -> usize {
        match self {
            Mode::Numeric => n / 3 * 10 + [0, 4, 7][n % 3],
            Mode::Alnum => n / 2 * 11 + (n % 2) * 6,
            Mode::Byte => n * 8,
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            Mode::Numeric => "numérique",
            Mode::Alnum => "alphanumérique",
            Mode::Byte => "octets",
        }
    }
}

#[derive(Clone, Debug)]
pub struct Segment {
    pub mode: Mode,
    pub data: Vec<u8>,
}

/// Découpage optimal du texte en segments (numérique / alphanumérique /
/// octets) par programmation dynamique. Le coût d'un changement de mode
/// (4 bits + compteur) dépend de la version, d'où le paramètre `ver`.
pub fn segment(text: &[u8], ver: usize) -> Vec<Segment> {
    const MODES: [Mode; 3] = [Mode::Numeric, Mode::Alnum, Mode::Byte];
    let n = text.len();
    if n == 0 {
        return vec![Segment { mode: Mode::Byte, data: vec![] }];
    }
    // best[i] = (coût minimal en bits pour text[..i], début du dernier segment, mode)
    let mut best: Vec<(usize, usize, Mode)> = vec![(usize::MAX, 0, Mode::Byte); n + 1];
    best[0].0 = 0;
    for i in 0..n {
        if best[i].0 == usize::MAX {
            continue;
        }
        for &m in &MODES {
            let header = 4 + m.count_bits(ver);
            let max = (1usize << m.count_bits(ver)) - 1;
            let mut j = i;
            while j < n && m.accepts(text[j]) && j - i < max {
                j += 1;
                let cost = best[i].0 + header + m.data_bits(j - i);
                if cost < best[j].0 {
                    best[j] = (cost, i, m);
                }
            }
        }
    }
    let mut segs = Vec::new();
    let mut end = n;
    while end > 0 {
        let (_, start, mode) = best[end];
        segs.push(Segment { mode, data: text[start..end].to_vec() });
        end = start;
    }
    segs.reverse();
    // deux segments consécutifs du même mode : on les recolle
    let mut merged: Vec<Segment> = Vec::new();
    for s in segs {
        match merged.last_mut() {
            Some(last) if last.mode == s.mode => last.data.extend(s.data),
            _ => merged.push(s),
        }
    }
    merged
}

fn segments_bits(segs: &[Segment], ver: usize) -> Option<usize> {
    let mut total = 0;
    for s in segs {
        if s.data.len() >= 1 << s.mode.count_bits(ver) {
            return None;
        }
        total += 4 + s.mode.count_bits(ver) + s.mode.data_bits(s.data.len());
    }
    Some(total)
}

struct Bits(Vec<bool>);

impl Bits {
    fn push(&mut self, value: u32, n: usize) {
        for i in (0..n).rev() {
            self.0.push((value >> i) & 1 == 1);
        }
    }
}

// En-tête ECI 26 = « la suite est en UTF-8 ». Sans lui beaucoup de lecteurs
// devinent l'encodage, et transforment « ça » en idéogrammes.
const ECI_UTF8_BITS: usize = 4 + 8;

fn encode_segments(segs: &[Segment], ver: usize, capacity: usize, utf8: bool) -> Vec<u8> {
    let mut bits = Bits(Vec::new());
    if utf8 {
        bits.push(0b0111, 4);
        bits.push(26, 8);
    }
    for s in segs {
        bits.push(s.mode.indicator(), 4);
        bits.push(s.data.len() as u32, s.mode.count_bits(ver));
        match s.mode {
            Mode::Numeric => {
                for chunk in s.data.chunks(3) {
                    let v = chunk.iter().fold(0u32, |acc, &d| acc * 10 + (d - b'0') as u32);
                    bits.push(v, chunk.len() * 3 + 1);
                }
            }
            Mode::Alnum => {
                let idx = |b: u8| ALNUM.iter().position(|&c| c == b).unwrap() as u32;
                for chunk in s.data.chunks(2) {
                    if chunk.len() == 2 {
                        bits.push(idx(chunk[0]) * 45 + idx(chunk[1]), 11);
                    } else {
                        bits.push(idx(chunk[0]), 6);
                    }
                }
            }
            Mode::Byte => {
                for &b in &s.data {
                    bits.push(b as u32, 8);
                }
            }
        }
    }
    let cap_bits = capacity * 8;
    let term = (cap_bits - bits.0.len()).min(4);
    bits.push(0, term);
    let pad = (8 - bits.0.len() % 8) % 8;
    bits.push(0, pad);
    let mut bytes: Vec<u8> = bits.0.chunks(8).map(|c| c.iter().fold(0u8, |a, &b| a << 1 | b as u8)).collect();
    let mut filler = [0xEC, 0x11].iter().cycle();
    while bytes.len() < capacity {
        bytes.push(*filler.next().unwrap());
    }
    bytes
}

/// Découpe en blocs, ajoute la correction de chaque bloc, puis entrelace.
fn add_ecc_and_interleave(gf: &Tables, data: &[u8], ver: usize, ecc: Ecc) -> Vec<u8> {
    let e = ecc.index();
    let nblocks = NUM_BLOCKS[e][ver] as usize;
    let ecc_len = ECC_PER_BLOCK[e][ver] as usize;
    let raw = raw_data_modules(ver) / 8;
    let short_len = raw / nblocks; // longueur totale (données + ecc) d'un bloc court
    let num_short = nblocks - raw % nblocks;
    let divisor = gf::generator(gf, ecc_len);

    let mut blocks: Vec<Vec<u8>> = Vec::new();
    let mut k = 0;
    for i in 0..nblocks {
        let dlen = short_len - ecc_len + if i < num_short { 0 } else { 1 };
        let chunk = &data[k..k + dlen];
        k += dlen;
        let mut b = chunk.to_vec();
        if i < num_short {
            b.push(0); // trou pour aligner avec les blocs longs, sauté plus bas
        }
        b.extend(gf::remainder(gf, chunk, &divisor));
        blocks.push(b);
    }
    let mut out = Vec::with_capacity(raw);
    for i in 0..blocks[0].len() {
        for (j, b) in blocks.iter().enumerate() {
            if i != short_len - ecc_len || j >= num_short {
                out.push(b[i]);
            }
        }
    }
    debug_assert_eq!(out.len(), raw);
    out
}

// ------------------------------------------------------------------ matrice

pub struct Qr {
    pub version: usize,
    pub ecc: Ecc,
    pub mask: usize,
    pub size: usize,
    modules: Vec<bool>,
    function: Vec<bool>,
    pub segments: Vec<Segment>,
    pub penalties: [u32; 8],
}

#[derive(Debug)]
pub enum Error {
    TooLong,
    BadVersion,
}

pub struct Options {
    pub ecc: Ecc,
    pub min_version: usize,
    pub max_version: usize,
    pub mask: Option<usize>,
    pub boost: bool,
}

impl Qr {
    pub fn encode(text: &[u8], opt: &Options) -> Result<Qr, Error> {
        if opt.min_version < 1 || opt.max_version > 40 || opt.min_version > opt.max_version {
            return Err(Error::BadVersion);
        }
        let utf8 = !text.is_ascii() && std::str::from_utf8(text).is_ok();
        let extra = if utf8 { ECI_UTF8_BITS } else { 0 };
        let mut found = None;
        for ver in opt.min_version..=opt.max_version {
            let segs = segment(text, ver);
            if let Some(used) = segments_bits(&segs, ver).map(|b| b + extra)
                && used <= data_codewords(ver, opt.ecc) * 8
            {
                found = Some((ver, segs, used));
                break;
            }
        }
        let (ver, segs, used) = found.ok_or(Error::TooLong)?;
        let mut ecc = opt.ecc;
        if opt.boost {
            // on monte le niveau de correction tant que ça tient dans la même version
            for e in [Ecc::M, Ecc::Q, Ecc::H] {
                if e.index() > ecc.index() && used <= data_codewords(ver, e) * 8 {
                    ecc = e;
                }
            }
        }
        let gf = Tables::new();
        let data = encode_segments(&segs, ver, data_codewords(ver, ecc), utf8);
        let codewords = add_ecc_and_interleave(&gf, &data, ver, ecc);

        let size = ver * 4 + 17;
        let mut qr = Qr {
            version: ver,
            ecc,
            mask: 0,
            size,
            modules: vec![false; size * size],
            function: vec![false; size * size],
            segments: segs,
            penalties: [0; 8],
        };
        qr.draw_function_patterns();
        qr.draw_codewords(&codewords);

        let mut best = (u32::MAX, 0);
        for m in 0..8 {
            qr.apply_mask(m);
            qr.draw_format(m);
            let p = qr.penalty();
            qr.penalties[m] = p;
            if p < best.0 {
                best = (p, m);
            }
            qr.apply_mask(m); // XOR : on annule
        }
        qr.mask = opt.mask.unwrap_or(best.1);
        qr.apply_mask(qr.mask);
        qr.draw_format(qr.mask);
        Ok(qr)
    }

    pub fn get(&self, x: usize, y: usize) -> bool {
        self.modules[y * self.size + x]
    }

    fn set_fn(&mut self, x: usize, y: usize, dark: bool) {
        self.modules[y * self.size + x] = dark;
        self.function[y * self.size + x] = true;
    }

    fn draw_function_patterns(&mut self) {
        let s = self.size;
        for i in 0..s {
            self.set_fn(6, i, i % 2 == 0);
            self.set_fn(i, 6, i % 2 == 0);
        }
        self.finder(3, 3);
        self.finder(s - 4, 3);
        self.finder(3, s - 4);

        let pos = alignment_positions(self.version, s);
        let n = pos.len();
        for i in 0..n {
            for j in 0..n {
                let corner = (i == 0 && j == 0) || (i == 0 && j == n - 1) || (i == n - 1 && j == 0);
                if !corner {
                    self.alignment(pos[i], pos[j]);
                }
            }
        }
        self.draw_format(0); // réserve les zones, valeurs réécrites plus tard
        self.draw_version();
    }

    fn finder(&mut self, cx: usize, cy: usize) {
        for dy in -4i32..=4 {
            for dx in -4i32..=4 {
                let (x, y) = (cx as i32 + dx, cy as i32 + dy);
                if x < 0 || y < 0 || x >= self.size as i32 || y >= self.size as i32 {
                    continue;
                }
                let d = dx.abs().max(dy.abs());
                self.set_fn(x as usize, y as usize, d != 2 && d != 4);
            }
        }
    }

    fn alignment(&mut self, cx: usize, cy: usize) {
        for dy in -2i32..=2 {
            for dx in -2i32..=2 {
                let d = dx.abs().max(dy.abs());
                self.set_fn((cx as i32 + dx) as usize, (cy as i32 + dy) as usize, d != 1);
            }
        }
    }

    fn draw_format(&mut self, mask: usize) {
        let data = self.ecc.format_bits() << 3 | mask as u32;
        let mut rem = data;
        for _ in 0..10 {
            rem = (rem << 1) ^ ((rem >> 9) * 0x537);
        }
        let bits = (data << 10 | rem) ^ 0x5412;
        let bit = |i: u32| (bits >> i) & 1 == 1;
        let s = self.size;
        for i in 0..6 {
            self.set_fn(8, i, bit(i as u32));
        }
        self.set_fn(8, 7, bit(6));
        self.set_fn(8, 8, bit(7));
        self.set_fn(7, 8, bit(8));
        for i in 9..15 {
            self.set_fn(14 - i, 8, bit(i as u32));
        }
        for i in 0..8 {
            self.set_fn(s - 1 - i, 8, bit(i as u32));
        }
        for i in 8..15 {
            self.set_fn(8, s - 15 + i, bit(i as u32));
        }
        self.set_fn(8, s - 8, true); // le module toujours noir
    }

    fn draw_version(&mut self) {
        if self.version < 7 {
            return;
        }
        let mut rem = self.version as u32;
        for _ in 0..12 {
            rem = (rem << 1) ^ ((rem >> 11) * 0x1F25);
        }
        let bits = (self.version as u32) << 12 | rem;
        for i in 0..18 {
            let dark = (bits >> i) & 1 == 1;
            let a = self.size - 11 + i % 3;
            let b = i / 3;
            self.set_fn(a, b, dark);
            self.set_fn(b, a, dark);
        }
    }

    /// Placement en zigzag : colonnes de 2 modules, de droite à gauche,
    /// alternativement vers le haut et vers le bas, en sautant la colonne 6.
    fn draw_codewords(&mut self, data: &[u8]) {
        let s = self.size;
        let mut i = 0;
        let mut right = s as i32 - 1;
        while right >= 1 {
            if right == 6 {
                right = 5;
            }
            for vert in 0..s {
                for j in 0..2 {
                    let x = (right - j) as usize;
                    let upward = (right + 1) & 2 == 0;
                    let y = if upward { s - 1 - vert } else { vert };
                    if !self.function[y * s + x] && i < data.len() * 8 {
                        self.modules[y * s + x] = (data[i >> 3] >> (7 - (i & 7))) & 1 == 1;
                        i += 1;
                    }
                }
            }
            right -= 2;
        }
    }

    fn apply_mask(&mut self, mask: usize) {
        for y in 0..self.size {
            for x in 0..self.size {
                let invert = match mask {
                    0 => (x + y) % 2 == 0,
                    1 => y % 2 == 0,
                    2 => x % 3 == 0,
                    3 => (x + y) % 3 == 0,
                    4 => (x / 3 + y / 2) % 2 == 0,
                    5 => x * y % 2 + x * y % 3 == 0,
                    6 => (x * y % 2 + x * y % 3) % 2 == 0,
                    _ => ((x + y) % 2 + x * y % 3) % 2 == 0,
                };
                let i = y * self.size + x;
                if invert && !self.function[i] {
                    self.modules[i] ^= true;
                }
            }
        }
    }

    /// Les 4 règles de pénalité du standard : on garde le masque qui donne
    /// le symbole le plus « lisible » (pas de longues lignes unies, pas de
    /// faux motifs de repérage, autant de noir que de blanc).
    fn penalty(&self) -> u32 {
        let s = self.size;
        let mut p = 0u32;
        let lines = |by_row: bool| -> Vec<Vec<bool>> {
            (0..s)
                .map(|a| (0..s).map(|b| if by_row { self.get(b, a) } else { self.get(a, b) }).collect())
                .collect()
        };
        for by_row in [true, false] {
            for line in lines(by_row) {
                let mut run = 1;
                for k in 1..s {
                    if line[k] == line[k - 1] {
                        run += 1;
                    } else {
                        if run >= 5 {
                            p += 3 + (run - 5);
                        }
                        run = 1;
                    }
                }
                if run >= 5 {
                    p += 3 + (run - 5);
                }
                // 1:1:3:1:1 avec 4 modules clairs d'un côté
                const A: [bool; 11] = [true, false, true, true, true, false, true, false, false, false, false];
                for k in 0..s.saturating_sub(10) {
                    let w = &line[k..k + 11];
                    let rev: Vec<bool> = A.iter().rev().copied().collect();
                    if w == A || w == rev.as_slice() {
                        p += 40;
                    }
                }
            }
        }
        for y in 0..s - 1 {
            for x in 0..s - 1 {
                let c = self.get(x, y);
                if c == self.get(x + 1, y) && c == self.get(x, y + 1) && c == self.get(x + 1, y + 1) {
                    p += 3;
                }
            }
        }
        let dark = self.modules.iter().filter(|&&m| m).count() as u32;
        let total = (s * s) as u32;
        let k = (dark * 20).abs_diff(total * 10).div_ceil(total) - 1;
        p + k * 10
    }
}

fn alignment_positions(ver: usize, size: usize) -> Vec<usize> {
    if ver == 1 {
        return vec![];
    }
    let n = ver / 7 + 2;
    let step = if ver == 32 { 26 } else { (ver * 4 + n * 2 + 1) / (n * 2 - 2) * 2 };
    let mut v = vec![6];
    let mut pos = size - 7;
    let mut rest = Vec::new();
    for _ in 0..n - 1 {
        rest.push(pos);
        pos -= step;
    }
    rest.reverse();
    v.extend(rest);
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capacities() {
        // capacités en octets du standard pour quelques versions
        assert_eq!(data_codewords(1, Ecc::L), 19);
        assert_eq!(data_codewords(1, Ecc::H), 9);
        assert_eq!(data_codewords(10, Ecc::M), 216);
        assert_eq!(data_codewords(40, Ecc::L), 2956);
        assert_eq!(data_codewords(40, Ecc::H), 1276);
    }

    #[test]
    fn alignment_table() {
        assert_eq!(alignment_positions(2, 25), vec![6, 18]);
        assert_eq!(alignment_positions(7, 45), vec![6, 22, 38]);
        assert_eq!(alignment_positions(32, 145), vec![6, 34, 60, 86, 112, 138]);
        assert_eq!(alignment_positions(40, 177), vec![6, 30, 58, 86, 114, 142, 170]);
    }

    #[test]
    fn hello_world() {
        let segs = segment(b"HELLO WORLD", 1);
        assert_eq!(segs.len(), 1);
        assert_eq!(segs[0].mode, Mode::Alnum);
        let data = encode_segments(&segs, 1, data_codewords(1, Ecc::M), false);
        assert_eq!(data, vec![32, 91, 11, 120, 209, 114, 220, 77, 67, 64, 236, 17, 236, 17, 236, 17]);
    }

    #[test]
    fn mixed_segments() {
        // un long nombre au milieu d'un texte mérite son propre segment
        let segs = segment(b"commande 123456789012345 ok", 1);
        assert!(segs.iter().any(|s| s.mode == Mode::Numeric));
    }
}
