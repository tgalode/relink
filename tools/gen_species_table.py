#!/usr/bin/env python3
"""Génère la table Rust `INDEX_TO_DEX` (tâche 5) et recoupe la source.

Ce script n'est pas un composant du crate `relink-protocol` : c'est un
outil de développement, utilisé une fois pour produire le littéral Rust
collé dans `crates/protocol/src/gen1/species.rs`, et rejouable ensuite pour
vérifier que ce littéral correspond toujours à sa source documentée.

Deux commandes :

  extract     Extrait la table des 151 espèces depuis
              `docs/protocol/gen1-species-index.md` et imprime le
              littéral Rust `INDEX_TO_DEX` (256 entrées, 0 = inutilisé).

  crosscheck  Télécharge `src/pokemon_table.c` du dépôt
              kbembedded/Flipper-Zero-Game-Boy-Pokemon-Trading (source 2
              du document), en extrait les 151 premières entrées (ordre
              Pokédex national, vérifié par position) et compare chaque
              paire (numéro national, index interne) à celle extraite de
              la source 1 (Bulbapedia, table du document). Affiche le
              nombre d'écarts.

Le fichier `pokemon_table.c` est téléchargé à la volée dans un répertoire
temporaire, comparé, puis jamais conservé ni versionné dans ce dépôt.

Usage :
    python3 tools/gen_species_table.py extract
    python3 tools/gen_species_table.py crosscheck
"""

from __future__ import annotations

import argparse
import re
import sys
import urllib.request
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
SOURCE_DOC = REPO_ROOT / "docs" / "protocol" / "gen1-species-index.md"

FLIPPER_TABLE_URL = (
    "https://raw.githubusercontent.com/"
    "kbembedded/Flipper-Zero-Game-Boy-Pokemon-Trading/"
    "main/src/pokemon_table.c"
)

# Nombre d'espèces de première génération, et donc de premières entrées de
# `pokemon_table[]` à retenir (voir docstring : elles sont dans l'ordre
# Pokédex national, position 1 = Bulbasaur, ..., position 151 = Mew).
GEN1_SPECIES_COUNT = 151

TABLE_ROW_RE = re.compile(
    r"^\|\s*(\d+)\s*\|\s*([^|]+?)\s*\|\s*(0[xX][0-9A-Fa-f]{1,2})\s*\|\s*(\d+)\s*\|\s*$"
)


def parse_source_table(md_path: Path) -> list[tuple[int, str, int]]:
    """Extrait (numéro national, nom, index interne) depuis le document source.

    Ne retient que les lignes de la table des 151 espèces (colonnes
    N° national / Nom / Index interne (hex) / Index interne (déc)) ; les
    autres tableaux du document (aucun à ce jour) sont ignorés par la forme
    du motif.
    """
    rows: list[tuple[int, str, int]] = []
    for line in md_path.read_text(encoding="utf-8").splitlines():
        match = TABLE_ROW_RE.match(line)
        if not match:
            continue
        dex_str, name, index_hex, index_dec = match.groups()
        dex = int(dex_str)
        index = int(index_hex, 16)
        if int(index_dec) != index:
            raise ValueError(
                f"ligne incohérente pour {name!r} : hex {index_hex} != déc {index_dec}"
            )
        rows.append((dex, name, index))
    return rows


def validate_species_table(rows: list[tuple[int, str, int]]) -> None:
    """Vérifie la complétude (1..151) et l'absence de doublon, sur les deux axes."""
    dex_numbers = [dex for dex, _, _ in rows]
    indices = [index for _, _, index in rows]

    if len(rows) != GEN1_SPECIES_COUNT:
        raise ValueError(
            f"{len(rows)} lignes extraites, {GEN1_SPECIES_COUNT} attendues"
        )
    if sorted(dex_numbers) != list(range(1, GEN1_SPECIES_COUNT + 1)):
        manquants = sorted(set(range(1, GEN1_SPECIES_COUNT + 1)) - set(dex_numbers))
        doublons = sorted({d for d in dex_numbers if dex_numbers.count(d) > 1})
        raise ValueError(
            f"numéros nationaux incomplets ou dupliqués : manquants={manquants} doublons={doublons}"
        )
    if len(set(indices)) != len(indices):
        doublons = sorted({i for i in indices if indices.count(i) > 1})
        raise ValueError(f"index internes dupliqués : {[hex(i) for i in doublons]}")
    for index in indices:
        if not (0 <= index <= 0xFF):
            raise ValueError(f"index interne hors borne u8 : {index:#x}")


