//! Formats de la première génération : Rouge, Bleu, Jaune.

mod party_pokemon;
mod trade_block;

pub use party_pokemon::{Dvs, PARTY_POKEMON_LEN, PartyPokemon};
pub use trade_block::{NAME_LEN, Name, PARTY_CAPACITY, TRADE_BLOCK_LEN, TradeBlock};
