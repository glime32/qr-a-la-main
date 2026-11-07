# qr-a-la-main

Un générateur de QR codes en Rust, sans aucune dépendance. Je voulais comprendre ce qu'il y a vraiment dans ces petits carrés, donc tout est refait : codes de Reed-Solomon, découpage en blocs, placement des bits, masques.

![QR code vers ce dépôt](images/depot.png)

```
$ cargo build --release
$ ./target/release/qr salut
$ ./target/release/qr "https://github.com/glime32" --png lien.png
$ ./target/release/qr -e H --svg carte.svg "BEGIN:VCARD ..."
$ echo "texte long" | ./target/release/qr
```

Le QR s'affiche directement dans le terminal (deux modules par caractère avec `▀ ▄ █`), ou en PNG / SVG. Options : niveau de correction (`-e L|M|Q|H`), version et masque forcés, `--boost` pour prendre la meilleure correction qui tient dans la même taille, `--clair` pour les terminaux à fond blanc.

`-d` montre comment le symbole a été construit :

```
$ ./target/release/qr -d "Commande 4211 : 0612345678"
version 2 (25×25 modules), correction M, masque 2
  segment octets            8 caractères  « Commande »
  segment alphanumérique    8 caractères  «  4211 :  »
  segment numérique        10 caractères  « 0612345678 »
  28 octets de données disponibles
  pénalités des masques : 0:529  1:499  2:487  3:583  4:676  5:699  6:589  7:666
```

## Ce qui se passe

1. **Segments.** Un QR peut coder des chiffres (3 par 10 bits), de l'alphanumérique majuscule (2 par 11 bits) ou des octets. Le texte est découpé en segments par programmation dynamique pour minimiser le nombre total de bits (changer de mode coûte un en-tête, ça ne vaut pas toujours le coup). Pour le texte non ASCII, un en-tête ECI annonce de l'UTF-8 ; sans lui, certains lecteurs affichaient « ça » en idéogrammes.
2. **Version.** La plus petite des 40 tailles où les données tiennent, avec le niveau de correction demandé.
3. **Correction d'erreurs.** Les octets sont répartis en blocs, et chaque bloc reçoit des octets de Reed-Solomon : le reste de la division du message par un polynôme générateur, dans le corps fini GF(256). Puis les blocs sont entrelacés, pour qu'une tache sur le symbole abîme un peu chaque bloc plutôt que tout un bloc.
4. **Matrice.** On pose les motifs fixes (les trois carrés de repérage, les lignes de synchronisation, les petits carrés d'alignement, les informations de version), puis les bits de données en zigzag, deux colonnes à la fois, de droite à gauche.
5. **Masque.** Il y a 8 façons d'inverser un module sur deux, sur trois... On les essaie toutes et on garde celle qui a la plus petite pénalité selon les 4 règles du standard (longues lignes unies, carrés 2×2, faux motifs de repérage, déséquilibre noir/blanc).
6. **Format.** Le niveau de correction et le masque sont écrits deux fois, protégés par un code BCH.

## Tests

`cargo test` vérifie le corps fini et Reed-Solomon sur l'exemple classique « HELLO WORLD » en 1-M, les capacités, et la table des motifs d'alignement.

Les vraies vérifications passent par un lecteur indépendant :

```
$ python3 verifier.py 300
453 / 453 relus à l'identique par zbarimg
```

Le script génère 300 textes au hasard (chiffres, majuscules, ASCII, accents et emojis, mélanges), plus un symbole pour chacune des 40 versions et des 4 niveaux de correction, et vérifie que `zbarimg` relit exactement le même texte.

## Pas fait

Le mode kanji, les QR structurés en plusieurs symboles, les micro QR. Et pas de décodeur : c'est un autre projet, beaucoup plus dur (il faut retrouver le symbole dans une photo).
