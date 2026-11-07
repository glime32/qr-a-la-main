#!/usr/bin/env python3
"""Génère des QR codes au hasard et vérifie que zbarimg les relit à l'identique.

    cargo build --release && python3 verifier.py [nombre]
"""
import os
import random
import subprocess
import sys
import tempfile

BIN = os.path.join(os.path.dirname(os.path.abspath(__file__)), "target/release/qr")
ALNUM = "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ $%*+-./:"


def texte_au_hasard(rng, n):
    genre = rng.choice(["chiffres", "alnum", "ascii", "accents", "mixte"])
    if genre == "chiffres":
        return "".join(rng.choice("0123456789") for _ in range(n))
    if genre == "alnum":
        return "".join(rng.choice(ALNUM) for _ in range(n)).strip() or "A"
    if genre == "ascii":
        return "".join(chr(rng.randrange(33, 127)) for _ in range(n))
    if genre == "accents":
        return "".join(rng.choice("abcéèàçùôœ€ ♥漢字") for _ in range(n))
    morceaux = []
    while sum(map(len, morceaux)) < n:
        morceaux.append(rng.choice([
            "".join(rng.choice("0123456789") for _ in range(rng.randrange(1, 30))),
            "".join(rng.choice(ALNUM[:36]) for _ in range(rng.randrange(1, 20))),
            "".join(chr(rng.randrange(97, 123)) for _ in range(rng.randrange(1, 15))),
        ]))
    return "".join(morceaux)


def main():
    total = int(sys.argv[1]) if len(sys.argv) > 1 else 200
    rng = random.Random(2026)
    ok = 0
    with tempfile.TemporaryDirectory() as d:
        png = os.path.join(d, "qr.png")
        for k in range(total):
            ecc = rng.choice("LMQH")
            n = rng.choice([1, 5, 20, 80, 300, 1000])
            texte = texte_au_hasard(rng, n)
            args = [BIN, "--ecc", ecc, "--png", png, "--echelle", "3", "--", texte]
            if rng.random() < 0.3:
                args[1:1] = ["--masque", str(rng.randrange(8))]
            r = subprocess.run(args, capture_output=True, text=True)
            if "trop long" in r.stderr:
                total -= 1  # normal : ça ne rentre pas, même en version 40
                continue
            if r.returncode:
                print(f"  refusé ({ecc}, {len(texte)} car.) : {r.stderr.strip()}")
                continue
            lu = subprocess.run(["zbarimg", "-q", "--raw", png], capture_output=True).stdout.decode("utf-8", "replace")
            if lu.rstrip("\n") == texte:
                ok += 1
            else:
                print(f"  ÉCHEC ecc {ecc}, {len(texte)} caractères : {texte[:40]!r}")
        # et chacune des 40 versions, avec les 4 niveaux de correction
        for ver in range(1, 41):
            for ecc in "LMQH":
                total += 1
                texte = "".join(rng.choice(ALNUM[:36]) for _ in range(6))  # tient même en 1-H
                subprocess.run([BIN, "--version", str(ver), "--ecc", ecc, "--png", png, "--echelle", "2", "--", texte], check=True)
                lu = subprocess.run(["zbarimg", "-q", "--raw", png], capture_output=True).stdout.decode("utf-8", "replace")
                if lu.rstrip("\n") == texte:
                    ok += 1
                else:
                    print(f"  ÉCHEC version {ver}-{ecc}")
    print(f"{ok} / {total} relus à l'identique par zbarimg")
    sys.exit(ok != total)


if __name__ == "__main__":
    main()
