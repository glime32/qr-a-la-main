mod gf;
mod qr;
mod render;

use qr::{Ecc, Options, Qr};
use std::io::Read;
use std::process::exit;

const AIDE: &str = "\
qr — génère un QR code, sans bibliothèque

usage : qr [options] [texte]        (sans texte : lit l'entrée standard)

  -e, --ecc L|M|Q|H     niveau de correction (défaut M : ~15 % du symbole peut être abîmé)
  -v, --version N       forcer la version (1 à 40)
  -m, --masque N        forcer le masque (0 à 7)
      --boost           monter la correction si ça tient dans la même version
      --png FICHIER     image PNG
      --svg FICHIER     image SVG
      --echelle N       taille d'un module en pixels pour le PNG (défaut 8)
      --clair           pour un terminal à fond clair
  -d, --details         montrer comment le symbole a été construit
";

fn fail(msg: &str) -> ! {
    eprintln!("qr : {msg}");
    exit(1)
}

fn main() {
    let mut args = std::env::args().skip(1);
    let mut opt = Options { ecc: Ecc::M, min_version: 1, max_version: 40, mask: None, boost: false };
    let (mut png, mut svg, mut scale, mut light, mut details) = (None, None, 8, false, false);
    let mut text: Option<String> = None;

    while let Some(a) = args.next() {
        let mut value = |name: &str| args.next().unwrap_or_else(|| fail(&format!("{name} attend une valeur")));
        match a.as_str() {
            "-h" | "--aide" | "--help" => {
                print!("{AIDE}");
                return;
            }
            "-e" | "--ecc" => opt.ecc = Ecc::parse(&value(&a)).unwrap_or_else(|| fail("niveau inconnu (L, M, Q ou H)")),
            "-v" | "--version" => {
                let v: usize = value(&a).parse().unwrap_or(0);
                if !(1..=40).contains(&v) {
                    fail("la version va de 1 à 40");
                }
                opt.min_version = v;
                opt.max_version = v;
            }
            "-m" | "--masque" => {
                let m: usize = value(&a).parse().unwrap_or(99);
                if m > 7 {
                    fail("le masque va de 0 à 7");
                }
                opt.mask = Some(m);
            }
            "--boost" => opt.boost = true,
            "--png" => png = Some(value(&a)),
            "--svg" => svg = Some(value(&a)),
            "--echelle" => scale = value(&a).parse().ok().filter(|&s| s > 0 && s <= 64).unwrap_or_else(|| fail("échelle entre 1 et 64")),
            "--clair" => light = true,
            "-d" | "--details" => details = true,
            _ if a.starts_with('-') && a.len() > 1 => fail(&format!("option inconnue {a}")),
            _ => text = Some(match text {
                None => a,
                Some(t) => t + " " + &a,
            }),
        }
    }

    let text = text.unwrap_or_else(|| {
        let mut s = String::new();
        std::io::stdin().read_to_string(&mut s).unwrap_or_else(|e| fail(&e.to_string()));
        s.strip_suffix('\n').unwrap_or(&s).to_string()
    });

    let qr = match Qr::encode(text.as_bytes(), &opt) {
        Ok(q) => q,
        Err(qr::Error::TooLong) => fail("texte trop long pour ces réglages (2953 octets maximum en version 40-L)"),
        Err(qr::Error::BadVersion) => fail("version invalide"),
    };

    if details {
        eprintln!("version {} ({}×{} modules), correction {:?}, masque {}", qr.version, qr.size, qr.size, qr.ecc, qr.mask);
        for s in &qr.segments {
            let preview: String = String::from_utf8_lossy(&s.data).chars().take(30).collect();
            eprintln!("  segment {:<14} {:>4} caractères  « {} »", s.mode.name(), s.data.len(), preview);
        }
        let cap = qr::data_codewords(qr.version, qr.ecc);
        eprintln!("  {cap} octets de données disponibles");
        let p: Vec<String> = qr.penalties.iter().enumerate().map(|(i, p)| format!("{i}:{p}")).collect();
        eprintln!("  pénalités des masques : {}", p.join("  "));
    }

    if let Some(path) = &png {
        render::write_file(path, &render::png(&qr, scale)).unwrap_or_else(|e| fail(&format!("{path} : {e}")));
    }
    if let Some(path) = &svg {
        render::write_file(path, render::svg(&qr).as_bytes()).unwrap_or_else(|e| fail(&format!("{path} : {e}")));
    }
    if png.is_none() && svg.is_none() {
        print!("{}", render::terminal(&qr, light));
    }
}