def build_index_to_dex(rows: list[tuple[int, str, int]]) -> list[int]:
    """Construit le tableau de 256 entrées, indexé par l'index interne."""
    table = [0] * 256
    for dex, _name, index in rows:
        table[index] = dex
    return table


def render_rust_array(table: list[int]) -> str:
    """Rend le littéral Rust `INDEX_TO_DEX`, 16 valeurs par ligne, avec commentaire d'index."""
    lines = []
    lines.append("const INDEX_TO_DEX: [u8; 256] = [")
    for start in range(0, 256, 16):
        chunk = table[start : start + 16]
        values = ", ".join(str(v) for v in chunk)
        lines.append(f"    {values}, // 0x{start:02X}")
    lines.append("];")
    return "\n".join(lines)


def cmd_extract(_args: argparse.Namespace) -> int:
    rows = parse_source_table(SOURCE_DOC)
    validate_species_table(rows)
    table = build_index_to_dex(rows)
    print(render_rust_array(table))
    print(
        f"\n// {len(rows)} espèces extraites de {SOURCE_DOC.relative_to(REPO_ROOT)}, "
        "validées (complètes, sans doublon).",
        file=sys.stderr,
    )
    return 0


# --- Recoupement avec la source 2 (Flipper Zero) --------------------------

FLIPPER_ENTRY_RE = re.compile(
    r'\{"([^"]+)"\s*,\s*(0[xX][0-9A-Fa-f]{1,2})\s*,', re.MULTILINE
)


def fetch_flipper_table_source(url: str) -> str:
    with urllib.request.urlopen(url, timeout=30) as response:  # noqa: S310
        return response.read().decode("utf-8")


def parse_flipper_first_n(source: str, n: int) -> list[tuple[int, str, int]]:
    """Extrait les `n` premières entrées de `pokemon_table[]` : (position, nom, index).

    La position (1-based) sert de numéro national : les `n` premières
    entrées de `pokemon_table[]` sont, dans ce fichier, exactement les
    espèces de première génération dans l'ordre du Pokédex national
    (Bulbasaur en position 1, ..., Mew en position 151), avant que la
    table n'enchaîne sur la génération II.
    """
    start = source.index("pokemon_table[] = {")
    matches = FLIPPER_ENTRY_RE.findall(source[start:])
    if len(matches) < n:
        raise ValueError(f"seulement {len(matches)} entrées trouvées, {n} attendues")
    return [
        (position, name, int(index_hex, 16))
        for position, (name, index_hex) in enumerate(matches[:n], start=1)
    ]


def cmd_crosscheck(args: argparse.Namespace) -> int:
    rows = parse_source_table(SOURCE_DOC)
    validate_species_table(rows)
    source1 = {dex: (name, index) for dex, name, index in rows}

    print(f"Téléchargement de {args.url} ...", file=sys.stderr)
    flipper_source = fetch_flipper_table_source(args.url)
    flipper_rows = parse_flipper_first_n(flipper_source, GEN1_SPECIES_COUNT)
    source2 = {dex: (name, index) for dex, name, index in flipper_rows}

    ecarts = []
    for dex in range(1, GEN1_SPECIES_COUNT + 1):
        name1, index1 = source1[dex]
        name2, index2 = source2[dex]
        if index1 != index2:
            ecarts.append(
                f"  n°{dex}: Bulbapedia={name1!r} index={index1:#04x} "
                f"vs Flipper={name2!r} index={index2:#04x}"
            )

    print(f"Recoupement : {GEN1_SPECIES_COUNT} paires comparées, {len(ecarts)} écart(s).")
    for ligne in ecarts:
        print(ligne)

    return 1 if ecarts else 0


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="command", required=True)

    p_extract = sub.add_parser(
        "extract", help="Imprime le littéral Rust INDEX_TO_DEX sur stdout."
    )
    p_extract.set_defaults(func=cmd_extract)

    p_crosscheck = sub.add_parser(
        "crosscheck",
        help="Télécharge la source 2 et compare les 151 paires index/espèce.",
    )
    p_crosscheck.add_argument(
        "--url", default=FLIPPER_TABLE_URL, help="URL brute de pokemon_table.c"
    )
    p_crosscheck.set_defaults(func=cmd_crosscheck)

    args = parser.parse_args()
    return args.func(args)


if __name__ == "__main__":
    raise SystemExit(main())
