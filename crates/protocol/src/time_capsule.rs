//! Règles de la Capsule Temporelle.
//!
//! Aucune conversion n'est inventée : les jeux ont déjà tranché, et ces règles
//! sont les leurs. Sourcées dans `docs/protocol/time-capsule-rules.md`.

use crate::gen1::{PartyPokemon, national_dex_number};

/// Identifiant de la dernière capacité de première génération.
pub const LAST_GEN1_MOVE_ID: u8 = 165;

/// Pourquoi un Pokémon ne peut pas descendre vers une cartouche Gen 1.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Ineligible {
    /// L'index d'espèce ne désigne aucune espèce valide.
    UnknownSpecies,
    /// L'espèce est postérieure à la première génération.
    SpeciesTooRecent {
        /// Numéro national de l'espèce fautive.
        dex: u8,
    },
    /// Une capacité est postérieure à la première génération.
    MoveTooRecent {
        /// Emplacement de capacité concerné, de 0 à 3.
        slot: u8,
        /// Identifiant de la capacité fautive.
        move_id: u8,
    },
}

/// Vérifie qu'un Pokémon peut être remis à une cartouche de première
/// génération. Le premier motif de refus rencontré est rendu ; l'espèce est
/// examinée avant les capacités.
///
/// Un emplacement de capacité à `0` est traité comme vide et ignoré : les
/// jeux utilisent cette valeur pour un emplacement inoccupé (voir la note sur
/// la confiance « probable » de cette convention dans
/// `docs/protocol/gen1-trade-block.md`), et `0` est de toute façon inférieur
/// à `LAST_GEN1_MOVE_ID`, donc jamais rejeté même si la convention s'avérait
/// fausse.
///
/// # Errors
///
/// Rend le motif d'inéligibilité. Ne panique jamais.
pub fn eligible_for_gen1(pokemon: &PartyPokemon) -> Result<(), Ineligible> {
    let Some(dex) = national_dex_number(pokemon.species_index()) else {
        return Err(Ineligible::UnknownSpecies);
    };
    if dex > crate::gen1::LAST_GEN1_DEX_NUMBER {
        return Err(Ineligible::SpeciesTooRecent { dex });
    }
    for (slot, move_id) in pokemon.moves().into_iter().enumerate() {
        if move_id > LAST_GEN1_MOVE_ID {
            return Err(Ineligible::MoveTooRecent {
                slot: u8::try_from(slot).unwrap_or(u8::MAX),
                move_id,
            });
        }
    }
    Ok(())
}
