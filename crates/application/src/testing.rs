//! Doublures en mémoire des ports, réservées aux tests.
//!
//! Ce module n'est **pas** derrière `#[cfg(test)]` : les tests d'intégration
//! des tâches 5 à 10 vivent dans des crates séparés (`crates/application/tests/`
//! et au-delà) et ont besoin de le trouver comme dépendance normale du crate.
//! Rien ici n'est destiné à un adaptateur de production — chaque doublure
//! garde son état en mémoire de processus et l'oublie au premier redémarrage.
//!
//! Chaque doublure reste fidèle aux garanties documentées dans
//! [`crate::ports`], notamment l'atomicité : la vérification d'un état et son
//! écriture se font sous le **même** verrou, jamais deux verrous successifs.
//! Une doublure infidèle validerait, dans les tests des tâches suivantes, un
//! domaine qui ne marcherait pas en production.

use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};

use crate::domain::{EntryId, EntryState, Pokemon, PoolEntry, ReservationId, Timestamp, TrainerId};
use crate::ports::{
    ClaimOutcome, Clock, CommitOutcome, IdSource, LegalityChecker, ModuleId, ModuleTransport,
    Notifier, PoolRepository, PortError,
};

/// File des pannes à injecter, consommée un appel à la fois.
///
/// Partagée par toutes les doublures qui exposent `fail_next` : chaque appel
/// public en dépile au plus une, avant de faire quoi que ce soit d'autre.
#[derive(Default)]
struct FailQueue(VecDeque<PortError>);

impl FailQueue {
    fn push(&mut self, error: PortError) {
        self.0.push_back(error);
    }

    fn take(&mut self) -> Option<PortError> {
        self.0.pop_front()
    }
}

/// Une horloge dont l'instant est fixé par le test, jamais par le système.
///
/// Se clone en partageant son état sous [`Arc`] : deux clones voient et
/// avancent la même horloge, ce dont les tâches 7 et 10 ont besoin pour
/// piloter le temps depuis l'extérieur du composant sous test.
#[derive(Clone)]
pub struct FixedClock {
    now: Arc<Mutex<Timestamp>>,
}

impl FixedClock {
    /// Crée une horloge fixée à cet instant.
    #[must_use]
    pub fn new(now: Timestamp) -> Self {
        Self {
            now: Arc::new(Mutex::new(now)),
        }
    }

    /// Avance l'horloge de ce nombre de millisecondes.
    pub fn advance(&self, millis: u64) {
        let mut now = self.now.lock().unwrap();
        *now = now.saturating_add_millis(millis);
    }

    /// Fixe l'horloge à cet instant, sans égard à sa valeur précédente.
    pub fn set(&self, at: Timestamp) {
        *self.now.lock().unwrap() = at;
    }
}

impl Clock for FixedClock {
    async fn now(&self) -> Timestamp {
        *self.now.lock().unwrap()
    }
}

/// Une source d'identifiants séquentiels : 1, 2, 3…, jamais 0, pour que les
/// traces de test restent lisibles.
///
/// Chaque famille d'identifiants — entrées, réservations — a son propre
/// compteur.
pub struct SequentialIds {
    next_entry: Mutex<u128>,
    next_reservation: Mutex<u128>,
}

impl SequentialIds {
    /// Crée une source dont les deux compteurs partent de zéro.
    #[must_use]
    pub fn new() -> Self {
        Self {
            next_entry: Mutex::new(0),
            next_reservation: Mutex::new(0),
        }
    }
}

impl Default for SequentialIds {
    fn default() -> Self {
        Self::new()
    }
}

impl IdSource for SequentialIds {
    async fn next_entry_id(&self) -> EntryId {
        let mut next = self.next_entry.lock().unwrap();
        *next += 1;
        EntryId::from_u128(*next)
    }

    async fn next_reservation_id(&self) -> ReservationId {
        let mut next = self.next_reservation.lock().unwrap();
        *next += 1;
        ReservationId::from_u128(*next)
    }
}

