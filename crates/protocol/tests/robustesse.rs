//! Preuve d'infaillibilité des codecs Gen 1 : quels que soient les octets
//! fournis — cartouche corrompue ou modifiée comprise — aucun accesseur ne
//! doit paniquer. Une panique en plein échange laisse la cartouche dans un
//! état indéterminé, potentiellement une sauvegarde détruite.

use proptest::prelude::*;
use relink_protocol::gen1::{
    LAST_GEN1_DEX_NUMBER, PARTY_CAPACITY, PARTY_POKEMON_LEN, PartyPokemon, TRADE_BLOCK_LEN,
    TradeBlock, national_dex_number,
};
use relink_protocol::text::GbString;
use relink_protocol::time_capsule::{Ineligible, eligible_for_gen1};

const OFF_PARTY_LIST: usize = 11;

proptest! {
    /// Aucun accesseur ne panique, quels que soient les octets.
    #[test]
    fn le_bloc_ne_panique_sur_aucune_entree(raw in prop::array::uniform32(any::<u8>())) {
        // On étale 32 octets aléatoires sur tout le bloc pour couvrir
        // chaque champ sans exiger une stratégie de 415 octets.
        let mut bytes = [0u8; TRADE_BLOCK_LEN];
        for (i, b) in bytes.iter_mut().enumerate() {
            *b = raw[i % raw.len()];
        }
        let block = TradeBlock::from_bytes(bytes);

        let _ = block.trainer_name().len();
        prop_assert!(block.party_len() <= PARTY_CAPACITY);
        for i in 0..=PARTY_CAPACITY {
            if let Some(p) = block.pokemon(i) {
                let _ = (p.species_index(), p.level(), p.experience(), p.trainer_id());
                let _ = (p.moves(), p.dvs());
                let _ = eligible_for_gen1(&p);
            }
            let _ = block.original_trainer(i);
            let _ = block.nickname(i);
        }
        prop_assert_eq!(block.as_bytes(), &bytes);
    }

    /// Le ré-encodage est identique à l'octet près. C'est la propriété qui
    /// protège les sauvegardes.
    #[test]
    fn le_reencodage_est_identique(seed in any::<u8>()) {
        let bytes = [seed; TRADE_BLOCK_LEN];
        let block = TradeBlock::from_bytes(bytes);
        prop_assert_eq!(block.as_bytes(), &bytes);
    }

    /// Un index hors équipe ne rend jamais rien, jamais de panique.
    #[test]
    fn hors_equipe_rend_none(index in PARTY_CAPACITY..usize::MAX) {
        let block = TradeBlock::from_bytes([0u8; TRADE_BLOCK_LEN]);
        prop_assert!(block.pokemon(index).is_none());
        prop_assert!(block.original_trainer(index).is_none());
        prop_assert!(block.nickname(index).is_none());
    }

    /// La longueur d'une chaîne ne dépasse jamais son champ.
    #[test]
    fn la_longueur_reste_dans_le_champ(raw in prop::array::uniform11(any::<u8>())) {
        prop_assert!(GbString::<11>::from_bytes(raw).len() <= 11);
    }

    /// L'éligibilité ne panique jamais et ne refuse jamais pour espèce trop
    /// récente tant que la table ne contient que de la première génération.
    #[test]
    fn l_eligibilite_ne_panique_jamais(raw in prop::array::uniform32(any::<u8>())) {
        let mut bytes = [0u8; PARTY_POKEMON_LEN];
        for (i, b) in bytes.iter_mut().enumerate() {
            *b = raw[i % raw.len()];
        }
        let p = PartyPokemon::from_bytes(bytes);
        if let Err(Ineligible::SpeciesTooRecent { dex }) = eligible_for_gen1(&p) {
            prop_assert!(dex > LAST_GEN1_DEX_NUMBER, "refus incohérent avec la table");
        }
    }

    /// Toute traduction d'espèce reste dans le Pokédex de première génération.
    #[test]
    fn la_traduction_reste_bornee(index in any::<u8>()) {
        if let Some(dex) = national_dex_number(index) {
            prop_assert!((1..=LAST_GEN1_DEX_NUMBER).contains(&dex));
        }
    }

    /// Quel que soit l'octet de compte annoncé (y compris largement
    /// surdimensionné, jusqu'à 255), l'équipe entière jusqu'à la capacité
    /// reste indexable de bout en bout sans jamais paniquer : `pokemon`,
    /// `original_trainer` et `nickname` restent utilisables sur tout index
    /// `0..PARTY_CAPACITY`, y compris le dernier, `PARTY_CAPACITY - 1`.
    #[test]
    fn une_equipe_surdimensionnee_reste_indexable_jusqu_a_la_capacite(
        announced in (PARTY_CAPACITY as u8 + 1)..=u8::MAX,
        raw in prop::array::uniform32(any::<u8>()),
    ) {
        let mut bytes = [0u8; TRADE_BLOCK_LEN];
        for (i, b) in bytes.iter_mut().enumerate() {
            *b = raw[i % raw.len()];
        }
        bytes[OFF_PARTY_LIST] = announced;
        let block = TradeBlock::from_bytes(bytes);

        prop_assert_eq!(block.party_len(), PARTY_CAPACITY);
        for i in 0..PARTY_CAPACITY {
            let p = block.pokemon(i);
            prop_assert!(p.is_some(), "index {i} devrait rester dans l'équipe bornée");
            let p = p.expect("vérifié ci-dessus");
            let _ = (p.species_index(), p.level(), p.experience(), p.trainer_id());
            let _ = (p.moves(), p.dvs());
            prop_assert!(block.original_trainer(i).is_some());
            prop_assert!(block.nickname(i).is_some());
        }
        // Toujours rien au-delà, même avec un compte annoncé énorme.
        prop_assert!(block.pokemon(PARTY_CAPACITY).is_none());
        prop_assert!(block.original_trainer(PARTY_CAPACITY).is_none());
        prop_assert!(block.nickname(PARTY_CAPACITY).is_none());
    }
}

/// Cas limite exact : le compte annoncé vaut `PARTY_CAPACITY` pile, ni plus
/// ni moins. C'est la frontière entre « équipe complète légitime » et
/// « équipe surdimensionnée » — le dernier emplacement doit rester lisible
/// et rien au-delà.
#[test]
fn le_compte_annonce_egal_a_la_capacite_expose_toute_l_equipe() {
    let mut bytes = [0u8; TRADE_BLOCK_LEN];
    bytes[OFF_PARTY_LIST] = PARTY_CAPACITY as u8;
    let block = TradeBlock::from_bytes(bytes);

    assert_eq!(block.party_len(), PARTY_CAPACITY);
    for i in 0..PARTY_CAPACITY {
        assert!(block.pokemon(i).is_some(), "index {i} doit être présent");
        assert!(block.original_trainer(i).is_some());
        assert!(block.nickname(i).is_some());
    }
    assert!(block.pokemon(PARTY_CAPACITY).is_none());
    assert!(block.original_trainer(PARTY_CAPACITY).is_none());
    assert!(block.nickname(PARTY_CAPACITY).is_none());
}
