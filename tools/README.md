# tools/

Outils de développement du dépôt. Rien ici ne fait partie d'un crate Rust ;
ces scripts ne sont soumis à aucune des contraintes de `crates/protocol`
(`no_std`, `unsafe_code = "forbid"`, etc.).

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