/// L'état interne d'[`InMemoryPool`], tenu sous un unique verrou pour que
/// toute vérification-puis-écriture reste indivisible.
struct PoolState {
    /// Les entrées, jamais purgées par cette doublure.
    entries: HashMap<EntryId, PoolEntry>,
    /// Le registre `DepositId -> EntryId`, distinct du cycle de vie des
    /// entrées et jamais oublié : c'est lui qui porte la déduplication de
    /// [`PoolRepository::insert`], pas la survie de la ligne.
    deposit_registry: HashMap<crate::domain::DepositId, EntryId>,
    /// L'entrée que tient chaque réservation au moment où elle a été posée.
    /// Reste en place même après que la réservation a été tranchée ou a
    /// expiré : c'est ce qui permet de distinguer un rejeu tardif d'une
    /// réservation qui n'a plus cours d'une réservation inconnue.
    reservation_entry: HashMap<ReservationId, EntryId>,
    /// Pannes à injecter, une par appel.
    failures: FailQueue,
}

/// Le pool, en mémoire, pour les tests.
///
/// Toutes les opérations du trait [`PoolRepository`] verrouillent le même
/// [`Mutex`] pour la durée de leur vérification et de leur écriture : c'est
/// ce qui rend `insert`, `claim`, `record_commit`, `record_abandon`,
/// `record_delivery` et `expire_due` atomiques, comme le contrat l'exige.
pub struct InMemoryPool {
    state: Mutex<PoolState>,
}

impl InMemoryPool {
    /// Crée un pool vide.
    #[must_use]
    pub fn new() -> Self {
        Self {
            state: Mutex::new(PoolState {
                entries: HashMap::new(),
                deposit_registry: HashMap::new(),
                reservation_entry: HashMap::new(),
                failures: FailQueue::default(),
            }),
        }
    }

    /// Empile une panne : le prochain appel, quel qu'il soit, la rendra au
    /// lieu de faire son travail, et ne modifiera rien.
    pub fn fail_next(&self, error: PortError) {
        self.state.lock().unwrap().failures.push(error);
    }

    /// Le nombre d'entrées que le pool contient, y compris celles qui ne
    /// sont plus disponibles.
    #[must_use]
    pub fn len(&self) -> usize {
        self.state.lock().unwrap().entries.len()
    }

    /// Vrai si le pool ne contient aucune entrée.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl Default for InMemoryPool {
    fn default() -> Self {
        Self::new()
    }
}

/// Tranche une réservation au profit de `record_commit` ou `record_abandon`
/// (voir « Trancher une réservation, une fois pour toutes » sur
/// [`PoolRepository`]) : le premier des deux appels rend
/// [`CommitOutcome::Recorded`] et fixe l'issue, tout appel ultérieur de l'un
/// ou l'autre rend [`CommitOutcome::AlreadyRecorded`] sans rien modifier.
fn decide(
    state: &mut PoolState,
    reservation: ReservationId,
    at: Timestamp,
    settle: impl FnOnce(ReservationId, Timestamp) -> EntryState,
) -> CommitOutcome {
    let Some(&entry_id) = state.reservation_entry.get(&reservation) else {
        return CommitOutcome::Unknown;
    };
    // Les entrées ne sont jamais retirées de la table : si le registre
    // pointe vers un identifiant, l'entrée existe.
    let entry = state
        .entries
        .get_mut(&entry_id)
        .expect("entrée référencée par une réservation");
    match entry.state {
        EntryState::Reserved {
            reservation: current,
            ..
        } if current == reservation => {
            entry.state = settle(reservation, at);
            CommitOutcome::Recorded
        }
        EntryState::Committed {
            reservation: current,
            ..
        }
        | EntryState::Abandoned {
            reservation: current,
            ..
        } if current == reservation => CommitOutcome::AlreadyRecorded,
        // La réservation a existé, mais ne tient plus cette entrée — elle a
        // expiré et l'entrée a depuis été reprise par une autre réservation,
        // ou n'a jamais été prise par celle-ci. Ni « déjà tranchée », ni
        // « à trancher » : inconnue à cet instant.
        _ => CommitOutcome::Unknown,
    }
}

impl PoolRepository for InMemoryPool {
    async fn insert(&self, entry: PoolEntry) -> Result<EntryId, PortError> {
        let mut state = self.state.lock().unwrap();
        if let Some(error) = state.failures.take() {
            return Err(error);
        }
        if let Some(&existing) = state.deposit_registry.get(&entry.deposit) {
            return Ok(existing);
        }
        if state.entries.contains_key(&entry.id) {
            return Err(PortError::new(
                "identifiant d'entrée déjà enregistré sous une autre clé de dépôt",
            ));
        }
        let id = entry.id;
        state.deposit_registry.insert(entry.deposit, id);
        state.entries.insert(id, entry);
        Ok(id)
    }

