# Jeu de caractères Gen 1

Table octet → caractère utilisée par les jeux Pokémon Rouge/Bleu/Jaune
version occidentale (anglais ; français, italien et espagnol partagent le
même jeu de caractères utilisateur d'après la source) pour le nom du
dresseur, les surnoms et les noms de dresseur d'origine transportés dans le
bloc d'échange.

**Récapitulatif de confiance** : 5 des 6 entrées de ce document
(terminateur, espace, majuscules, minuscules, chiffres) sont **confirmées**
(≥ 2 sources indépendantes convergentes) dans leur totalité. La sixième
(ponctuation et caractères spéciaux) est confirmée pour la quasi-totalité
de ses octets, sauf un : `0xF2` reste **probable** (source unique,
Bulbapedia). **0 hypothèse.**

**Source principale** — Bulbapedia, [« Character encoding (Generation I) »](https://bulbapedia.bulbagarden.net/wiki/Character_encoding_(Generation_I)),
section « Character map », table « English ». Article de référence
communautaire ; les valeurs ci-dessous sont recopiées de sa table, pas
d'un désassemblage.

**Sources de recoupement** :

- nitwhiz, [« Spoofing a Pokémon (Red) Trade »](https://blog.nitwhiz.dev/posts/002-pokemon-red-trade/),
  table `textTable` (octet → chaîne), construite par l'auteur pour décoder
  les noms observés sur trafic série réel. Incomplète par endroits : pour
  plusieurs octets sans usage dans son projet (dont `0xEF` et `0xF5`,
  symboles mâle/femelle), l'auteur a mis un simple `"?"` de substitution —
  ce n'est pas une affirmation contraire à Bulbapedia, seulement une case
  non renseignée, donc ces octets-là ne comptent pas comme confirmés par
  cette source.
- kbembedded, [« Flipper-Zero-Game-Boy-Pokemon-Trading »](https://github.com/kbembedded/Flipper-Zero-Game-Boy-Pokemon-Trading),
  fichier `pokemon_char_encode.h` : table de macros octet → caractère
  (`TERM_`, `SPACE_`, `A_`…`Z_`, `a_`…`z_`, `_0_`…`_9_`, ponctuation),
  utilisée par une application qui échange réellement avec une cartouche
  physique. Couvre notamment `MALE_ 0xef` et `FEMALE_ 0xf5`, comblant le
  trou laissé par nitwhiz sur ces deux octets.

## Terminateur

- **Ce que c'est** — octet qui marque la fin d'une chaîne dans un champ de
  longueur fixe ; le reste du champ est du remplissage sans signification
  mais conservé tel quel à la lecture.
- **Valeur / disposition** — `0x50`.
- **Source** — Bulbapedia, section « Control characters » : « `0x50` |
  Print control | String terminator | Used as a string terminator. For
  strings in fixed length fields, it is often used to pad shorter strings
  to the required length. » kbembedded/Flipper-Zero-Game-Boy-Pokemon-Trading,
  `#define TERM_ 0x50`, vérifié sur matériel réel. (nitwhiz ne liste pas cet
  octet dans sa table de caractères imprimables, puisqu'il le traite à part
  comme terminateur plutôt que comme entrée de la table.)
- **Confiance** — confirmée.

## Espace

- **Ce que c'est** — caractère espace, utilisé notamment pour remplir un nom
  volontairement vide (une chaîne ne commençant que par des espaces avant
  le terminateur).
- **Valeur / disposition** — `0x7F`.
- **Source** — Bulbapedia, section « 0x60-0x7F » : « 0x7F is a space. »
  nitwhiz, `0x7F : " "`. kbembedded/Flipper-Zero-Game-Boy-Pokemon-Trading,
  `#define SPACE_ 0x7f`.
- **Confiance** — confirmée.

## Majuscules A–Z

- **Ce que c'est** — lettres majuscules de l'alphabet latin.
- **Valeur / disposition** — `0x80` = 'A' … `0x99` = 'Z', consécutif
  (`0x80`–`0x8F` = A–P, `0x90`–`0x99` = Q–Z).
- **Source** — Bulbapedia, table « English », lignes `8-` et `9-`. nitwhiz,
  `0x80`–`0x99` dans `textTable`.
  kbembedded/Flipper-Zero-Game-Boy-Pokemon-Trading, macros `A_ 0x80` …
  `Z_ 0x99`, consécutives.
- **Confiance** — confirmée.

## Minuscules a–z

- **Ce que c'est** — lettres minuscules de l'alphabet latin.
- **Valeur / disposition** — `0xA0` = 'a' … `0xB9` = 'z', consécutif
  (`0xA0`–`0xAF` = a–p, `0xB0`–`0xB9` = q–z).
- **Source** — Bulbapedia, table « English », lignes `A-` et `B-`. nitwhiz,
  `0xA0`–`0xB9` dans `textTable`.
  kbembedded/Flipper-Zero-Game-Boy-Pokemon-Trading, macros `a_ 0xa0` …
  `z_ 0xb9`, consécutives.
- **Confiance** — confirmée.

## Chiffres 0–9

- **Ce que c'est** — chiffres décimaux.
- **Valeur / disposition** — `0xF6` = '0' … `0xFF` = '9', consécutif.
- **Source** — Bulbapedia, table « English », ligne `F-`, colonnes `-6` à
  `-F`. nitwhiz, `0xF6`–`0xFF` dans `textTable`.
  kbembedded/Flipper-Zero-Game-Boy-Pokemon-Trading, macros `_0_ 0xf6` …
  `_9_ 0xff`.
- **Confiance** — confirmée.

## Ponctuation et caractères spéciaux (non exhaustif, hors terminateur/espace)

Ces caractères n'ont pas de rôle structurel dans le bloc d'échange
(contrairement au terminateur), mais peuvent apparaître dans un nom saisi
par un joueur. Listés ici pour compléter la table ; non utilisés par le
code de la tâche 2 au-delà du terminateur et de l'espace.

| Octet | Caractère | Recoupé par | Octet | Caractère | Recoupé par |
|---|---|---|---|---|---|
| 0x9A | ( | nitwhiz, Flipper | 0x9B | ) | nitwhiz, Flipper |
| 0x9C | : | nitwhiz, Flipper | 0x9D | ; | nitwhiz, Flipper |
| 0x9E | [ | nitwhiz, Flipper | 0x9F | ] | nitwhiz, Flipper |
| 0xBA | é | nitwhiz, Flipper | 0xE0 | ' (apostrophe) | nitwhiz, Flipper |
| 0xE1 | PK (abréviation Poké) | nitwhiz, Flipper | 0xE2 | MN (abréviation Mon) | nitwhiz, Flipper |
| 0xE3 | - (tiret) | nitwhiz, Flipper | 0xE6 | ? | nitwhiz, Flipper |
| 0xE7 | ! | nitwhiz, Flipper | 0xE8 | . (point) | nitwhiz, Flipper |
| 0xEF | ♂ (symbole mâle) | Flipper (nitwhiz laisse un `"?"` de substitution) | 0xF0 | symbole Pokédollar | nitwhiz, Flipper |
| 0xF1 | × | nitwhiz, Flipper | 0xF2 | . (point, variante) | *(aucun — voir ci-dessous)* |
| 0xF3 | / | nitwhiz, Flipper | 0xF4 | , | nitwhiz, Flipper |
| 0xF5 | ♀ (symbole femelle) | Flipper (nitwhiz laisse un `"?"` de substitution) | | | |

- **Source** — Bulbapedia, table « English » (lignes `9-`, `B-`, `E-`,
  `F-`) et section « 0x80-0xFF » pour les remarques sur 0xE8/0xF2, pour
  toutes les entrées. Colonne « Recoupé par » : seconde source indépendante
  qui porte le même octet sur la même valeur (voir tables `textTable` de
  nitwhiz et macros de kbembedded/Flipper-Zero-Game-Boy-Pokemon-Trading).
  Les ligatures `'d`, `'l`, `'s`, `'t`, `'v` (0xBB–0xBF) et `'r`, `'m`
  (0xE4, 0xE5), qui compressent une apostrophe et une lettre dans un seul
  octet, sont mentionnées par Bulbapedia mais omises du tableau ci-dessus
  car sans usage prévu dans ce projet.
- **Confiance** — confirmée pour toutes les entrées sauf `0xF2` : cet octet
  n'apparaît que dans la table Bulbapedia (absent des deux sources de
  recoupement), et la source elle-même le présente comme une variante
  peu distincte de `0xE8` (point, décalé d'un pixel), utilisée seulement en
  saisie utilisateur — un cas qui ne concerne pas ce projet, qui ne fait
  que lire des noms déjà stockés. `0xF2` reste donc en confiance
  **probable** (source unique : Bulbapedia).

## Ce qui n'est pas couvert ici

- **Octets 0x00–0x4F et 0x51–0x5F** — caractères de contrôle exécutant du
  code plutôt qu'affichant un caractère (fin de ligne, effets d'affichage,
  etc.), ou tuiles de la carte en cours dans le cas de 0x01–0x48. Sans
  usage dans ce projet, qui ne fait que lire des noms déjà saisis. Source :
  Bulbapedia, sections « 0x00-0x5F » et « Control characters ».
- **Jeux de caractères allemand, italien/espagnol, japonais** — variantes
  documentées par la même page Bulbapedia (tables « French and German »,
  « Italian and Spanish », « Japanese ») mais hors périmètre : ce projet ne
  traite que l'échange entre versions occidentales, dont le jeu de
  caractères utilisateur est identique d'une langue à l'autre d'après la
  source.
