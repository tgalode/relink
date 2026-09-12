# tools/

Outils de développement du dépôt. Rien ici n'appartient au workspace Rust de
`relink` : ni le script Python, ni le banc de mesure, qui vise Xtensa et exige
une chaîne d'outils que la CI n'installe pas.

## `banc-esp32/`

Banc de mesure sur matériel : le lien Game Boy se décode-t-il par interruption
sur `SCK` ? Se téléverse sur une carte ESP32 et n'exige qu'un strap entre
`GPIO25` et `GPIO18` — tout est en 3,3 V, aucun composant, aucune console.

Il est versionné pour que les chiffres de
[`docs/firmware/latence-decodage.md`](../docs/firmware/latence-decodage.md)
puissent être refaits, contestés, ou repris sur une autre carte. Une mesure
dont le montage n'est pas rejouable n'est pas une source.

### Prérequis

La chaîne Xtensa d'esp-rs, que `rustup target add` ne suffit pas à installer :
l'ESP32 original n'est pas RISC-V et son back-end vit dans un fork de LLVM.

```bash
cargo install espup espflash --locked
espup install --targets esp32
. ~/export-esp.sh          # à refaire dans chaque terminal
```

N'ajoutez pas `export-esp.sh` à votre profil : il exporte `LIBCLANG_PATH` vers
le clang d'Espressif, ce qui détournerait tout autre projet utilisant
`bindgen`.

### Les trois binaires

| Binaire | Ce qu'il fait | Straps |
|---|---|---|
| `banc` | Latence et gigue de l'ISR, en trois phases (repos, ordonnanceur, radio) | `D25→D18` |
| `echange` | Couche bit : 1024 octets par sens, comparés à une suite connue | + `D26→D19`, `D21→D27` |
| `echange-gen1` | Un échange Gen 1 complet à travers `Session`, comparé au même joué en mémoire | les trois |

### Utilisation

```bash
cd tools/banc-esp32
cargo build --release --bin echange-gen1
espflash flash --port /dev/ttyUSB0 target/xtensa-esp32-none-elf/release/echange-gen1
espflash monitor --port /dev/ttyUSB0
```

Sans le strap, le banc le dit au lieu de boucler en silence :

```
AUCUN front reçu. Strap GPIO25 → GPIO18 en place ?
```

Schéma du montage : [`docs/diagrams/banc-esp32.html`](../docs/diagrams/banc-esp32.html).

### Lire la sortie

Chaque ligne donne `min`, `moy`, `max`, **l'indice** du pire échantillon, et
le compte de ceux qui dépassent 4 µs. Deux séries sont mesurées d'affilée par
cadence, sans rien journaliser entre les deux.

L'indice et la seconde série ne sont pas de la coquetterie : c'est ce qui a
permis de distinguer une reprise à froid du cache — toujours à l'indice 0,
toujours après une ligne de journal — d'une vraie gigue d'interruption. Un
`max` élevé à l'indice 0 de la première série seulement se lit comme un
artefact ; réparti, il se lit comme de l'interférence.

## `gen_species_table.py`

Génère et vérifie la table de correspondance entre l'index interne
d'espèce Gen 1 et le numéro national du Pokédex, utilisée par
`crates/protocol/src/gen1/species.rs`.

Sert deux besoins :

1. **Générer le littéral Rust** à partir de la table déjà sourcée dans
   `docs/protocol/gen1-species-index.md`, pour éviter toute transcription
   manuelle (256 entrées, dont 151 significatives — une seule paire index
   ↔ espèce mal recopiée ne serait rattrapée par aucun test).
2. **Recouper** cette table contre une seconde source indépendante — le
   projet Flipper Zero
   [`kbembedded/Flipper-Zero-Game-Boy-Pokemon-Trading`](https://github.com/kbembedded/Flipper-Zero-Game-Boy-Pokemon-Trading),
   fichier `src/pokemon_table.c` — et rendre ce recoupement rejouable par
   quiconque, plutôt que de le laisser affirmé sans trace vérifiable.

Le fichier `pokemon_table.c` n'est jamais écrit sur disque ni versionné
dans ce dépôt : le script le télécharge en mémoire à chaque exécution de
`crosscheck` et le compare directement.

### Prérequis

Python 3.9+, bibliothèque standard uniquement (aucune dépendance à
installer). Un accès réseau sortant vers `raw.githubusercontent.com` est
nécessaire pour `crosscheck` (pas pour `extract`).

### Utilisation

Générer le littéral Rust `INDEX_TO_DEX` (sortie sur stdout, à coller dans
`crates/protocol/src/gen1/species.rs` si la table source a changé) :

```bash
python3 tools/gen_species_table.py extract
```

Recouper la table extraite contre la source Flipper Zero et afficher le
nombre d'écarts :

```bash
python3 tools/gen_species_table.py crosscheck
```

Sortie attendue à ce jour :

```
Recoupement : 151 paires comparées, 0 écart(s).
```

Le script sort avec un code non nul si des écarts sont trouvés, ce qui le
rend utilisable dans une CI si on souhaite surveiller une dérive entre les
deux sources.

### Que faire en cas d'écart

`crosscheck` liste chaque paire (numéro national, nom, index) qui diverge
entre les deux sources. Un écart ne doit pas être corrigé en silence dans
`species.rs` : il se documente dans
`docs/protocol/gen1-species-index.md`, en expliquant laquelle des deux
sources est retenue et pourquoi, puis la table est régénérée avec
`extract`.
