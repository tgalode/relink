//! Règles de la Capsule Temporelle.
//!
//! Aucune conversion n'est inventée : les jeux ont déjà tranché, et ces règles
//! sont les leurs. Sourcées dans `docs/protocol/time-capsule-rules.md`.

use crate::gen1::{PartyPokemon, national_dex_number};

/// Identifiant de la dernière capacité de première génération.
///
/// Confiance **probable** (source unique nommée), voir
/// `docs/protocol/time-capsule-rules.md`.
pub const LAST_GEN1_MOVE_ID: u8 = 165;

/// Pourquoi un Pokémon ne peut pas descendre vers une cartouche Gen 1.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Ineligible {
    /// L'index d'espèce ne désigne aucune espèce valide.
    UnknownSpecies,
    /// L'espèce est postérieure à la première génération.
    ///
    /// Inatteignable tant que la table de correspondance
    /// [`crate::gen1::national_dex_number`] ne couvre que le Pokédex de
    /// première génération : aucun index ne peut aujourd'hui produire un
    /// numéro national supérieur à
    /// [`LAST_GEN1_DEX_NUMBER`](crate::gen1::LAST_GEN1_DEX_NUMBER). La
    /// variante est conservée quand même parce qu'elle rend cette énumération
    /// **totale** vis-à-vis des deux règles d'espèce documentées dans
    /// `docs/protocol/time-capsule-rules.md` (espèce inconnue, espèce hors
    /// Pokédex Gen 1) — pas parce qu'un futur codec Gen 2 réutiliserait
    /// [`eligible_for_gen1`] : cette fonction prend un `&gen1::PartyPokemon`
    /// et interroge une table d'index interne à la première génération ; un
    /// Pokémon Gen 2 ne pourra pas l'alimenter sans changement de signature.
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
