//! Formats de la première génération : Rouge, Bleu, Jaune.

pub mod patch_list;

mod party_pokemon;
mod species;
mod trade_block;

pub use party_pokemon::{Dvs, PARTY_POKEMON_LEN, PartyPokemon};
pub use species::{LAST_GEN1_DEX_NUMBER, national_dex_number};
pub use trade_block::{NAME_LEN, Name, PARTY_CAPACITY, TRADE_BLOCK_LEN, TradeBlock};