    async fn get(&self, id: EntryId) -> Result<Option<PoolEntry>, PortError> {
        let mut state = self.state.lock().unwrap();
        if let Some(error) = state.failures.take() {
            return Err(error);
        }
        Ok(state.entries.get(&id).cloned())
    }

    async fn list_claimable(&self) -> Result<Vec<PoolEntry>, PortError> {
        let mut state = self.state.lock().unwrap();
        if let Some(error) = state.failures.take() {
            return Err(error);
        }
        Ok(state
            .entries
            .values()
            .filter(|entry| entry.is_claimable())
            .cloned()
            .collect())
    }

    async fn claim(
        &self,
        id: EntryId,
        reservation: ReservationId,
        expires_at: Timestamp,
    ) -> Result<ClaimOutcome, PortError> {
        let mut guard = self.state.lock().unwrap();
        if let Some(error) = guard.failures.take() {
            return Err(error);
        }
        let PoolState {
            entries,
            reservation_entry,
            ..
        } = &mut *guard;
        let Some(entry) = entries.get_mut(&id) else {
            return Ok(ClaimOutcome::NotFound);
        };
        let outcome = match entry.state {
            EntryState::Available => {
                entry.state = EntryState::Reserved {
                    reservation,
                    expires_at,
                    delivered: false,
                };
                reservation_entry.insert(reservation, id);
                ClaimOutcome::Claimed
            }
            EntryState::Reserved {
                reservation: current,
                ..
            } if current == reservation => {
                // Rejeu à l'identique : c'est bien sa réservation.
                ClaimOutcome::Claimed
            }
            _ => ClaimOutcome::AlreadyTaken,
        };
        Ok(outcome)
    }

    async fn record_commit(
        &self,
        reservation: ReservationId,
        at: Timestamp,
    ) -> Result<CommitOutcome, PortError> {
        let mut state = self.state.lock().unwrap();
        if let Some(error) = state.failures.take() {
            return Err(error);
        }
        Ok(decide(&mut state, reservation, at, |reservation, at| {
            EntryState::Committed { reservation, at }
        }))
    }

    async fn record_abandon(
        &self,
        reservation: ReservationId,
        at: Timestamp,
    ) -> Result<CommitOutcome, PortError> {
        let mut state = self.state.lock().unwrap();
        if let Some(error) = state.failures.take() {
            return Err(error);
        }
        Ok(decide(&mut state, reservation, at, |reservation, at| {
            EntryState::Abandoned { reservation, at }
        }))
    }

    async fn record_delivery(
        &self,
        reservation: ReservationId,
    ) -> Result<CommitOutcome, PortError> {
        let mut guard = self.state.lock().unwrap();
        if let Some(error) = guard.failures.take() {
            return Err(error);
        }
        let PoolState {
            entries,
            reservation_entry,
            ..
        } = &mut *guard;
        let Some(&entry_id) = reservation_entry.get(&reservation) else {
            return Ok(CommitOutcome::Unknown);
        };
        let entry = entries
            .get_mut(&entry_id)
            .expect("entrée référencée par une réservation");
        let outcome = match entry.state {
            EntryState::Reserved {
                reservation: current,
                expires_at,
                delivered,
            } if current == reservation => {
                if delivered {
                    CommitOutcome::AlreadyRecorded
                } else {
                    entry.state = EntryState::Reserved {
                        reservation,
                        expires_at,
                        delivered: true,
                    };
                    CommitOutcome::Recorded
                }
            }
            EntryState::Committed {
                reservation: current,
                ..
            }
            | EntryState::Abandoned {
                reservation: current,
                ..
            } if current == reservation => CommitOutcome::AlreadyRecorded,
            _ => CommitOutcome::Unknown,
        };
        Ok(outcome)
    }

    async fn expire_due(&self, now: Timestamp) -> Result<Vec<EntryId>, PortError> {
        let mut state = self.state.lock().unwrap();
        if let Some(error) = state.failures.take() {
            return Err(error);
        }
        let mut expired = Vec::new();
        for entry in state.entries.values_mut() {
            if let EntryState::Reserved {
                delivered: false,
                expires_at,
                ..
            } = entry.state
            {
                if expires_at <= now {
                    expired.push(entry.id);
                    entry.state = EntryState::Available;
                }
            }
        }
        expired.sort_by_key(|id| id.as_u128());
        Ok(expired)
    }
}

