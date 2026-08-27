//! Vue sur le bloc d'échange Gen 1.
//!
//! Disposition sourcée dans `docs/protocol/gen1-trade-block.md`.

use crate::gen1::{PARTY_POKEMON_LEN, PartyPokemon};
use crate::text::GbString;

/// Taille totale du bloc échangé par le câble.
pub const TRADE_BLOCK_LEN: usize = 415;
/// Nombre maximal de Pokémon dans une équipe.
pub const PARTY_CAPACITY: usize = 6;
/// Longueur d'un champ de nom.
pub const NAME_LEN: usize = 11;

/// Un nom tel que la cartouche le stocke.
pub type Name = GbString<NAME_LEN>;

const OFF_TRAINER_NAME: usize = 0;
const OFF_PARTY_LIST: usize = 11;
const OFF_PARTY_DATA: usize = 19;
const OFF_OT_NAMES: usize = 283;
const OFF_NICKNAMES: usize = 349;

/// Vue sur le bloc d'échange. Les octets sont conservés à l'identique :
/// ce type sert à lire, jamais à reconstruire.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct TradeBlock {
    bytes: [u8; TRADE_BLOCK_LEN],
}

impl TradeBlock {
    /// Enveloppe des octets bruts. Ne valide rien : n'importe quelle suite de
    /// 415 octets est acceptée, et les accesseurs restent sûrs.
    #[must_use]
    pub const fn from_bytes(bytes: [u8; TRADE_BLOCK_LEN]) -> Self {
        Self { bytes }
    }

    /// Les octets d'origine, inchangés.
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; TRADE_BLOCK_LEN] {
        &self.bytes
    }

    /// Nom du dresseur.
    #[must_use]
    pub fn trainer_name(&self) -> Name {
        Name::from_bytes(self.name_at(OFF_TRAINER_NAME))
    }

    /// Nombre de Pokémon dans l'équipe, borné par la capacité même si la
    /// cartouche annonce davantage.
    #[must_use]
    pub fn party_len(&self) -> usize {
        let announced = self.bytes[OFF_PARTY_LIST] as usize;
        if announced > PARTY_CAPACITY {
            PARTY_CAPACITY
        } else {
            announced
        }
    }

    /// Le Pokémon à cette position, ou `None` au-delà de l'équipe.
    #[must_use]
    pub fn pokemon(&self, index: usize) -> Option<PartyPokemon> {
        if index >= self.party_len() {
            return None;
        }
        let start = OFF_PARTY_DATA + index * PARTY_POKEMON_LEN;
        let mut raw = [0u8; PARTY_POKEMON_LEN];
        raw.copy_from_slice(&self.bytes[start..start + PARTY_POKEMON_LEN]);
        Some(PartyPokemon::from_bytes(raw))
    }

    /// Le dresseur d'origine du Pokémon à cette position.
    #[must_use]
    pub fn original_trainer(&self, index: usize) -> Option<Name> {
        if index >= self.party_len() {
            return None;
        }
        Some(Name::from_bytes(
            self.name_at(OFF_OT_NAMES + index * NAME_LEN),
        ))
    }

    /// Le surnom du Pokémon à cette position.
    #[must_use]
    pub fn nickname(&self, index: usize) -> Option<Name> {
        if index >= self.party_len() {
            return None;
        }
        Some(Name::from_bytes(
            self.name_at(OFF_NICKNAMES + index * NAME_LEN),
        ))
    }

    fn name_at(&self, offset: usize) -> [u8; NAME_LEN] {
        let mut raw = [0u8; NAME_LEN];
        raw.copy_from_slice(&self.bytes[offset..offset + NAME_LEN]);
        raw
    }
}
