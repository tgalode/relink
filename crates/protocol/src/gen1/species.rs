//! Correspondance entre l'index interne d'espèce et le numéro national.
//!
//! Table sourcée dans `docs/protocol/gen1-species-index.md`. Le littéral
//! `INDEX_TO_DEX` ci-dessous est généré, pas transcrit à la main : voir
//! `tools/gen_species_table.py` (commande `extract`) et `tools/README.md`.

/// Dernier numéro du Pokédex couvert par la première génération.
pub const LAST_GEN1_DEX_NUMBER: u8 = 151;

/// Table indexée par l'index interne. `0` marque un emplacement inutilisé.
///
/// Générée par `python3 tools/gen_species_table.py extract` à partir de
/// `docs/protocol/gen1-species-index.md` ; ne pas éditer à la main.
const INDEX_TO_DEX: [u8; 256] = [
    0, 112, 115, 32, 35, 21, 100, 34, 80, 2, 103, 108, 102, 88, 94, 29, // 0x00
    31, 104, 111, 131, 59, 151, 130, 90, 72, 92, 123, 120, 9, 127, 114, 0, // 0x10
    0, 58, 95, 22, 16, 79, 64, 75, 113, 67, 122, 106, 107, 24, 47, 54, // 0x20
    96, 76, 0, 126, 0, 125, 82, 109, 0, 56, 86, 50, 128, 0, 0, 0, // 0x30
    83, 48, 149, 0, 0, 0, 84, 60, 124, 146, 144, 145, 132, 52, 98, 0, // 0x40
    0, 0, 37, 38, 25, 26, 0, 0, 147, 148, 140, 141, 116, 117, 0, 0, // 0x50
    27, 28, 138, 139, 39, 40, 133, 136, 135, 134, 66, 41, 23, 46, 61, 62, // 0x60
    13, 14, 15, 0, 85, 57, 51, 49, 87, 0, 0, 10, 11, 12, 68, 0, // 0x70
    55, 97, 42, 150, 143, 129, 0, 0, 89, 0, 99, 91, 0, 101, 36, 110, // 0x80
    53, 105, 0, 93, 63, 65, 17, 18, 121, 1, 3, 73, 0, 118, 119, 0, // 0x90
    0, 0, 0, 77, 78, 19, 20, 33, 30, 74, 137, 142, 0, 81, 0, 0, // 0xA0
    4, 7, 5, 8, 6, 0, 0, 0, 0, 43, 44, 45, 69, 70, 71, 0, // 0xB0
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // 0xC0
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // 0xD0
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // 0xE0
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // 0xF0
];

/// Numéro national de l'espèce, ou `None` si l'index ne désigne aucune espèce
/// valide.
#[must_use]
pub const fn national_dex_number(species_index: u8) -> Option<u8> {
    match INDEX_TO_DEX[species_index as usize] {
        0 => None,
        dex => Some(dex),
    }
}
