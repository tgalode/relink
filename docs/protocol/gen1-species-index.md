# Correspondance index interne → numéro national (Gen 1)

L'octet d'espèce stocké dans les 44 octets d'un Pokémon d'équipe (décalage
0x00, voir `gen1-trade-block.md`) est un **index interne**, propre à
l'ordre de programmation du jeu, distinct du numéro national du Pokédex.
Cette table établit la correspondance pour les 151 espèces de première
génération, plus les index inutilisés.

Note de planification : cette table est produite en tâche 1 bien que le
plan initial la place en tâche 5 — elle repose sur exactement le même
travail de sourçage que le reste de ce document, et rien dans les tâches 2
à 6 ne peut être vérifié sans elle.

## Source

- **Ce que c'est** — correspondance complète entre l'index interne d'une
  espèce (tel que stocké dans le bloc d'échange) et son numéro national.
- **Source** — Bulbapedia, [« List of Pokémon by index number in
  Generation I »](https://bulbapedia.bulbagarden.net/wiki/List_of_Pok%C3%A9mon_by_index_number_in_Generation_I),
  section « List of Pokémon by index number », table complète des 190
  index non-nuls (1 à 190) avec leur numéro national. Article de référence
  communautaire, aucun désassemblage cité.
- **Confiance** — confirmée. La table extraite couvre exactement les 151
  numéros nationaux de 1 à 151 une seule fois chacun (vérifié par script à
  partir du wikitexte de la page), ce qui correspond à l'invariant que le
  code de la tâche 5 doit vérifier.

## Divergence trouvée avec le plan

**Le plan (`docs/superpowers/plans/2026-08-27-codecs-gen1.md`, tâches 3, 5
et 6) affirme à plusieurs endroits que l'index interne `0x15` correspond à
Bulbizarre (n° 1) et que `0x14` correspond à Mew (n° 151). C'est inversé et
faux dans les deux cas :**

| Index interne | Le plan affirmait | La source dit |
|---|---|---|
| `0x15` (21 déc.) | Bulbizarre (n° 1) | **Mew (n° 151)** |
| `0x14` (20 déc.) | Mew (n° 151) | **Arcanine (n° 59)** |
| — | — | Bulbizarre (n° 1) est en réalité à l'index `0x99` (153 déc.) |

C'est un fait bien documenté de la genèse Gen 1 : Mew a été inséré tôt dans
la liste des espèces (index interne 21), ce qui est à l'origine du
« glitch Mew » ; Bulbizarre, en tête du Pokédex national, se retrouve très
loin dans l'ordre interne de programmation, à l'index 153.

Ce document corrige `docs/superpowers/plans/2026-08-27-codecs-gen1.md` en
conséquence (fixtures des tâches 3 et 6, assertions de la tâche 5) — voir
le rapport de tâche pour le détail des lignes modifiées.

## Table complète (151 espèces)

| N° national | Nom | Index interne (hex) | Index interne (déc) |
|---|---|---|---|
| 1 | Bulbasaur | 0x99 | 153 |
| 2 | Ivysaur | 0x09 | 9 |
| 3 | Venusaur | 0x9A | 154 |
| 4 | Charmander | 0xB0 | 176 |
| 5 | Charmeleon | 0xB2 | 178 |
| 6 | Charizard | 0xB4 | 180 |
| 7 | Squirtle | 0xB1 | 177 |
| 8 | Wartortle | 0xB3 | 179 |
| 9 | Blastoise | 0x1C | 28 |
| 10 | Caterpie | 0x7B | 123 |
| 11 | Metapod | 0x7C | 124 |
| 12 | Butterfree | 0x7D | 125 |
| 13 | Weedle | 0x70 | 112 |
| 14 | Kakuna | 0x71 | 113 |
| 15 | Beedrill | 0x72 | 114 |
| 16 | Pidgey | 0x24 | 36 |
| 17 | Pidgeotto | 0x96 | 150 |
| 18 | Pidgeot | 0x97 | 151 |
| 19 | Rattata | 0xA5 | 165 |
| 20 | Raticate | 0xA6 | 166 |
| 21 | Spearow | 0x05 | 5 |
| 22 | Fearow | 0x23 | 35 |
| 23 | Ekans | 0x6C | 108 |
| 24 | Arbok | 0x2D | 45 |
| 25 | Pikachu | 0x54 | 84 |
| 26 | Raichu | 0x55 | 85 |
| 27 | Sandshrew | 0x60 | 96 |
| 28 | Sandslash | 0x61 | 97 |
| 29 | Nidoran♀ | 0x0F | 15 |
| 30 | Nidorina | 0xA8 | 168 |
| 31 | Nidoqueen | 0x10 | 16 |
| 32 | Nidoran♂ | 0x03 | 3 |
| 33 | Nidorino | 0xA7 | 167 |
| 34 | Nidoking | 0x07 | 7 |
| 35 | Clefairy | 0x04 | 4 |
| 36 | Clefable | 0x8E | 142 |
| 37 | Vulpix | 0x52 | 82 |
| 38 | Ninetales | 0x53 | 83 |
| 39 | Jigglypuff | 0x64 | 100 |
| 40 | Wigglytuff | 0x65 | 101 |
| 41 | Zubat | 0x6B | 107 |
| 42 | Golbat | 0x82 | 130 |
| 43 | Oddish | 0xB9 | 185 |
| 44 | Gloom | 0xBA | 186 |
| 45 | Vileplume | 0xBB | 187 |
| 46 | Paras | 0x6D | 109 |
| 47 | Parasect | 0x2E | 46 |
| 48 | Venonat | 0x41 | 65 |
| 49 | Venomoth | 0x77 | 119 |
| 50 | Diglett | 0x3B | 59 |
| 51 | Dugtrio | 0x76 | 118 |
| 52 | Meowth | 0x4D | 77 |
| 53 | Persian | 0x90 | 144 |
| 54 | Psyduck | 0x2F | 47 |
| 55 | Golduck | 0x80 | 128 |
| 56 | Mankey | 0x39 | 57 |
| 57 | Primeape | 0x75 | 117 |
| 58 | Growlithe | 0x21 | 33 |
| 59 | Arcanine | 0x14 | 20 |
| 60 | Poliwag | 0x47 | 71 |
| 61 | Poliwhirl | 0x6E | 110 |
| 62 | Poliwrath | 0x6F | 111 |
| 63 | Abra | 0x94 | 148 |
| 64 | Kadabra | 0x26 | 38 |
| 65 | Alakazam | 0x95 | 149 |
| 66 | Machop | 0x6A | 106 |
| 67 | Machoke | 0x29 | 41 |
| 68 | Machamp | 0x7E | 126 |
| 69 | Bellsprout | 0xBC | 188 |
| 70 | Weepinbell | 0xBD | 189 |
| 71 | Victreebel | 0xBE | 190 |
| 72 | Tentacool | 0x18 | 24 |
| 73 | Tentacruel | 0x9B | 155 |
| 74 | Geodude | 0xA9 | 169 |
| 75 | Graveler | 0x27 | 39 |
| 76 | Golem | 0x31 | 49 |
| 77 | Ponyta | 0xA3 | 163 |
| 78 | Rapidash | 0xA4 | 164 |
| 79 | Slowpoke | 0x25 | 37 |
| 80 | Slowbro | 0x08 | 8 |
| 81 | Magnemite | 0xAD | 173 |
| 82 | Magneton | 0x36 | 54 |
| 83 | Farfetch'd | 0x40 | 64 |
| 84 | Doduo | 0x46 | 70 |
| 85 | Dodrio | 0x74 | 116 |
| 86 | Seel | 0x3A | 58 |
| 87 | Dewgong | 0x78 | 120 |
| 88 | Grimer | 0x0D | 13 |
| 89 | Muk | 0x88 | 136 |
| 90 | Shellder | 0x17 | 23 |
| 91 | Cloyster | 0x8B | 139 |
| 92 | Gastly | 0x19 | 25 |
| 93 | Haunter | 0x93 | 147 |
| 94 | Gengar | 0x0E | 14 |
| 95 | Onix | 0x22 | 34 |
| 96 | Drowzee | 0x30 | 48 |
| 97 | Hypno | 0x81 | 129 |
| 98 | Krabby | 0x4E | 78 |
| 99 | Kingler | 0x8A | 138 |
| 100 | Voltorb | 0x06 | 6 |
| 101 | Electrode | 0x8D | 141 |
| 102 | Exeggcute | 0x0C | 12 |
| 103 | Exeggutor | 0x0A | 10 |
| 104 | Cubone | 0x11 | 17 |
| 105 | Marowak | 0x91 | 145 |
| 106 | Hitmonlee | 0x2B | 43 |
| 107 | Hitmonchan | 0x2C | 44 |
| 108 | Lickitung | 0x0B | 11 |
| 109 | Koffing | 0x37 | 55 |
| 110 | Weezing | 0x8F | 143 |
| 111 | Rhyhorn | 0x12 | 18 |
| 112 | Rhydon | 0x01 | 1 |
| 113 | Chansey | 0x28 | 40 |
| 114 | Tangela | 0x1E | 30 |
| 115 | Kangaskhan | 0x02 | 2 |
| 116 | Horsea | 0x5C | 92 |
| 117 | Seadra | 0x5D | 93 |
| 118 | Goldeen | 0x9D | 157 |
| 119 | Seaking | 0x9E | 158 |
| 120 | Staryu | 0x1B | 27 |
| 121 | Starmie | 0x98 | 152 |
| 122 | Mr. Mime | 0x2A | 42 |
| 123 | Scyther | 0x1A | 26 |
| 124 | Jynx | 0x48 | 72 |
| 125 | Electabuzz | 0x35 | 53 |
| 126 | Magmar | 0x33 | 51 |
| 127 | Pinsir | 0x1D | 29 |
| 128 | Tauros | 0x3C | 60 |
| 129 | Magikarp | 0x85 | 133 |
| 130 | Gyarados | 0x16 | 22 |
| 131 | Lapras | 0x13 | 19 |
| 132 | Ditto | 0x4C | 76 |
| 133 | Eevee | 0x66 | 102 |
| 134 | Vaporeon | 0x69 | 105 |
| 135 | Jolteon | 0x68 | 104 |
| 136 | Flareon | 0x67 | 103 |
| 137 | Porygon | 0xAA | 170 |
| 138 | Omanyte | 0x62 | 98 |
| 139 | Omastar | 0x63 | 99 |
| 140 | Kabuto | 0x5A | 90 |
| 141 | Kabutops | 0x5B | 91 |
| 142 | Aerodactyl | 0xAB | 171 |
| 143 | Snorlax | 0x84 | 132 |
| 144 | Articuno | 0x4A | 74 |
| 145 | Zapdos | 0x4B | 75 |
| 146 | Moltres | 0x49 | 73 |
| 147 | Dratini | 0x58 | 88 |
| 148 | Dragonair | 0x59 | 89 |
| 149 | Dragonite | 0x42 | 66 |
| 150 | Mewtwo | 0x83 | 131 |
| 151 | Mew | 0x15 | 21 |

- **Source** — Bulbapedia, page citée en tête de document.
- **Confiance** — confirmée.

## Index inutilisés

- **Ce que c'est** — index internes qui ne désignent aucune des 151 espèces
  de première génération (« MissingNo. » et emplacements glitchés).
- **Valeur / disposition** — 105 index sur 256 :
  - `0x00` (index 0) ;
  - 39 index dans la plage 1–190, listés par la source comme
    « MissingNo. » : `0x1F`, `0x20`, `0x32`, `0x34`, `0x38`, `0x3D`–`0x3F`,
    `0x43`–`0x45`, `0x4F`–`0x51`, `0x56`–`0x57`, `0x5E`–`0x5F`, `0x73`,
    `0x79`–`0x7A`, `0x7F`, `0x86`–`0x87`, `0x89`, `0x8C`, `0x92`, `0x9C`,
    `0x9F`–`0xA2`, `0xAC`, `0xAE`–`0xAF`, `0xB5`–`0xB8` ;
  - `0xBF`–`0xFF` (index 191 à 255), non couverts individuellement par la
    table (la source les décrit collectivement comme des Pokémon glitch,
    sans lister leur contenu un par un).
- **Source** — Bulbapedia, même page : « Index 0 and indices from 191 to
  255 (hex 0xFF) contain all other glitch Pokémon in Generation I. » pour
  les deux plages hors table ; les 39 index à l'intérieur de la table
  portent explicitement le nom « MissingNo. » avec un numéro national de
  0 (absence de correspondance).
- **Confiance** — confirmée pour les 40 index listés individuellement
  (`0x00` et les 39 de la plage 1–190) ; confirmée mais non détaillée
  individuellement pour la plage `0xBF`–`0xFF` (la source affirme
  collectivement qu'elle ne contient aucune des 151 espèces, sans
  distinguer les 65 index un par un — suffisant pour l'usage de ce projet,
  qui n'a besoin que de savoir qu'aucun ne traduit une espèce valide).

L'index `0x1F`, cité par le plan comme exemple d'index inutilisé, est bien
confirmé inutilisé par cette table — c'est la seule des trois affirmations
du plan sur les espèces qui n'avait pas besoin de correction.
