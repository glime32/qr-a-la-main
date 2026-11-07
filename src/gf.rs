// Arithmétique dans GF(256) avec le polynôme du standard QR : x^8 + x^4 + x^3 + x^2 + 1.

const POLY: u16 = 0x11D;

pub struct Tables {
    exp: [u8; 512],
    log: [u8; 256],
}

impl Tables {
    pub fn new() -> Self {
        let mut exp = [0u8; 512];
        let mut log = [0u8; 256];
        let mut x: u16 = 1;
        for i in 0..255 {
            exp[i] = x as u8;
            log[x as usize] = i as u8;
            x <<= 1;
            if x & 0x100 != 0 {
                x ^= POLY;
            }
        }
        // on double la table pour éviter un modulo dans mul()
        for i in 255..512 {
            exp[i] = exp[i - 255];
        }
        Tables { exp, log }
    }

    pub fn mul(&self, a: u8, b: u8) -> u8 {
        if a == 0 || b == 0 {
            return 0;
        }
        self.exp[self.log[a as usize] as usize + self.log[b as usize] as usize]
    }

    pub fn pow2(&self, n: usize) -> u8 {
        self.exp[n % 255]
    }
}

/// Polynôme générateur de degré `degree` : produit des (x - 2^i), i = 0..degree.
/// Coefficients du plus haut degré au plus bas, sans le 1 de tête.
pub fn generator(gf: &Tables, degree: usize) -> Vec<u8> {
    let mut g = vec![0u8; degree];
    g[degree - 1] = 1;
    for i in 0..degree {
        let root = gf.pow2(i);
        for j in 0..degree {
            g[j] = gf.mul(g[j], root);
            if j + 1 < degree {
                g[j] ^= g[j + 1];
            }
        }
    }
    g
}

/// Reste de la division de data·x^n par le générateur : les n octets de correction.
pub fn remainder(gf: &Tables, data: &[u8], divisor: &[u8]) -> Vec<u8> {
    let mut rem = vec![0u8; divisor.len()];
    for &b in data {
        let factor = b ^ rem[0];
        rem.rotate_left(1);
        *rem.last_mut().unwrap() = 0;
        for (r, &g) in rem.iter_mut().zip(divisor) {
            *r ^= gf.mul(g, factor);
        }
    }
    rem
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inverse_exists() {
        let gf = Tables::new();
        for a in 1..=255u8 {
            let found = (1..=255u8).filter(|&b| gf.mul(a, b) == 1).count();
            assert_eq!(found, 1, "{a} doit avoir un unique inverse");
        }
    }

    #[test]
    fn known_codeword() {
        // exemple classique (thonky.com) : "HELLO WORLD" en 1-M
        let gf = Tables::new();
        let data = [32, 91, 11, 120, 209, 114, 220, 77, 67, 64, 236, 17, 236, 17, 236, 17];
        let ecc = remainder(&gf, &data, &generator(&gf, 10));
        assert_eq!(ecc, vec![196, 35, 39, 119, 235, 215, 231, 226, 93, 23]);
    }

    #[test]
    fn codeword_divisible() {
        // data suivi de son reste doit être un multiple du générateur
        let gf = Tables::new();
        let data: Vec<u8> = (0..40).map(|i| (i * 37 + 11) as u8).collect();
        let g = generator(&gf, 18);
        let mut full = data.clone();
        full.extend(remainder(&gf, &data, &g));
        assert!(remainder(&gf, &full, &g).iter().all(|&x| x == 0));
    }
}
