# Jeu de caractères Gen 1

Table octet → caractère utilisée par les jeux Pokémon Rouge/Bleu/Jaune
version occidentale (anglais ; français, italien et espagnol partagent le
même jeu de caractères utilisateur d'après la source) pour le nom du
dresseur, les surnoms et les noms de dresseur d'origine transportés dans le
bloc d'échange.

**Source principale** — Bulbapedia, [« Character encoding (Generation I) »](https://bulbapedia.bulbagarden.net/wiki/Character_encoding_(Generation_I)),
section « Character map », table « English ». Article de référence
communautaire ; les valeurs ci-dessous sont recopiées de sa table, pas
d'un désassemblage.

**Source de recoupement** — nitwhiz, [« Spoofing a Pokémon (Red) Trade »](https://blog.nitwhiz.dev/posts/002-pokemon-red-trade/),
table `textTable` (octet → chaîne), construite par l'auteur pour décoder les
noms observés sur trafic série réel. Utilisée uniquement pour confirmer les
valeurs ci-dessous, jamais comme source unique.

**Confiance** — confirmée pour toutes les entrées ci-dessous : les deux
sources s'accordent sur chaque valeur reprise dans ce document.

## Terminateur

- **Ce que c'est** — octet qui marque la fin d'une chaîne dans un champ de
  longueur fixe ; le reste du champ est du remplissage sans signification
  mais conservé tel quel à la lecture.
- **Valeur / disposition** — `0x50`.
- **Source** — Bulbapedia, section « Control characters » : « `0x50` |
  Print control | String terminator | Used as a string terminator. For
  strings in fixed length fields, it is often used to pad shorter strings
  to the required length. »
- **Confiance** — confirmée.

## Espace

- **Ce que c'est** — caractère espace, utilisé notamment pour remplir un nom
  volontairement vide (une chaîne ne commençant que par des espaces avant
  le terminateur).
- **Valeur / disposition** — `0x7F`.
- **Source** — Bulbapedia, section « 0x60-0x7F » : « 0x7F is a space. »
- **Confiance** — confirmée.

## Majuscules A–Z

- **Ce que c'est** — lettres majuscules de l'alphabet latin.
- **Valeur / disposition** — `0x80` = 'A' … `0x99` = 'Z', consécutif
  (`0x80`–`0x8F` = A–P, `0x90`–`0x99` = Q–Z).
- **Source** — Bulbapedia, table « English », lignes `8-` et `9-` (colonnes
  `-0` à `-F` et `-0` à `-9`).
- **Confiance** — confirmée.

## Minuscules a–z

- **Ce que c'est** — lettres minuscules de l'alphabet latin.
- **Valeur / disposition** — `0xA0` = 'a' … `0xB9` = 'z', consécutif
  (`0xA0`–`0xAF` = a–p, `0xB0`–`0xB9` = q–z).
- **Source** — Bulbapedia, table « English », lignes `A-` et `B-`.
- **Confiance** — confirmée.

## Chiffres 0–9

- **Ce que c'est** — chiffres décimaux.
- **Valeur / disposition** — `0xF6` = '0' … `0xFF` = '9', consécutif.
- **Source** — Bulbapedia, table « English », ligne `F-`, colonnes `-6` à
  `-F`.
- **Confiance** — confirmée.

## Ponctuation et caractères spéciaux (non exhaustif, hors terminateur/espace)

Ces caractères n'ont pas de rôle structurel dans le bloc d'échange
(contrairement au terminateur), mais peuvent apparaître dans un nom saisi
par un joueur. Listés ici pour compléter la table ; non utilisés par le
code de la tâche 2 au-delà du terminateur et de l'espace.

| Octet | Caractère | Octet | Caractère |
|---|---|---|---|
| 0x9A | ( | 0x9B | ) |
| 0x9C | : | 0x9D | ; |
| 0x9E | [ | 0x9F | ] |
| 0xBA | é | 0xE0 | ' (apostrophe) |
| 0xE1 | PK (abréviation Poké) | 0xE2 | MN (abréviation Mon) |
| 0xE3 | - (tiret) | 0xE6 | ? |
| 0xE7 | ! | 0xE8 | . (point) |
| 0xEF | ♂ (symbole mâle) | 0xF0 | symbole Pokédollar |
| 0xF1 | × | 0xF2 | . (point, variante) |
| 0xF3 | / | 0xF4 | , |
| 0xF5 | ♀ (symbole femelle) | | |

- **Source** — Bulbapedia, table « English » (lignes `9-`, `B-`, `E-`,
  `F-`) et section « 0x80-0xFF » pour les remarques sur 0xE8/0xF2. Les
  ligatures `'d`, `'l`, `'s`, `'t`, `'v` (0xBB–0xBF) et `'r`, `'m` (0xE4,
  0xE5), qui compressent une apostrophe et une lettre dans un seul octet,
  sont mentionnées par Bulbapedia mais omises du tableau ci-dessus car
  sans usage prévu dans ce projet.
- **Confiance** — confirmée.

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
