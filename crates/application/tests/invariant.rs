//! *Jamais de duplication* : l'unique propriété qui protège le pool.
//!
//! On simule le cycle de vie complet d'une entrée en insérant une interruption
//! à chaque point où le monde réel peut lâcher, et on vérifie qu'aucune
//! séquence ne produit deux exemplaires du même Pokémon.

use pollster::block_on;
use relink_application::commit::{Commit, CommitVerdict};
use relink_application::domain::{EntryId, EntryState, ReservationId, Timestamp};
use relink_application::expiry::ExpireReservations;
use relink_application::ports::{CommitOutcome, PoolRepository};
use relink_application::testing::{FixedClock, InMemoryPool};

mod util;
use util::sample_entry;

/// Ce que le monde peut faire subir à un échange en cours.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Event {
    /// Le module accuse réception de la réservation.
    ModuleAcknowledged,
    /// Le module écrit son intention en flash, puis la cartouche commite.
    CartridgeCommitted,
    /// Le serveur enregistre la confirmation.
    ServerConfirmed,
    /// Le message est rejoué (MQTT QoS 1, ou rejeu de journal au redémarrage).
    Redelivered,
    /// Le module redémarre et rejoue son journal.
    ModuleRebooted,
    /// On ne saura jamais si la cartouche a reçu : flash perdue, module détruit.
    Abandoned,
    /// Le temps passe au-delà de l'échéance de la réservation.
    TtlElapsed,
}

const ALL_EVENTS: [Event; 7] = [
    Event::ModuleAcknowledged,
    Event::CartridgeCommitted,
    Event::ServerConfirmed,
    Event::Redelivered,
    Event::ModuleRebooted,
    Event::Abandoned,
    Event::TtlElapsed,
];

/// L'état du monde qu'on suit en parallèle du pool : ce que les cartouches
/// détiennent réellement, indépendamment de ce que le serveur croit.
struct World {
    /// Le module a-t-il accusé réception ? Une cartouche ne peut rien recevoir
    /// avant cela.
    module_acknowledged: bool,
    /// La cartouche a-t-elle réellement reçu le Pokémon ?
    cartridge_holds: bool,
}

/// Rejoue une séquence d'événements et vérifie l'invariant après chacun.
fn replay(sequence: &[Event]) {
    let id = EntryId::from_u128(1);
    let res = ReservationId::from_u128(10);
    let pool = InMemoryPool::new();
    let clock = FixedClock::new(Timestamp::from_millis(0));
    block_on(pool.insert(sample_entry(id))).expect("insertion");
    block_on(pool.claim(id, res, Timestamp::from_millis(1_000))).expect("réservation");

    let commit = Commit::new(&pool, clock.clone());
    let expiry = ExpireReservations::new(&pool, clock.clone());
    let mut world = World {
        module_acknowledged: false,
        cartridge_holds: false,
    };

    for event in sequence {
        match event {
            Event::ModuleAcknowledged => {
                // Le verdict est une **autorisation**, pas une trace. `Unknown`
                // veut dire que la réservation ne tient plus d'entrée — elle a
                // expiré et quelqu'un d'autre l'a prise. Le module doit alors
                // détruire ce qu'il détient sans rien donner à la cartouche.
                let verdict = block_on(pool.record_delivery(res)).expect("accusé de réception");
                // `|=` et non `=` : un modèle monotone. Un second accusé qui
                // rendrait `Unknown` ne doit pas faire *oublier* une remise déjà
                // simulée, sinon le harnais cesserait de modéliser la cartouche
                // et masquerait une duplication.
                world.module_acknowledged |= verdict != CommitOutcome::Unknown;
            }
            // Une cartouche ne peut recevoir que d'un module qui a reçu la
            // réservation. Le modèle doit refléter cette dépendance, sinon il
            // teste un monde qui n'existe pas.
            Event::CartridgeCommitted => {
                if world.module_acknowledged {
                    world.cartridge_holds = true;
                }
            }
            Event::ServerConfirmed | Event::Redelivered | Event::ModuleRebooted => {
                // Le module ne confirme que ce que la cartouche a réellement pris.
                if world.cartridge_holds {
                    // Les trois verdicts sont légitimes ici, `Unknown` compris :
                    // après une expiration la réservation ne tient plus d'entrée.
                    // Ne resserre pas cette assertion — c'est l'invariant qui juge,
                    // pas le harnais, et ce cas est précisément le plus intéressant.
                    let _verdict = block_on(commit.confirm(res)).expect("commit");
                    let _ = CommitVerdict::Applied;
                }
            }
            Event::Abandoned => {
                block_on(commit.abandon(res)).expect("abandon");
            }
            Event::TtlElapsed => {
                clock.set(Timestamp::from_millis(2_000));
                block_on(expiry.run()).expect("expiration");
            }
        }
        assert_invariant(&pool, id, &world, sequence, *event);
    }
}

/// Le cœur : compte les exemplaires et refuse qu'il y en ait deux.
fn assert_invariant(pool: &InMemoryPool, id: EntryId, world: &World, seq: &[Event], last: Event) {
    let entry = block_on(pool.get(id)).expect("lecture").expect("présente");
    let in_pool = usize::from(entry.is_claimable());
    let on_cartridge = usize::from(world.cartridge_holds);

    assert!(
        in_pool + on_cartridge <= 1,
        "duplication : le Pokémon est à la fois sur une cartouche et dans le pool.\n\
         séquence : {seq:?}\n dernier événement : {last:?}\n état : {:?}",
        entry.state
    );

    // Corollaire de la spec §7.1 : rien de tranché ne revient jamais.
    if matches!(
        entry.state,
        EntryState::Committed { .. } | EntryState::Abandoned { .. }
    ) {
        assert!(
            !entry.is_claimable(),
            "une réservation tranchée ne revient jamais au pool"
        );
    }
}

#[test]
fn aucune_sequence_d_interruption_ne_produit_de_duplication() {
    let mut sequences = 0usize;
    for len in 1..=4 {
        let mut indices = vec![0usize; len];
        loop {
            let sequence: Vec<Event> = indices.iter().map(|&i| ALL_EVENTS[i]).collect();
            replay(&sequence);
            sequences += 1;

            // Incrémente le compteur en base ALL_EVENTS.len().
            let mut pos = len;
            loop {
                if pos == 0 {
                    break;
                }
                pos -= 1;
                indices[pos] += 1;
                if indices[pos] < ALL_EVENTS.len() {
                    break;
                }
                indices[pos] = 0;
                if pos == 0 {
                    break;
                }
            }
            if indices.iter().all(|&i| i == 0) {
                break;
            }
        }
    }
    assert_eq!(
        sequences,
        7 + 49 + 343 + 2401,
        "l'énumération doit être exhaustive"
    );
}

#[test]
fn le_scenario_le_plus_dangereux_est_couvert_explicitement() {
    // Celui qui a fait corriger la spec §7.2 : la cartouche a reçu, le serveur
    // ne le saura jamais, et le temps passe.
    replay(&[
        Event::ModuleAcknowledged,
        Event::CartridgeCommitted,
        Event::TtlElapsed,
    ]);
    replay(&[
        Event::ModuleAcknowledged,
        Event::CartridgeCommitted,
        Event::Abandoned,
        Event::TtlElapsed,
    ]);
    replay(&[
        Event::ModuleAcknowledged,
        Event::CartridgeCommitted,
        Event::TtlElapsed,
        Event::Abandoned,
    ]);
    replay(&[
        Event::ModuleAcknowledged,
        Event::CartridgeCommitted,
        Event::ModuleRebooted,
        Event::Redelivered,
    ]);
}