/// Un contrôleur de légalité dont le verdict est fixé à la construction.
pub struct StubLegality {
    accepts: bool,
}

impl StubLegality {
    /// Un contrôleur qui accepte tout Pokémon.
    #[must_use]
    pub fn accepting() -> Self {
        Self { accepts: true }
    }

    /// Un contrôleur qui rejette tout Pokémon.
    #[must_use]
    pub fn rejecting() -> Self {
        Self { accepts: false }
    }
}

impl LegalityChecker for StubLegality {
    async fn is_legal(&self, _pokemon: &Pokemon) -> Result<bool, PortError> {
        Ok(self.accepts)
    }
}

/// Une poussée enregistrée par [`RecordingTransport`].
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct PushedReservation {
    /// Le module vers lequel la poussée a été faite.
    pub module: ModuleId,
    /// La réservation poussée.
    pub reservation: ReservationId,
    /// Le Pokémon poussé.
    pub pokemon: Pokemon,
}

/// L'état interne de [`RecordingTransport`], sous un seul verrou.
#[derive(Default)]
struct TransportState {
    pushed: Vec<PushedReservation>,
    failures: FailQueue,
}

/// Un transport de test qui enregistre chaque poussée au lieu de parler à un
/// module physique.
pub struct RecordingTransport {
    state: Mutex<TransportState>,
}

impl RecordingTransport {
    /// Crée un transport dont aucune poussée n'a encore eu lieu.
    #[must_use]
    pub fn new() -> Self {
        Self {
            state: Mutex::new(TransportState::default()),
        }
    }

    /// Les poussées enregistrées jusqu'ici, dans l'ordre des appels.
    #[must_use]
    pub fn pushed(&self) -> Vec<PushedReservation> {
        self.state.lock().unwrap().pushed.clone()
    }

    /// Empile une panne : le prochain appel la rendra au lieu de pousser.
    pub fn fail_next(&self, error: PortError) {
        self.state.lock().unwrap().failures.push(error);
    }
}

impl Default for RecordingTransport {
    fn default() -> Self {
        Self::new()
    }
}

impl ModuleTransport for RecordingTransport {
    async fn push_reservation(
        &self,
        module: ModuleId,
        reservation: ReservationId,
        pokemon: &Pokemon,
    ) -> Result<(), PortError> {
        let mut state = self.state.lock().unwrap();
        if let Some(error) = state.failures.take() {
            return Err(error);
        }
        state.pushed.push(PushedReservation {
            module,
            reservation,
            pokemon: *pokemon,
        });
        Ok(())
    }
}

/// Une notification enregistrée par [`RecordingNotifier`].
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct NotifiedClaim {
    /// Le dresseur prévenu.
    pub depositor: TrainerId,
    /// L'entrée que quelqu'un a prise.
    pub entry: EntryId,
}

/// L'état interne de [`RecordingNotifier`], sous un seul verrou.
#[derive(Default)]
struct NotifierState {
    notified: Vec<NotifiedClaim>,
    failures: FailQueue,
}

/// Un notificateur de test qui enregistre chaque notification au lieu de
/// parler à un joueur.
pub struct RecordingNotifier {
    state: Mutex<NotifierState>,
}

impl RecordingNotifier {
    /// Crée un notificateur dont aucune notification n'a encore eu lieu.
    #[must_use]
    pub fn new() -> Self {
        Self {
            state: Mutex::new(NotifierState::default()),
        }
    }

    /// Les notifications enregistrées jusqu'ici, dans l'ordre des appels.
    #[must_use]
    pub fn notified(&self) -> Vec<NotifiedClaim> {
        self.state.lock().unwrap().notified.clone()
    }

    /// Empile une panne : le prochain appel la rendra au lieu de notifier.
    pub fn fail_next(&self, error: PortError) {
        self.state.lock().unwrap().failures.push(error);
    }
}

impl Default for RecordingNotifier {
    fn default() -> Self {
        Self::new()
    }
}

impl Notifier for RecordingNotifier {
    async fn entry_claimed(&self, depositor: &TrainerId, entry: EntryId) -> Result<(), PortError> {
        let mut state = self.state.lock().unwrap();
        if let Some(error) = state.failures.take() {
            return Err(error);
        }
        state.notified.push(NotifiedClaim {
            depositor: *depositor,
            entry,
        });
        Ok(())
    }
}
