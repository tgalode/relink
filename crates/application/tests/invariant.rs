//! *Jamais de duplication*, direction **retrait**.
//!
//! On simule le cycle de vie complet d'**une** entrée en insérant une
//! interruption à chaque point où le monde réel peut lâcher, et on vérifie
//! qu'aucune séquence ne produit deux exemplaires du même Pokémon.
//!
//! # Ce que ce fichier ne couvre pas
//!
//! Son périmètre est une entrée, une réservation, un `claim`, une cartouche.
//! Il ne voit donc pas :
//!
//! - la duplication direction **dépôt** du §7.4 — deux entrées distinctes pour
//!   un seul Pokémon physique. La formule ci-dessous porte sur un seul
//!   `EntryId` et lui est structurellement aveugle ; c'est `tests/deposit.rs`
//!   qui la couvre ;
//! - la concurrence sur `claim`, appelé une seule fois par rejeu ;
//! - le tranchage croisé commit/abandon, couvert par `tests/commit.rs` ;
//! - l'invariant « seule `expire_due` ramène une entrée à `Available` »,
//!   couvert par `tests/expiry.rs`.
//!
//! **Ce fichier ne certifie pas les autres tâches du lot** : il en certifie une
//! propriété, et les autres se certifient par leurs propres suites.

use pollster::block_on;
use relink_application::commit::{Commit, CommitVerdict};
use relink_application::domain::{EntryId, ReservationId, Timestamp};
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
    ///
    /// Note d'honnêteté : cet événement et [`Event::ModuleRebooted`] partagent
    /// le bras de `match` de [`Event::ServerConfirmed`]. Ils n'ajoutent aucune
    /// couverture comportementale et multiplient le nombre de séquences ; le
    /// compte de 2800 est un compteur d'énumération, pas une mesure de
    /// couverture. Ils sont gardés parce qu'ils se différencieront le jour où
    /// le modèle distinguera le rejeu du redémarrage.
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
    /// La réservation a-t-elle déjà été tranchée, dans un sens ou dans l'autre ?
    ///
    /// Suivi ici plutôt que lu depuis l'entrée : lire l'état puis en déduire
    /// qu'il n'est pas `Available` serait une tautologie, vraie pour n'importe
    /// quelle implémentation.
    ever_settled: bool,
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
        ever_settled: false,
    };

    for event in sequence {
        match event {
            Event::ModuleAcknowledged => {
                // Le verdict est une **autorisation**, pas une trace. `Unknown`
                // veut dire que la réservation ne tient plus d'entrée — elle a
                // expiré et quelqu'un d'autre l'a prise. Le module doit alors
                // détruire ce qu'il détient sans rien donner à la cartouche.
                let verdict = block_on(pool.record_delivery(res)).expect("accusé de réception");
                // `|=` et non `=`, par posture défensive. Dans l'état actuel
                // du code cette distinction est inobservable — après un accusé
                // réussi, `record_delivery` ne peut plus rendre `Unknown`,
                // puisque `delivered: true` interdit l'expiration. Le `|=`
                // garde le modèle monotone si cette propriété venait à changer.
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
                    let verdict = block_on(commit.confirm(res)).expect("commit");
                    world.ever_settled |= verdict != CommitVerdict::Unknown;
                }
            }
            Event::Abandoned => {
                // Gardé par le verdict, symétriquement à `ModuleAcknowledged` :
                // `Unknown` signifie que la réservation ne tient plus d'entrée —
                // elle a expiré — et que rien n'a donc été tranché. Poser
                // `ever_settled` sur la simple tentative enregistrerait un
                // tranchage qui n'a pas eu lieu, et le corollaire deviendrait
                // faux sur toute séquence où une expiration précède un abandon.
                let verdict = block_on(commit.abandon(res)).expect("abandon");
                world.ever_settled |= verdict != CommitVerdict::Unknown;
            }
            Event::TtlElapsed => {
                clock.set(Timestamp::from_millis(2_000));
                block_on(expiry.execute()).expect("expiration");
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

    // Corollaire de la spec §7.1 : rien de tranché ne revient jamais au pool.
    //
    // La condition porte sur ce que le **monde** a fait, pas sur l'état lu :
    // déduire de `state != Available` que l'entrée n'est pas prenable serait
    // vrai par construction et ne testerait rien.
    assert!(
        !world.ever_settled || !entry.is_claimable(),
        "une réservation tranchée est revenue au pool.\n séquence : {seq:?}\n          dernier événement : {last:?}\n état : {:?}",
        entry.state
    );
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
