# Crate `application` — plan d'implémentation

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Donner au crate `relink-application` les trois cas d'usage du service — déposer, réserver puis retirer, apparier un échange direct — et le commit idempotent qui empêche qu'un Pokémon existe en deux exemplaires.

**Architecture:** Hexagonal. Les cas d'usage sont génériques sur des ports déclarés en traits, dont aucun n'a d'implémentation ici. Le crate ne connaît ni base de données, ni MQTT, ni horloge système. La seule chose qu'il connaît vraiment, c'est l'ordre dans lequel les choses doivent arriver pour qu'une cartouche ne puisse jamais recevoir un Pokémon qui reste aussi dans le pool.

**Tech Stack:** Rust 2024, `std`, ports en traits `async`, `thiserror` pour les erreurs, `pollster` et `proptest` en dépendances de développement.

**Spec:** `docs/superpowers/specs/2026-08-27-relink-coeur-metier-design.md` — sections 6 et 7. En cas de conflit entre ce plan et la spec, **la spec l'emporte**.

## Global Constraints

- `crates/application` est `std`, licence `AGPL-3.0-or-later`. Il dépend de `relink-protocol` ; **jamais l'inverse**.
- `unsafe_code = "forbid"` et `missing_docs = "warn"` sont hérités du workspace. Tout élément public porte un commentaire de documentation, variantes d'énumération et champs compris.
- **Aucun port n'a d'implémentation dans ce lot.** Les seules implémentations autorisées sont les doublures de test.
- **Le crate ne touche jamais à l'horloge système, ni à un générateur d'aléa, ni au réseau.** Le temps et les identifiants arrivent par des ports. C'est ce qui rend le test d'invariant de la tâche 10 possible.
- Rust 1.85 minimum, édition 2024.
- Les tests vivent dans `crates/application/tests/` et n'utilisent que l'API publique. Un `//!` de documentation en tête est nécessaire.
- **Arbitrage de la spec §7.1, non négociable : en cas d'ambiguïté, on choisit la perte.** Une entrée dont on ne sait pas si la cartouche l'a reçue reste consommée. Aucun code de ce lot ne doit pouvoir rendre au pool une entrée dont un commit a été tenté.
- **Spec §7.2 : le TTL ne protège que contre une réservation qui n'est jamais parvenue à un module.** Une entrée dont un module a accusé réception ne revient **jamais** automatiquement au pool, même très longtemps après son échéance — seul le module peut la trancher. Un module hors ligne ayant déjà remis le Pokémon à la cartouche est indiscernable d'un module qui n'a rien reçu : rendre l'entrée au pool dans le doute produirait une duplication.
- **Spec §7.3 : il n'existe aucun chemin de commit à deux cartouches.** L'échange direct est un dépôt et un retrait appariés, rien d'autre.

## Un écart de méthode, déclaré

Le plan des codecs donnait le code d'implémentation complet de chaque tâche. Celui-ci ne le fait
que là où la forme est contraignante, et se contente ailleurs d'imposer l'**interface exacte**,
l'**ordre des opérations** et le **jeu de tests complet**. Motif : les cas d'usage sont minces, et
c'est leur enchaînement qui compte, pas leur corps. Les tests de chaque tâche sont écrits pour
contraindre entièrement le comportement attendu — un implémenteur qui les fait passer sans respecter
l'ordre imposé écrira du code qui échoue à un test au moins ; c'est voulu.

Conséquence pour la relecture : partout où une étape décrit au lieu de montrer, **le jeu de tests est
le cahier des charges**. Un relecteur qui trouve un comportement non contraint par les tests a trouvé
un trou du plan, pas une liberté de l'implémenteur.

## Trois décisions prises par ce plan

La spec fixe l'architecture, pas ces détails. Les voici tranchées, avec ce qu'elles coûtent si elles sont mauvaises.

**Ports `async`, cas d'usage génériques, jamais de `dyn`.** `ModuleTransport` et `Notifier` sont intrinsèquement asynchrones, et un serveur Rust écrit aujourd'hui l'est de bout en bout ; envelopper un cœur synchrone dans `spawn_blocking` est exactement ce qui se fait arracher un an plus tard. Les cas d'usage sont génériques sur leurs ports plutôt que de prendre des objets-traits, ce qui évite le passage par `Box` que `async fn` en trait impose encore à l'usage dynamique. *Coût si erroné : toutes les signatures de ports à reprendre.*

**Les tests n'embarquent pas de runtime.** Le crate ne dépend d'aucun exécuteur : il ne lance rien en tâche de fond, ne dort jamais, et lit l'heure par un port. Les tests se contentent donc de `pollster::block_on`. Si un jour une tâche de ce plan avait besoin de `tokio` pour passer, c'est que le domaine aurait acquis une dépendance temporelle qu'il ne doit pas avoir — **c'est un signal d'alerte, pas un détail d'outillage**.

**Le temps est un `Timestamp` du domaine, pas un `std::time::Instant`.** Un entier de millisecondes depuis l'époque Unix : sérialisable, comparable, reproductible en test, et transportable jusqu'au module. `Instant` n'est aucune de ces choses.

---

### Task 1: Éligibilité au niveau de l'équipe

Dette identifiée par la relecture finale du lot précédent. La règle sourcée de la Capsule Temporelle est **d'équipe** — aucun Pokémon de l'équipe ne doit connaître de capacité postérieure à la Gen 1 — alors que `relink-protocol` n'offre qu'un verdict par Pokémon. `application` en a besoin dès le premier cas d'usage : c'est ici que la sémantique de l'erreur se décide, ce qui est précisément pourquoi elle avait été repoussée.

Cette tâche modifie le crate `protocol`, donc `no_std` sans allocateur s'applique.

**Files:**
- Modify: `crates/protocol/src/time_capsule.rs`
- Test: `crates/protocol/tests/time_capsule.rs`

**Interfaces:**
- Consumes: `gen1::TradeBlock`, `gen1::PARTY_CAPACITY`, `time_capsule::{Ineligible, eligible_for_gen1}`
- Produces: `pub fn time_capsule::first_ineligible_in_party(block: &gen1::TradeBlock) -> Option<(usize, Ineligible)>`

- [ ] **Step 1: Écrire le test qui échoue**

Ajouter à `crates/protocol/tests/time_capsule.rs` :

```rust
use relink_protocol::gen1::{PARTY_CAPACITY, TRADE_BLOCK_LEN, TradeBlock};
use relink_protocol::time_capsule::first_ineligible_in_party;

/// Les décalages viennent de `docs/protocol/gen1-trade-block.md`.
const OFF_PARTY_LIST: usize = 11;
const OFF_PARTY_DATA: usize = 19;
const PARTY_POKEMON: usize = 44;

/// Un bloc dont l'équipe compte `count` Pokémon, tous d'espèce `species`,
/// et dont le Pokémon à `bad_slot` connaît une capacité trop récente.
fn party(count: u8, species: u8, bad_slot: Option<usize>) -> TradeBlock {
    let mut raw = [0u8; TRADE_BLOCK_LEN];
    raw[OFF_PARTY_LIST] = count;
    for i in 0..count as usize {
        let base = OFF_PARTY_DATA + i * PARTY_POKEMON;
        raw[base] = species;
        raw[base + 0x08] = 1;
        if bad_slot == Some(i) {
            raw[base + 0x09] = 200; // postérieure à la Gen 1
        }
    }
    TradeBlock::from_bytes(raw)
}

/// Espèce valide en Gen 1 selon `docs/protocol/gen1-species-index.md`.
const MEW: u8 = 0x15;

#[test]
fn une_equipe_entierement_eligible_ne_rend_rien() {
    assert_eq!(first_ineligible_in_party(&party(3, MEW, None)), None);
}

#[test]
fn une_equipe_vide_ne_rend_rien() {
    assert_eq!(first_ineligible_in_party(&party(0, MEW, None)), None);
}

#[test]
fn rend_la_position_du_premier_fautif() {
    let (slot, _) = first_ineligible_in_party(&party(4, MEW, Some(2))).expect("un fautif");
    assert_eq!(slot, 2);
}

#[test]
fn rend_le_premier_fautif_et_pas_un_suivant() {
    let block = party(4, MEW, Some(3));
    let mut raw = *block.as_bytes();
    raw[OFF_PARTY_DATA + PARTY_POKEMON + 0x09] = 200;
    let (slot, _) = first_ineligible_in_party(&TradeBlock::from_bytes(raw)).expect("un fautif");
    assert_eq!(slot, 1, "c'est le premier fautif qui doit être rendu");
}

#[test]
fn n_examine_jamais_au_dela_de_l_equipe_annoncee() {
    // Une équipe de 1, mais des octets fautifs dans les emplacements suivants.
    let block = party(1, MEW, None);
    let mut raw = *block.as_bytes();
    for i in 1..PARTY_CAPACITY {
        raw[OFF_PARTY_DATA + i * PARTY_POKEMON + 0x09] = 200;
    }
    assert_eq!(first_ineligible_in_party(&TradeBlock::from_bytes(raw)), None);
}
```

- [ ] **Step 2: Lancer le test et vérifier qu'il échoue**

Run: `cargo test -p relink-protocol --test time_capsule`
Expected: FAIL — `first_ineligible_in_party` introuvable.

- [ ] **Step 3: Écrire l'implémentation**

Ajouter à `crates/protocol/src/time_capsule.rs` :

```rust
use crate::gen1::TradeBlock;

/// Cherche le premier Pokémon de l'équipe qui ne peut pas descendre vers une
/// cartouche de première génération, et rend sa position dans l'équipe avec
/// le motif du refus.
///
/// La règle de la Capsule Temporelle porte sur l'équipe entière : il suffit
/// d'un Pokémon inéligible pour que l'échange soit refusé. Cette fonction rend
/// le **premier** fautif, celui qu'on montre à l'utilisateur ; elle n'examine
/// jamais au-delà de l'équipe réellement annoncée.
///
/// Rend `None` si toute l'équipe est éligible, équipe vide comprise.
#[must_use]
pub fn first_ineligible_in_party(block: &TradeBlock) -> Option<(usize, Ineligible)> {
    let mut index = 0;
    while let Some(pokemon) = block.pokemon(index) {
        if let Err(reason) = eligible_for_gen1(&pokemon) {
            return Some((index, reason));
        }
        index += 1;
    }
    None
}
```

La boucle s'arrête sur le premier `None` de `TradeBlock::pokemon`, qui borne déjà l'index à la capacité : elle ne peut donc pas tourner indéfiniment.

- [ ] **Step 4: Lancer les tests et vérifier qu'ils passent**

Run: `cargo test -p relink-protocol`
Expected: PASS, 5 tests de plus.

Run: `cargo clippy --workspace --all-targets`, `cargo fmt --all --check`, puis `cargo build -p relink-protocol --target thumbv7em-none-eabihf`
Expected: aucun avertissement, compilation croisée réussie.

- [ ] **Step 5: Commit**

```bash
git add crates/protocol/src/time_capsule.rs crates/protocol/tests/time_capsule.rs
git commit -m "feat(protocol): éligibilité Capsule Temporelle au niveau de l'équipe"
```

---

### Task 2: Les types du domaine

**Files:**
- Create: `crates/application/src/domain.rs`
- Modify: `crates/application/src/lib.rs`
- Modify: `crates/application/Cargo.toml`
- Test: `crates/application/tests/domain.rs`

**Interfaces:**
- Produces, tous depuis `relink_application::domain` :
  - `pub struct Timestamp(u64)` avec `from_millis`, `as_millis`, `saturating_add_millis`
  - `pub struct TrainerId { pub name: gen1::Name, pub number: u16 }`
  - `pub struct EntryId(u128)` et `pub struct ReservationId(u128)`, chacun avec `from_u128` et `as_u128`
  - `pub struct Provenance { pub depositor: TrainerId, pub deposited_at: Timestamp, pub previous: Vec<TrainerId> }`
  - `pub struct Pokemon { pub bytes: [u8; gen1::PARTY_POKEMON_LEN], pub nickname: gen1::Name, pub original_trainer: gen1::Name }`
  - `pub enum EntryState { Available, Reserved { reservation: ReservationId, expires_at: Timestamp }, Committed { reservation: ReservationId, at: Timestamp }, Abandoned { reservation: ReservationId, at: Timestamp } }`
  - `pub struct PoolEntry { pub id: EntryId, pub pokemon: Pokemon, pub provenance: Provenance, pub state: EntryState }`
  - `PoolEntry::is_claimable(&self) -> bool`
  - `EntryState::reservation(&self) -> Option<ReservationId>`

- [ ] **Step 1: Déclarer les dépendances**

Dans `crates/application/Cargo.toml`, remplacer la section `[dependencies]` et ajouter les dépendances de développement :

```toml
[dependencies]
relink-protocol = { path = "../protocol" }
thiserror = "2"

[dev-dependencies]
pollster = "0.4"
proptest = "1"
```

- [ ] **Step 2: Écrire le test qui échoue**

`crates/application/tests/domain.rs` :

```rust
//! Types du domaine du service relink.

use relink_application::domain::{EntryId, EntryState, PoolEntry, ReservationId, Timestamp};
use relink_protocol::gen1::{NAME_LEN, PARTY_POKEMON_LEN};

fn at(ms: u64) -> Timestamp {
    Timestamp::from_millis(ms)
}

fn entry(state: EntryState) -> PoolEntry {
    use relink_application::domain::{Pokemon, Provenance, TrainerId};
    use relink_protocol::gen1::Name;
    let name = Name::from_bytes([0x50; NAME_LEN]);
    PoolEntry {
        id: EntryId::from_u128(1),
        pokemon: Pokemon {
            bytes: [0u8; PARTY_POKEMON_LEN],
            nickname: name,
            original_trainer: name,
        },
        provenance: Provenance {
            depositor: TrainerId { name, number: 42 },
            deposited_at: at(0),
            previous: Vec::new(),
        },
        state,
    }
}

#[test]
fn le_temps_se_compare_et_s_avance() {
    assert!(at(10) < at(20));
    assert_eq!(at(10).saturating_add_millis(5), at(15));
    assert_eq!(at(10).as_millis(), 10);
}

#[test]
fn l_avance_du_temps_sature_au_lieu_de_deborder() {
    assert_eq!(Timestamp::from_millis(u64::MAX).saturating_add_millis(1).as_millis(), u64::MAX);
}

#[test]
fn seule_une_entree_disponible_est_prenable() {
    let r = ReservationId::from_u128(7);
    assert!(entry(EntryState::Available).is_claimable());
    assert!(!entry(EntryState::Reserved { reservation: r, expires_at: at(1), delivered: false }).is_claimable());
    assert!(!entry(EntryState::Reserved { reservation: r, expires_at: at(1), delivered: true }).is_claimable());
    assert!(!entry(EntryState::Committed { reservation: r, at: at(1) }).is_claimable());
    assert!(!entry(EntryState::Abandoned { reservation: r, at: at(1) }).is_claimable());
}

#[test]
fn l_etat_rend_la_reservation_qui_le_gouverne() {
    let r = ReservationId::from_u128(7);
    assert_eq!(EntryState::Available.reservation(), None);
    assert_eq!(EntryState::Reserved { reservation: r, expires_at: at(1), delivered: false }.reservation(), Some(r));
    assert_eq!(EntryState::Committed { reservation: r, at: at(1) }.reservation(), Some(r));
    assert_eq!(EntryState::Abandoned { reservation: r, at: at(1) }.reservation(), Some(r));
}
```

- [ ] **Step 3: Lancer le test et vérifier qu'il échoue**

Run: `cargo test -p relink-application --test domain`
Expected: FAIL — `unresolved import relink_application::domain`

- [ ] **Step 4: Écrire l'implémentation**

`crates/application/src/domain.rs` :

```rust
//! Les types que le service manipule, indépendants de tout stockage.

use relink_protocol::gen1::{self, PARTY_POKEMON_LEN};

/// Un instant, en millisecondes depuis l'époque Unix.
///
/// Le domaine ne lit jamais l'horloge système : le temps entre par le port
/// [`crate::ports::Clock`]. C'est ce qui rend les scénarios reproductibles.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
pub struct Timestamp(u64);

impl Timestamp {
    /// Construit un instant à partir de millisecondes depuis l'époque Unix.
    #[must_use]
    pub const fn from_millis(millis: u64) -> Self {
        Self(millis)
    }

    /// Les millisecondes depuis l'époque Unix.
    #[must_use]
    pub const fn as_millis(self) -> u64 {
        self.0
    }

    /// Avance l'instant, en saturant plutôt qu'en débordant.
    #[must_use]
    pub const fn saturating_add_millis(self, millis: u64) -> Self {
        Self(self.0.saturating_add(millis))
    }
}

/// Le dresseur tel que la cartouche le connaît : son nom et son identifiant.
///
/// C'est la seule identité que le domaine manipule. Les comptes utilisateurs
/// sont un problème d'adaptateur et ne remontent pas jusqu'ici.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct TrainerId {
    /// Nom du dresseur, tel qu'il est stocké sur la cartouche.
    pub name: gen1::Name,
    /// Identifiant de dresseur.
    pub number: u16,
}

/// Identifiant d'une entrée du pool.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash, PartialOrd, Ord)]
pub struct EntryId(u128);

impl EntryId {
    /// Construit un identifiant.
    #[must_use]
    pub const fn from_u128(value: u128) -> Self {
        Self(value)
    }
    /// La valeur sous-jacente.
    #[must_use]
    pub const fn as_u128(self) -> u128 {
        self.0
    }
}

/// Identifiant d'une réservation.
///
/// Émis **avant** que quoi que ce soit ne parte vers le module : c'est la clé
/// sur laquelle repose toute la déduplication du commit.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash, PartialOrd, Ord)]
pub struct ReservationId(u128);

impl ReservationId {
    /// Construit un identifiant.
    #[must_use]
    pub const fn from_u128(value: u128) -> Self {
        Self(value)
    }
    /// La valeur sous-jacente.
    #[must_use]
    pub const fn as_u128(self) -> u128 {
        self.0
    }
}

/// D'où vient un Pokémon et par quelles mains il est passé.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Provenance {
    /// Le dresseur qui l'a déposé dans le pool.
    pub depositor: TrainerId,
    /// Quand il a été déposé.
    pub deposited_at: Timestamp,
    /// Les dresseurs qui l'ont possédé avant, du plus ancien au plus récent.
    pub previous: Vec<TrainerId>,
}

/// Un Pokémon tel que le pool le conserve.
///
/// Les octets sont gardés à l'identique, comme dans `relink-protocol` : on
/// stocke ce que la cartouche a envoyé, jamais une reconstruction.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Pokemon {
    /// Les octets bruts du Pokémon d'équipe.
    pub bytes: [u8; PARTY_POKEMON_LEN],
    /// Son surnom, tel que stocké sur la cartouche d'origine.
    pub nickname: gen1::Name,
    /// Le nom de son dresseur d'origine.
    pub original_trainer: gen1::Name,
}

/// Où en est une entrée du pool.
///
/// Une entrée quitte l'état [`EntryState::Available`] à la **réservation**, pas
/// au commit : sinon deux joueurs pourraient réserver la même. Elle n'y revient
/// que par expiration, et jamais après qu'un commit a été tenté.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum EntryState {
    /// Dans le pool, personne ne l'a réservée.
    Available,
    /// Réservée, en attente d'être remise à une cartouche.
    Reserved {
        /// La réservation qui la tient.
        reservation: ReservationId,
        /// Au-delà de cet instant, l'entrée retourne au pool — **mais seulement
        /// si aucun module n'en a accusé réception**.
        expires_at: Timestamp,
        /// Un module a-t-il accusé réception de cette réservation ?
        ///
        /// Tant que c'est faux, rien n'a pu atteindre une cartouche et
        /// l'expiration est sûre. Dès que c'est vrai, l'entrée ne revient plus
        /// jamais au pool toute seule : un module hors ligne ayant déjà remis
        /// le Pokémon est indiscernable d'un module qui n'a rien reçu.
        delivered: bool,
    },
    /// Remise à une cartouche, confirmée par le module.
    Committed {
        /// La réservation qui l'a consommée.
        reservation: ReservationId,
        /// Quand le commit a été enregistré.
        at: Timestamp,
    },
    /// Un commit a été tenté sans qu'on sache s'il a abouti.
    ///
    /// L'entrée reste consommée : c'est l'arbitrage de la spec §7.1, où l'on
    /// choisit la perte plutôt que le risque de duplication. Un traitement de
    /// litige manuel, hors périmètre, s'appuiera sur cet état.
    Abandoned {
        /// La réservation concernée.
        reservation: ReservationId,
        /// Quand l'ambiguïté a été constatée.
        at: Timestamp,
    },
}

impl EntryState {
    /// La réservation qui gouverne cet état, s'il y en a une.
    #[must_use]
    pub const fn reservation(&self) -> Option<ReservationId> {
        match self {
            Self::Available => None,
            Self::Reserved { reservation, .. }
            | Self::Committed { reservation, .. }
            | Self::Abandoned { reservation, .. } => Some(*reservation),
        }
    }
}

/// Un Pokémon déposé dans le pool, avec son état et sa provenance.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct PoolEntry {
    /// Son identifiant.
    pub id: EntryId,
    /// Le Pokémon lui-même.
    pub pokemon: Pokemon,
    /// D'où il vient.
    pub provenance: Provenance,
    /// Où il en est.
    pub state: EntryState,
}

impl PoolEntry {
    /// Vrai si cette entrée peut encore être réservée par quelqu'un.
    #[must_use]
    pub const fn is_claimable(&self) -> bool {
        matches!(self.state, EntryState::Available)
    }
}
```

Dans `crates/application/src/lib.rs`, remplacer la mention « Rien n'est encore implémenté » par la description du crate et ajouter :

```rust
pub mod domain;
```

- [ ] **Step 5: Lancer les tests et vérifier qu'ils passent**

Run: `cargo test -p relink-application`
Expected: PASS, 4 tests.

Run: `cargo clippy --workspace --all-targets` puis `cargo fmt --all --check`
Expected: aucun avertissement.

- [ ] **Step 6: Commit**

```bash
git add crates/application/
git commit -m "feat(application): types du domaine du pool"
```

---

### Task 3: Les ports

Les traits, et surtout **leurs contrats**. Une signature ne suffit pas : le commit idempotent de la tâche 7 n'est correct que si `PoolRepository` garantit certaines opérations atomiques. Ces garanties sont des obligations pour tout adaptateur, et elles se documentent ici.

**Files:**
- Create: `crates/application/src/ports.rs`
- Modify: `crates/application/src/lib.rs`
- Test: aucun — un trait sans implémentation ne se teste pas. La tâche 4 le fera à travers les doublures.

**Interfaces:**
- Produces, depuis `relink_application::ports` :
  - `pub trait Clock { async fn now(&self) -> Timestamp; }`
  - `pub trait IdSource { async fn next_entry_id(&self) -> EntryId; async fn next_reservation_id(&self) -> ReservationId; }`
  - `pub trait PoolRepository` avec `insert`, `get`, `list_claimable`, `claim`, `record_commit`, `record_abandon`, `expire_due`
  - `pub trait LegalityChecker { async fn is_legal(&self, pokemon: &Pokemon) -> Result<bool, PortError>; }`
  - `pub trait ModuleTransport { async fn push_reservation(&self, module: ModuleId, reservation: ReservationId, pokemon: &Pokemon) -> Result<(), PortError>; }`
  - `pub trait Notifier { async fn entry_claimed(&self, depositor: &TrainerId, entry: EntryId) -> Result<(), PortError>; }`
  - `pub struct ModuleId(u128)`, `pub struct PortError`, `pub enum ClaimOutcome`, `pub enum CommitOutcome`

- [ ] **Step 1: Écrire les traits et leurs contrats**

`crates/application/src/ports.rs` :

```rust
//! Les ports du service : ce que le domaine attend du monde extérieur.
//!
//! Aucun n'a d'implémentation dans ce crate, et c'est le principe. Mais un
//! port n'est pas qu'une signature : plusieurs portent des **garanties**
//! d'atomicité sans lesquelles le commit idempotent du domaine serait faux.
//! Ces garanties sont documentées trait par trait et lient tout adaptateur.

use crate::domain::{EntryId, Pokemon, PoolEntry, ReservationId, Timestamp, TrainerId};

/// Une panne du monde extérieur : base injoignable, courtier hors service,
/// service de légalité en erreur.
///
/// Le domaine ne cherche jamais à l'interpréter. Il ne distingue que deux
/// choses : l'opération a eu lieu, ou on ne sait pas.
#[derive(Clone, PartialEq, Eq, Debug, thiserror::Error)]
#[error("le port a échoué : {message}")]
pub struct PortError {
    /// Ce que l'adaptateur a à en dire, pour les journaux.
    pub message: String,
}

impl PortError {
    /// Construit une erreur de port.
    #[must_use]
    pub fn new(message: impl Into<String>) -> Self {
        Self { message: message.into() }
    }
}

/// Identifiant d'un module physique appairé à un compte.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub struct ModuleId(u128);

impl ModuleId {
    /// Construit un identifiant de module.
    #[must_use]
    pub const fn from_u128(value: u128) -> Self {
        Self(value)
    }
    /// La valeur sous-jacente.
    #[must_use]
    pub const fn as_u128(self) -> u128 {
        self.0
    }
}

/// Ce qu'une tentative de réservation a donné.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ClaimOutcome {
    /// L'entrée était disponible, elle est maintenant réservée.
    Claimed,
    /// Quelqu'un d'autre l'a prise en premier.
    AlreadyTaken,
    /// Aucune entrée ne porte cet identifiant.
    NotFound,
}

/// Ce qu'un enregistrement de commit a donné.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CommitOutcome {
    /// Premier enregistrement pour cette réservation.
    Recorded,
    /// Cette réservation avait déjà été commitée. **Ce n'est pas une erreur** :
    /// c'est le cas nominal d'un rejeu de message.
    AlreadyRecorded,
    /// Aucune réservation ne porte cet identifiant, ou elle ne tient plus
    /// d'entrée.
    Unknown,
}

/// L'horloge. Le domaine ne lit jamais l'heure autrement.
pub trait Clock {
    /// L'instant courant.
    async fn now(&self) -> Timestamp;
}

/// La source des identifiants. Le domaine ne tire jamais d'aléa lui-même.
pub trait IdSource {
    /// Un identifiant d'entrée qui n'a jamais été rendu.
    async fn next_entry_id(&self) -> EntryId;
    /// Un identifiant de réservation qui n'a jamais été rendu.
    async fn next_reservation_id(&self) -> ReservationId;
}

/// Le stockage du pool.
///
/// # Garanties exigées de tout adaptateur
///
/// Trois opérations doivent être **atomiques**, c'est-à-dire indivisibles vis-à-vis
/// de tout appel concurrent. Sans elles, deux joueurs peuvent réserver la même
/// entrée, ou un rejeu de message peut commiter deux fois :
///
/// - [`Self::claim`] doit vérifier la disponibilité **et** poser la réservation
///   en une seule opération. Un adaptateur qui lirait puis écrirait
///   séparément serait faux.
/// - [`Self::record_commit`] doit distinguer le premier enregistrement d'un
///   rejeu, en une seule opération, et rendre le même verdict pour toujours.
/// - [`Self::expire_due`] doit réclamer chaque réservation échue une seule
///   fois, même appelée en parallèle depuis plusieurs processus.
///
/// Le domaine ne peut pas compenser leur absence, et les doublures de test de
/// la tâche 4 vérifient qu'il ne le tente pas.
pub trait PoolRepository {
    /// Ajoute une entrée disponible au pool.
    async fn insert(&self, entry: PoolEntry) -> Result<(), PortError>;

    /// L'entrée portant cet identifiant, si elle existe.
    async fn get(&self, id: EntryId) -> Result<Option<PoolEntry>, PortError>;

    /// Les entrées actuellement réservables.
    async fn list_claimable(&self) -> Result<Vec<PoolEntry>, PortError>;

    /// **Atomique.** Réserve l'entrée si et seulement si elle est disponible.
    async fn claim(
        &self,
        id: EntryId,
        reservation: ReservationId,
        expires_at: Timestamp,
    ) -> Result<ClaimOutcome, PortError>;

    /// **Atomique et idempotente.** Enregistre qu'une réservation a été
    /// commitée sur une cartouche.
    async fn record_commit(
        &self,
        reservation: ReservationId,
        at: Timestamp,
    ) -> Result<CommitOutcome, PortError>;

    /// **Atomique et idempotente.** Marque une réservation comme abandonnée :
    /// un commit a été tenté sans qu'on sache s'il a abouti, et l'entrée reste
    /// consommée.
    async fn record_abandon(
        &self,
        reservation: ReservationId,
        at: Timestamp,
    ) -> Result<CommitOutcome, PortError>;

    /// **Atomique et idempotente.** Enregistre qu'un module a accusé réception
    /// d'une réservation. À partir de là, l'entrée ne peut plus expirer.
    ///
    /// L'accusé vient du module, jamais du courtier de messages : qu'un
    /// courtier ait accepté un message ne dit rien de ce que le module en a
    /// fait.
    async fn record_delivery(&self, reservation: ReservationId) -> Result<CommitOutcome, PortError>;

    /// **Atomique.** Rend au pool les entrées dont la réservation a expiré à
    /// cet instant, et rend leurs identifiants.
    ///
    /// N'en font **jamais** partie : une entrée dont un commit ou un abandon a
    /// été enregistré, ni une entrée dont un module a accusé réception. Seule
    /// une réservation qui n'est jamais parvenue à un module peut expirer.
    async fn expire_due(&self, now: Timestamp) -> Result<Vec<EntryId>, PortError>;
}

/// Le contrôle de légalité, derrière lequel vivra PKHeX.Core.
pub trait LegalityChecker {
    /// Vrai si ce Pokémon aurait pu être obtenu par le jeu normal.
    async fn is_legal(&self, pokemon: &Pokemon) -> Result<bool, PortError>;
}

/// Le lien vers un module physique.
pub trait ModuleTransport {
    /// Pousse vers un module le Pokémon qu'il devra remettre à la cartouche.
    async fn push_reservation(
        &self,
        module: ModuleId,
        reservation: ReservationId,
        pokemon: &Pokemon,
    ) -> Result<(), PortError>;
}

/// Les notifications vers les joueurs.
pub trait Notifier {
    /// Prévient un déposant que son Pokémon a été pris.
    async fn entry_claimed(
        &self,
        depositor: &TrainerId,
        entry: EntryId,
    ) -> Result<(), PortError>;
}
```

Dans `crates/application/src/lib.rs`, ajouter :

```rust
pub mod ports;
```

- [ ] **Step 2: Vérifier que tout compile**

Run: `cargo test -p relink-application` puis `cargo clippy --workspace --all-targets` puis `cargo fmt --all --check`
Expected: les 4 tests de la tâche 2 passent toujours, aucun avertissement.

Si clippy réclame `#[allow(async_fn_in_trait)]`, **ne l'ajoute pas sans réfléchir** : l'avertissement porte sur l'absence de borne `Send`, qui n'a de sens que pour un usage multi-tâches. Les cas d'usage étant génériques et jamais `dyn`, la borne est décidée par l'appelant. Documente ce choix à côté de l'attribut.

- [ ] **Step 3: Commit**

```bash
git add crates/application/src/
git commit -m "feat(application): déclarer les ports et leurs contrats d'atomicité"
```

---

### Task 4: Les doublures en mémoire

Elles servent tous les tests des tâches 5 à 10. Elles doivent être **fidèles aux contrats de la tâche 3** — notamment l'atomicité — sinon les tests valideraient un domaine qui ne marcherait pas en production.

**Files:**
- Create: `crates/application/src/testing.rs`
- Modify: `crates/application/src/lib.rs`
- Test: `crates/application/tests/testing_doubles.rs`

**Interfaces:**
- Produces, depuis `relink_application::testing` :
  - `pub struct FixedClock` avec `new(Timestamp)`, `advance(u64)`, `set(Timestamp)` ; elle dérive `Clone` en partageant son état sous `Arc`, les tâches 7 et 10 en ont besoin
  - `pub struct SequentialIds` avec `new()`
  - `pub struct InMemoryPool` avec `new()`, `fail_next(PortError)`, `len()`
  - `pub struct StubLegality` avec `accepting()`, `rejecting()`
  - `pub struct RecordingTransport` avec `new()`, `pushed()`, `fail_next(PortError)`
  - `pub struct RecordingNotifier` avec `new()`, `notified()`, `fail_next(PortError)`

- [ ] **Step 1: Écrire le test qui échoue**

`crates/application/tests/testing_doubles.rs` :

```rust
//! Les doublures doivent respecter les contrats des ports, sinon les tests
//! des tâches suivantes ne prouveraient rien.

use pollster::block_on;
use relink_application::domain::{EntryId, ReservationId, Timestamp};
use relink_application::ports::{ClaimOutcome, CommitOutcome, Clock, IdSource, PoolRepository};
use relink_application::testing::{FixedClock, InMemoryPool, SequentialIds};

mod util;
use util::sample_entry;

fn at(ms: u64) -> Timestamp {
    Timestamp::from_millis(ms)
}

#[test]
fn l_horloge_de_test_n_avance_que_quand_on_l_avance() {
    let clock = FixedClock::new(at(100));
    assert_eq!(block_on(clock.now()), at(100));
    assert_eq!(block_on(clock.now()), at(100));
    clock.advance(50);
    assert_eq!(block_on(clock.now()), at(150));
}

#[test]
fn les_identifiants_ne_se_repetent_jamais() {
    let ids = SequentialIds::new();
    let a = block_on(ids.next_entry_id());
    let b = block_on(ids.next_entry_id());
    let r = block_on(ids.next_reservation_id());
    assert_ne!(a, b);
    assert_ne!(r.as_u128(), 0);
}

#[test]
fn une_entree_ne_peut_etre_reservee_qu_une_fois() {
    let pool = InMemoryPool::new();
    let id = EntryId::from_u128(1);
    block_on(pool.insert(sample_entry(id))).expect("insertion");

    let first = block_on(pool.claim(id, ReservationId::from_u128(10), at(1000)));
    let second = block_on(pool.claim(id, ReservationId::from_u128(11), at(1000)));

    assert_eq!(first.expect("premier"), ClaimOutcome::Claimed);
    assert_eq!(second.expect("second"), ClaimOutcome::AlreadyTaken);
}

#[test]
fn reserver_une_entree_inexistante_le_dit() {
    let pool = InMemoryPool::new();
    let outcome = block_on(pool.claim(EntryId::from_u128(99), ReservationId::from_u128(1), at(1)));
    assert_eq!(outcome.expect("appel"), ClaimOutcome::NotFound);
}

#[test]
fn le_commit_est_idempotent() {
    let pool = InMemoryPool::new();
    let id = EntryId::from_u128(1);
    let res = ReservationId::from_u128(10);
    block_on(pool.insert(sample_entry(id))).expect("insertion");
    block_on(pool.claim(id, res, at(1000))).expect("réservation");

    assert_eq!(block_on(pool.record_commit(res, at(5))).expect("1"), CommitOutcome::Recorded);
    assert_eq!(
        block_on(pool.record_commit(res, at(9))).expect("2"),
        CommitOutcome::AlreadyRecorded
    );
}

#[test]
fn une_reservation_commitee_n_expire_jamais() {
    let pool = InMemoryPool::new();
    let id = EntryId::from_u128(1);
    let res = ReservationId::from_u128(10);
    block_on(pool.insert(sample_entry(id))).expect("insertion");
    block_on(pool.claim(id, res, at(1000))).expect("réservation");
    block_on(pool.record_commit(res, at(500))).expect("commit");

    let expired = block_on(pool.expire_due(at(9_999))).expect("expiration");
    assert!(expired.is_empty(), "une entrée commitée ne doit jamais revenir au pool");
}

#[test]
fn une_reservation_abandonnee_n_expire_jamais_non_plus() {
    let pool = InMemoryPool::new();
    let id = EntryId::from_u128(1);
    let res = ReservationId::from_u128(10);
    block_on(pool.insert(sample_entry(id))).expect("insertion");
    block_on(pool.claim(id, res, at(1000))).expect("réservation");
    block_on(pool.record_abandon(res, at(500))).expect("abandon");

    let expired = block_on(pool.expire_due(at(9_999))).expect("expiration");
    assert!(expired.is_empty(), "l'arbitrage de la spec §7.1 choisit la perte");
}

#[test]
fn une_reservation_echue_rend_l_entree_une_seule_fois() {
    let pool = InMemoryPool::new();
    let id = EntryId::from_u128(1);
    block_on(pool.insert(sample_entry(id))).expect("insertion");
    block_on(pool.claim(id, ReservationId::from_u128(10), at(1000))).expect("réservation");

    let first = block_on(pool.expire_due(at(1001))).expect("1");
    let second = block_on(pool.expire_due(at(1002))).expect("2");
    assert_eq!(first, vec![id]);
    assert!(second.is_empty(), "une expiration ne doit être réclamée qu'une fois");
}

#[test]
fn la_panne_injectee_ne_frappe_qu_une_fois() {
    use relink_application::ports::PortError;
    let pool = InMemoryPool::new();
    pool.fail_next(PortError::new("base injoignable"));
    assert!(block_on(pool.insert(sample_entry(EntryId::from_u128(1)))).is_err());
    assert!(block_on(pool.insert(sample_entry(EntryId::from_u128(2)))).is_ok());
}
```

Et le module partagé `crates/application/tests/util/mod.rs` :

```rust
//! Constructeurs communs aux tests d'intégration.

use relink_application::domain::{
    EntryId, EntryState, Pokemon, PoolEntry, Provenance, Timestamp, TrainerId,
};
use relink_protocol::gen1::{NAME_LEN, Name, PARTY_POKEMON_LEN};

/// Un nom de dresseur quelconque mais valide.
#[must_use]
pub fn some_name() -> Name {
    let mut raw = [0x50u8; NAME_LEN];
    raw[0] = 0x91;
    raw[1] = 0xA4;
    raw[2] = 0xA3;
    Name::from_bytes(raw)
}

/// Un Pokémon d'espèce Mew, éligible en Gen 1.
#[must_use]
pub fn some_pokemon() -> Pokemon {
    let mut bytes = [0u8; PARTY_POKEMON_LEN];
    bytes[0x00] = 0x15;
    bytes[0x08] = 1;
    Pokemon { bytes, nickname: some_name(), original_trainer: some_name() }
}

/// Une entrée disponible, portant l'identifiant donné.
#[must_use]
pub fn sample_entry(id: EntryId) -> PoolEntry {
    PoolEntry {
        id,
        pokemon: some_pokemon(),
        provenance: Provenance {
            depositor: TrainerId { name: some_name(), number: 1234 },
            deposited_at: Timestamp::from_millis(0),
            previous: Vec::new(),
        },
        state: EntryState::Available,
    }
}
```

- [ ] **Step 2: Lancer le test et vérifier qu'il échoue**

Run: `cargo test -p relink-application --test testing_doubles`
Expected: FAIL — `unresolved import relink_application::testing`

- [ ] **Step 3: Écrire l'implémentation**

`crates/application/src/testing.rs` : implémenter les six doublures. Points imposés, chacun correspondant à un test ci-dessus :

- Toutes utilisent `std::sync::Mutex` pour leur état interne et prennent `&self`, jamais `&mut self` : les ports sont déclarés en `&self`.
- `InMemoryPool::claim` vérifie l'état **et** l'écrit sous le même verrou. Deux verrous successifs seraient une violation du contrat, et le test « une entrée ne peut être réservée qu'une fois » l'attraperait.
- `record_commit` et `record_abandon` consultent une table `ReservationId -> ()` des réservations déjà tranchées : premier appel `Recorded`, suivants `AlreadyRecorded`, y compris entre commit et abandon — **une réservation ne se tranche qu'une fois, dans un sens ou dans l'autre**.
- `expire_due` ne rend que les entrées dont l'état est `Reserved`, dont `delivered` est **faux**, et dont `expires_at <= now` ; elle les repasse à `Available` sous le même verrou. Une entrée `Committed`, `Abandoned`, ou `Reserved` avec `delivered` vrai n'en fait **jamais** partie.
- `record_delivery` passe `delivered` à vrai, et rend `AlreadyRecorded` si c'était déjà le cas ou si la réservation est déjà tranchée.
- `fail_next` empile une erreur consommée par le prochain appel, quel qu'il soit. C'est ce qui permettra à la tâche 10 de simuler les pannes.
- `FixedClock` n'avance que sur `advance` ou `set`. Elle ne lit jamais `SystemTime`.
- `SequentialIds` rend 1, 2, 3… pour chaque famille d'identifiants, et ne rend jamais 0 — ce qui rend les traces de test lisibles et les scénarios reproductibles.

Ce module est compilé dans le crate et non derrière `#[cfg(test)]`, parce que les tests d'intégration sont des crates séparés. Documente-le comme réservé aux tests.

- [ ] **Step 4: Lancer les tests et vérifier qu'ils passent**

Run: `cargo test -p relink-application`
Expected: PASS, 9 tests de plus.

Run: `cargo clippy --workspace --all-targets` puis `cargo fmt --all --check`
Expected: aucun avertissement.

- [ ] **Step 5: Commit**

```bash
git add crates/application/
git commit -m "feat(application): doublures en mémoire respectant les contrats des ports"
```

---

### Task 5: Déposer un Pokémon dans le pool

**Files:**
- Create: `crates/application/src/deposit.rs`
- Modify: `crates/application/src/lib.rs`
- Test: `crates/application/tests/deposit.rs`

**Interfaces:**
- Produces : `pub struct deposit::Deposit<R, L, C, I>` avec `new(pool, legality, clock, ids)` et `async fn execute(&self, request: DepositRequest) -> Result<EntryId, DepositError>`, `pub struct DepositRequest { pub depositor: TrainerId, pub pokemon: Pokemon }`, `pub enum DepositError { Illegal, Ineligible(usize), Port(PortError) }`

- [ ] **Step 1: Écrire le test qui échoue**

`crates/application/tests/deposit.rs` :

```rust
//! Le cas d'usage de dépôt.

use pollster::block_on;
use relink_application::deposit::{Deposit, DepositError, DepositRequest};
use relink_application::domain::Timestamp;
use relink_application::ports::PoolRepository;
use relink_application::testing::{FixedClock, InMemoryPool, SequentialIds, StubLegality};

mod util;
use util::{some_name, some_pokemon};

fn request() -> DepositRequest {
    use relink_application::domain::TrainerId;
    DepositRequest {
        depositor: TrainerId { name: some_name(), number: 1234 },
        pokemon: some_pokemon(),
    }
}

#[test]
fn un_depot_valide_entre_dans_le_pool() {
    let pool = InMemoryPool::new();
    let uc = Deposit::new(&pool, StubLegality::accepting(), FixedClock::new(Timestamp::from_millis(7)), SequentialIds::new());

    let id = block_on(uc.execute(request())).expect("dépôt accepté");

    let stored = block_on(pool.get(id)).expect("lecture").expect("présente");
    assert!(stored.is_claimable());
    assert_eq!(stored.provenance.deposited_at, Timestamp::from_millis(7));
    assert_eq!(stored.provenance.depositor.number, 1234);
    assert!(stored.provenance.previous.is_empty());
    assert_eq!(stored.pokemon.bytes, some_pokemon().bytes);
}

#[test]
fn un_pokemon_illegal_est_refuse_et_n_entre_pas() {
    let pool = InMemoryPool::new();
    let uc = Deposit::new(&pool, StubLegality::rejecting(), FixedClock::new(Timestamp::from_millis(0)), SequentialIds::new());

    assert_eq!(block_on(uc.execute(request())), Err(DepositError::Illegal));
    assert_eq!(pool.len(), 0, "rien ne doit entrer dans le pool");
}

#[test]
fn une_panne_du_stockage_remonte_sans_perdre_le_pokemon() {
    use relink_application::ports::PortError;
    let pool = InMemoryPool::new();
    pool.fail_next(PortError::new("base injoignable"));
    let uc = Deposit::new(&pool, StubLegality::accepting(), FixedClock::new(Timestamp::from_millis(0)), SequentialIds::new());

    match block_on(uc.execute(request())) {
        Err(DepositError::Port(_)) => {}
        other => panic!("attendu une erreur de port, obtenu {other:?}"),
    }
    assert_eq!(pool.len(), 0);
}

#[test]
fn deux_depots_recoivent_des_identifiants_distincts() {
    let pool = InMemoryPool::new();
    let uc = Deposit::new(&pool, StubLegality::accepting(), FixedClock::new(Timestamp::from_millis(0)), SequentialIds::new());

    let a = block_on(uc.execute(request())).expect("premier");
    let b = block_on(uc.execute(request())).expect("second");
    assert_ne!(a, b);
    assert_eq!(pool.len(), 2);
}

#[test]
fn la_legalite_est_verifiee_avant_toute_ecriture() {
    // Le stockage échouerait s'il était touché ; il ne doit pas l'être.
    use relink_application::ports::PortError;
    let pool = InMemoryPool::new();
    pool.fail_next(PortError::new("ne devrait jamais être appelée"));
    let uc = Deposit::new(&pool, StubLegality::rejecting(), FixedClock::new(Timestamp::from_millis(0)), SequentialIds::new());

    assert_eq!(block_on(uc.execute(request())), Err(DepositError::Illegal));
}
```

- [ ] **Step 2: Lancer le test et vérifier qu'il échoue**

Run: `cargo test -p relink-application --test deposit`
Expected: FAIL — `unresolved import relink_application::deposit`

- [ ] **Step 3: Écrire l'implémentation**

`crates/application/src/deposit.rs`, structure imposée :

- `Deposit<R, L, C, I>` détient ses quatre ports par valeur ou par référence, au choix de l'implémenteur, du moment que les tests ci-dessus compilent tels quels.
- `execute` procède **dans cet ordre**, et le dernier test le vérifie : contrôle de légalité, puis lecture de l'heure et des identifiants, puis écriture. Rien n'est écrit tant que le Pokémon n'est pas accepté.
- `DepositError` dérive `Debug`, `PartialEq`, `Eq` et `thiserror::Error`. La variante `Ineligible(usize)` n'est pas produite par ce cas d'usage — elle appartient au retrait, où l'on sait vers quelle cartouche le Pokémon descend. **Ne l'utilise pas ici** ; elle est déclarée parce que la tâche 6 la remplira.
- La provenance d'un dépôt initial a une chaîne `previous` vide : le déposant est le premier maillon connu.

- [ ] **Step 4: Lancer les tests et vérifier qu'ils passent**

Run: `cargo test -p relink-application`
Expected: PASS, 5 tests de plus.

Run: `cargo clippy --workspace --all-targets` puis `cargo fmt --all --check`
Expected: aucun avertissement.

- [ ] **Step 5: Commit**

```bash
git add crates/application/
git commit -m "feat(application): cas d'usage de dépôt dans le pool"
```

---

### Task 6: Réserver une entrée du pool

**Files:**
- Create: `crates/application/src/reserve.rs`
- Modify: `crates/application/src/lib.rs`
- Test: `crates/application/tests/reserve.rs`

**Interfaces:**
- Produces : `pub struct reserve::Reserve<R, T, N, C, I>` avec `new(pool, transport, notifier, clock, ids, ttl_millis)` et `async fn execute(&self, request: ReserveRequest) -> Result<ReservationId, ReserveError>`, `pub struct ReserveRequest { pub entry: EntryId, pub module: ModuleId, pub claimant: TrainerId }`, `pub enum ReserveError { AlreadyTaken, NotFound, Port(PortError) }`

- [ ] **Step 1: Écrire le test qui échoue**

`crates/application/tests/reserve.rs` :

```rust
//! Le cas d'usage de réservation.

use pollster::block_on;
use relink_application::domain::{EntryId, EntryState, Timestamp, TrainerId};
use relink_application::ports::{ModuleId, PoolRepository, PortError};
use relink_application::reserve::{Reserve, ReserveError, ReserveRequest};
use relink_application::testing::{
    FixedClock, InMemoryPool, RecordingNotifier, RecordingTransport, SequentialIds,
};

mod util;
use util::{sample_entry, some_name};

const TTL: u64 = 3_600_000;

fn request(entry: EntryId) -> ReserveRequest {
    ReserveRequest {
        entry,
        module: ModuleId::from_u128(7),
        claimant: TrainerId { name: some_name(), number: 999 },
    }
}

#[test]
fn une_reservation_sort_l_entree_du_pool_et_pousse_vers_le_module() {
    let pool = InMemoryPool::new();
    let transport = RecordingTransport::new();
    let notifier = RecordingNotifier::new();
    let id = EntryId::from_u128(1);
    block_on(pool.insert(sample_entry(id))).expect("insertion");

    let uc = Reserve::new(&pool, &transport, &notifier, FixedClock::new(Timestamp::from_millis(1_000)), SequentialIds::new(), TTL);
    let reservation = block_on(uc.execute(request(id))).expect("réservation");

    let stored = block_on(pool.get(id)).expect("lecture").expect("présente");
    assert!(!stored.is_claimable(), "l'entrée quitte le pool à la réservation");
    // `delivered` reste faux : pousser vers le courtier n'est pas un accusé de
    // réception du module. Tant qu'il est faux, l'expiration reste sûre.
    assert_eq!(
        stored.state,
        EntryState::Reserved {
            reservation,
            expires_at: Timestamp::from_millis(1_000 + TTL),
            delivered: false,
        }
    );
    assert_eq!(transport.pushed(), vec![(ModuleId::from_u128(7), reservation)]);
}

#[test]
fn le_deposant_est_prevenu_que_son_pokemon_a_ete_pris() {
    let pool = InMemoryPool::new();
    let transport = RecordingTransport::new();
    let notifier = RecordingNotifier::new();
    let id = EntryId::from_u128(1);
    block_on(pool.insert(sample_entry(id))).expect("insertion");

    let uc = Reserve::new(&pool, &transport, &notifier, FixedClock::new(Timestamp::from_millis(0)), SequentialIds::new(), TTL);
    block_on(uc.execute(request(id))).expect("réservation");

    assert_eq!(notifier.notified(), vec![id]);
}

#[test]
fn deux_joueurs_ne_peuvent_pas_reserver_la_meme_entree() {
    let pool = InMemoryPool::new();
    let transport = RecordingTransport::new();
    let notifier = RecordingNotifier::new();
    let id = EntryId::from_u128(1);
    block_on(pool.insert(sample_entry(id))).expect("insertion");

    let uc = Reserve::new(&pool, &transport, &notifier, FixedClock::new(Timestamp::from_millis(0)), SequentialIds::new(), TTL);
    block_on(uc.execute(request(id))).expect("première");

    assert_eq!(block_on(uc.execute(request(id))), Err(ReserveError::AlreadyTaken));
    assert_eq!(transport.pushed().len(), 1, "rien ne doit partir vers le module la seconde fois");
}

#[test]
fn reserver_une_entree_inexistante_le_dit() {
    let pool = InMemoryPool::new();
    let uc = Reserve::new(&pool, &RecordingTransport::new(), &RecordingNotifier::new(), FixedClock::new(Timestamp::from_millis(0)), SequentialIds::new(), TTL);
    assert_eq!(block_on(uc.execute(request(EntryId::from_u128(42)))), Err(ReserveError::NotFound));
}

#[test]
fn l_entree_est_reservee_avant_que_quoi_que_ce_soit_ne_parte_vers_le_module() {
    // Si le module recevait le Pokémon avant que l'entrée ne soit sortie du
    // pool, un second joueur pourrait la réserver dans l'intervalle.
    let pool = InMemoryPool::new();
    let transport = RecordingTransport::new();
    let id = EntryId::from_u128(1);
    block_on(pool.insert(sample_entry(id))).expect("insertion");
    transport.fail_next(PortError::new("courtier hors service"));

    let uc = Reserve::new(&pool, &transport, &RecordingNotifier::new(), FixedClock::new(Timestamp::from_millis(0)), SequentialIds::new(), TTL);
    let outcome = block_on(uc.execute(request(id)));

    assert!(matches!(outcome, Err(ReserveError::Port(_))));
    let stored = block_on(pool.get(id)).expect("lecture").expect("présente");
    assert!(
        !stored.is_claimable(),
        "l'entrée reste réservée : elle reviendra par expiration, jamais par annulation"
    );
}

#[test]
fn une_notification_en_panne_n_annule_pas_la_reservation() {
    let pool = InMemoryPool::new();
    let transport = RecordingTransport::new();
    let notifier = RecordingNotifier::new();
    let id = EntryId::from_u128(1);
    block_on(pool.insert(sample_entry(id))).expect("insertion");
    notifier.fail_next(PortError::new("service de push hors ligne"));

    let uc = Reserve::new(&pool, &transport, &notifier, FixedClock::new(Timestamp::from_millis(0)), SequentialIds::new(), TTL);
    let reservation = block_on(uc.execute(request(id)));

    assert!(reservation.is_ok(), "prévenir le déposant est accessoire, pas critique");
}
```

Note : ce dernier test exige que `RecordingNotifier` porte aussi `fail_next`. Ajoute-le à la doublure de la tâche 4 si elle ne l'a pas.

- [ ] **Step 2: Lancer le test et vérifier qu'il échoue**

Run: `cargo test -p relink-application --test reserve`
Expected: FAIL — `unresolved import relink_application::reserve`

- [ ] **Step 3: Écrire l'implémentation**

Ordre imposé dans `execute`, et deux tests le vérifient :

1. `pool.claim(...)` d'abord, avec l'identifiant de réservation émis **avant** l'appel. C'est ce qui rend la déduplication du commit possible.
2. Selon le verdict : `Claimed` continue, `AlreadyTaken` et `NotFound` rendent l'erreur correspondante **sans rien pousser**.
3. `transport.push_reservation(...)` ensuite. Si elle échoue, l'erreur remonte mais **l'entrée reste réservée** : elle reviendra au pool par expiration, jamais par une annulation qui ouvrirait une fenêtre de duplication.
4. `notifier.entry_claimed(...)` en dernier, et **son échec est ignoré**. Prévenir le déposant est accessoire.

- [ ] **Step 4: Lancer les tests et vérifier qu'ils passent**

Run: `cargo test -p relink-application`
Expected: PASS, 6 tests de plus.

Run: `cargo clippy --workspace --all-targets` puis `cargo fmt --all --check`
Expected: aucun avertissement.

- [ ] **Step 5: Commit**

```bash
git add crates/application/
git commit -m "feat(application): cas d'usage de réservation"
```

---

### Task 7: Le commit idempotent

Le cœur de la spec §7. Un module rejoue son journal à la reconnexion, et MQTT peut livrer deux fois : le même message reçu dix fois doit produire exactement le même effet qu'une fois.

**Files:**
- Create: `crates/application/src/commit.rs`
- Modify: `crates/application/src/lib.rs`
- Test: `crates/application/tests/commit.rs`

**Interfaces:**
- Produces : `pub struct commit::Commit<R, C>` avec `new(pool, clock)`, `async fn confirm(&self, reservation: ReservationId) -> Result<CommitVerdict, PortError>`, `async fn abandon(&self, reservation: ReservationId) -> Result<CommitVerdict, PortError>`, `pub enum CommitVerdict { Applied, AlreadySettled, Unknown }`

- [ ] **Step 1: Écrire le test qui échoue**

`crates/application/tests/commit.rs` :

```rust
//! Le commit, seul endroit du service où l'on peut détruire des données.

use pollster::block_on;
use relink_application::commit::{Commit, CommitVerdict};
use relink_application::domain::{EntryId, EntryState, ReservationId, Timestamp};
use relink_application::ports::PoolRepository;
use relink_application::testing::{FixedClock, InMemoryPool, SequentialIds};

mod util;
use util::sample_entry;

fn reserved_pool(id: EntryId, res: ReservationId) -> InMemoryPool {
    let pool = InMemoryPool::new();
    block_on(pool.insert(sample_entry(id))).expect("insertion");
    block_on(pool.claim(id, res, Timestamp::from_millis(10_000))).expect("réservation");
    pool
}

#[test]
fn confirmer_consomme_l_entree() {
    let (id, res) = (EntryId::from_u128(1), ReservationId::from_u128(10));
    let pool = reserved_pool(id, res);
    let uc = Commit::new(&pool, FixedClock::new(Timestamp::from_millis(50)));

    assert_eq!(block_on(uc.confirm(res)).expect("commit"), CommitVerdict::Applied);
    let stored = block_on(pool.get(id)).expect("lecture").expect("présente");
    assert_eq!(stored.state, EntryState::Committed { reservation: res, at: Timestamp::from_millis(50) });
}

#[test]
fn rejouer_le_meme_commit_ne_change_rien() {
    let (id, res) = (EntryId::from_u128(1), ReservationId::from_u128(10));
    let pool = reserved_pool(id, res);
    let clock = FixedClock::new(Timestamp::from_millis(50));
    let uc = Commit::new(&pool, clock.clone());

    assert_eq!(block_on(uc.confirm(res)).expect("1"), CommitVerdict::Applied);
    clock.set(Timestamp::from_millis(9_999));
    assert_eq!(block_on(uc.confirm(res)).expect("2"), CommitVerdict::AlreadySettled);
    assert_eq!(block_on(uc.confirm(res)).expect("3"), CommitVerdict::AlreadySettled);

    let stored = block_on(pool.get(id)).expect("lecture").expect("présente");
    assert_eq!(
        stored.state,
        EntryState::Committed { reservation: res, at: Timestamp::from_millis(50) },
        "l'instant du premier commit ne doit pas bouger"
    );
}

#[test]
fn abandonner_laisse_l_entree_consommee() {
    let (id, res) = (EntryId::from_u128(1), ReservationId::from_u128(10));
    let pool = reserved_pool(id, res);
    let uc = Commit::new(&pool, FixedClock::new(Timestamp::from_millis(50)));

    assert_eq!(block_on(uc.abandon(res)).expect("abandon"), CommitVerdict::Applied);
    let stored = block_on(pool.get(id)).expect("lecture").expect("présente");
    assert_eq!(stored.state, EntryState::Abandoned { reservation: res, at: Timestamp::from_millis(50) });
    assert!(!stored.is_claimable(), "on choisit la perte, pas la duplication");
}

#[test]
fn on_ne_peut_pas_abandonner_ce_qui_est_deja_confirme() {
    let (id, res) = (EntryId::from_u128(1), ReservationId::from_u128(10));
    let pool = reserved_pool(id, res);
    let uc = Commit::new(&pool, FixedClock::new(Timestamp::from_millis(50)));

    block_on(uc.confirm(res)).expect("commit");
    assert_eq!(block_on(uc.abandon(res)).expect("abandon"), CommitVerdict::AlreadySettled);

    let stored = block_on(pool.get(id)).expect("lecture").expect("présente");
    assert!(matches!(stored.state, EntryState::Committed { .. }), "une réservation ne se tranche qu'une fois");
}

#[test]
fn on_ne_peut_pas_confirmer_ce_qui_est_deja_abandonne() {
    let (id, res) = (EntryId::from_u128(1), ReservationId::from_u128(10));
    let pool = reserved_pool(id, res);
    let uc = Commit::new(&pool, FixedClock::new(Timestamp::from_millis(50)));

    block_on(uc.abandon(res)).expect("abandon");
    assert_eq!(block_on(uc.confirm(res)).expect("commit"), CommitVerdict::AlreadySettled);
}

#[test]
fn une_reservation_inconnue_le_dit_sans_rien_casser() {
    let pool = InMemoryPool::new();
    let uc = Commit::new(&pool, FixedClock::new(Timestamp::from_millis(0)));
    assert_eq!(block_on(uc.confirm(ReservationId::from_u128(404))).expect("appel"), CommitVerdict::Unknown);
}

#[test]
fn une_entree_commitee_ne_revient_jamais_au_pool() {
    let (id, res) = (EntryId::from_u128(1), ReservationId::from_u128(10));
    let pool = reserved_pool(id, res);
    let uc = Commit::new(&pool, FixedClock::new(Timestamp::from_millis(50)));
    block_on(uc.confirm(res)).expect("commit");

    // Bien après l'échéance de la réservation.
    let expired = block_on(pool.expire_due(Timestamp::from_millis(u64::MAX))).expect("expiration");
    assert!(expired.is_empty());
    assert!(!block_on(pool.get(id)).expect("lecture").expect("présente").is_claimable());
}
```

Note : `FixedClock` doit être `Clone` pour ce fichier. Ajoute la dérive à la doublure de la tâche 4 si elle manque, en partageant l'état sous `Arc`.

- [ ] **Step 2: Lancer le test et vérifier qu'il échoue**

Run: `cargo test -p relink-application --test commit`
Expected: FAIL — `unresolved import relink_application::commit`

- [ ] **Step 3: Écrire l'implémentation**

`crates/application/src/commit.rs`. Ce cas d'usage est **mince par construction** : il lit l'heure et délègue au port, qui porte la garantie d'atomicité. Ce n'est pas un manque d'ambition, c'est le seul endroit où la garantie peut vivre — un domaine qui lirait puis écrirait serait faux dès qu'un second processus tournerait en parallèle.

- `confirm` appelle `pool.record_commit(reservation, clock.now())` et traduit `CommitOutcome` en `CommitVerdict`.
- `abandon` fait de même avec `record_abandon`.
- **Aucun chemin ne rend une entrée au pool.** Relis la contrainte globale : si ton implémentation contient un appel qui repasse un état à `Available`, elle est fausse.

Documente en tête du module que ce cas d'usage est délibérément mince et pourquoi.

- [ ] **Step 4: Lancer les tests et vérifier qu'ils passent**

Run: `cargo test -p relink-application`
Expected: PASS, 7 tests de plus.

Run: `cargo clippy --workspace --all-targets` puis `cargo fmt --all --check`
Expected: aucun avertissement.

- [ ] **Step 5: Commit**

```bash
git add crates/application/
git commit -m "feat(application): commit idempotent d'une réservation"
```

---

### Task 8: L'expiration des réservations

**Files:**
- Create: `crates/application/src/expiry.rs`
- Modify: `crates/application/src/lib.rs`
- Test: `crates/application/tests/expiry.rs`

**Interfaces:**
- Produces : `pub struct expiry::ExpireReservations<R, C>` avec `new(pool, clock)` et `async fn run(&self) -> Result<Vec<EntryId>, PortError>`

- [ ] **Step 1: Écrire le test qui échoue**

`crates/application/tests/expiry.rs` :

```rust
//! L'expiration des réservations : le seul chemin qui rend une entrée au pool.

use pollster::block_on;
use relink_application::commit::Commit;
use relink_application::domain::{EntryId, ReservationId, Timestamp};
use relink_application::expiry::ExpireReservations;
use relink_application::ports::PoolRepository;
use relink_application::testing::{FixedClock, InMemoryPool};

mod util;
use util::sample_entry;

fn at(ms: u64) -> Timestamp {
    Timestamp::from_millis(ms)
}

#[test]
fn une_reservation_echue_rend_l_entree_au_pool() {
    let pool = InMemoryPool::new();
    let id = EntryId::from_u128(1);
    block_on(pool.insert(sample_entry(id))).expect("insertion");
    block_on(pool.claim(id, ReservationId::from_u128(10), at(1_000))).expect("réservation");

    let clock = FixedClock::new(at(1_001));
    let released = block_on(ExpireReservations::new(&pool, clock).run()).expect("expiration");

    assert_eq!(released, vec![id]);
    assert!(block_on(pool.get(id)).expect("lecture").expect("présente").is_claimable());
}

#[test]
fn une_reservation_encore_valide_n_est_pas_touchee() {
    let pool = InMemoryPool::new();
    let id = EntryId::from_u128(1);
    block_on(pool.insert(sample_entry(id))).expect("insertion");
    block_on(pool.claim(id, ReservationId::from_u128(10), at(1_000))).expect("réservation");

    let released = block_on(ExpireReservations::new(&pool, FixedClock::new(at(999))).run()).expect("expiration");

    assert!(released.is_empty());
    assert!(!block_on(pool.get(id)).expect("lecture").expect("présente").is_claimable());
}

#[test]
fn l_echeance_exacte_n_expire_pas_encore() {
    let pool = InMemoryPool::new();
    let id = EntryId::from_u128(1);
    block_on(pool.insert(sample_entry(id))).expect("insertion");
    block_on(pool.claim(id, ReservationId::from_u128(10), at(1_000))).expect("réservation");

    // La convention du contrat de `expire_due` est `expires_at <= now`.
    let released = block_on(ExpireReservations::new(&pool, FixedClock::new(at(1_000))).run()).expect("expiration");
    assert_eq!(released, vec![id], "à l'échéance exacte, l'entrée revient");
}

#[test]
fn une_entree_commitee_n_expire_jamais_meme_tres_tard() {
    let pool = InMemoryPool::new();
    let id = EntryId::from_u128(1);
    let res = ReservationId::from_u128(10);
    block_on(pool.insert(sample_entry(id))).expect("insertion");
    block_on(pool.claim(id, res, at(1_000))).expect("réservation");
    block_on(Commit::new(&pool, FixedClock::new(at(500))).confirm(res)).expect("commit");

    let released = block_on(ExpireReservations::new(&pool, FixedClock::new(at(u64::MAX))).run()).expect("expiration");
    assert!(released.is_empty());
}

#[test]
fn une_entree_abandonnee_n_expire_jamais_non_plus() {
    let pool = InMemoryPool::new();
    let id = EntryId::from_u128(1);
    let res = ReservationId::from_u128(10);
    block_on(pool.insert(sample_entry(id))).expect("insertion");
    block_on(pool.claim(id, res, at(1_000))).expect("réservation");
    block_on(Commit::new(&pool, FixedClock::new(at(500))).abandon(res)).expect("abandon");

    let released = block_on(ExpireReservations::new(&pool, FixedClock::new(at(u64::MAX))).run()).expect("expiration");
    assert!(released.is_empty(), "on choisit la perte : elle ne revient jamais");
}

#[test]
fn une_entree_remise_a_un_module_n_expire_jamais() {
    // Le scénario qui a fait corriger la spec §7.2 : le module a accusé
    // réception, puis a disparu. Il a peut-être déjà remis le Pokémon à la
    // cartouche — on ne le saura jamais. Rendre l'entrée au pool serait une
    // duplication.
    let pool = InMemoryPool::new();
    let id = EntryId::from_u128(1);
    let res = ReservationId::from_u128(10);
    block_on(pool.insert(sample_entry(id))).expect("insertion");
    block_on(pool.claim(id, res, at(1_000))).expect("réservation");
    block_on(pool.record_delivery(res)).expect("accusé de réception");

    let released = block_on(ExpireReservations::new(&pool, FixedClock::new(at(u64::MAX))).run()).expect("expiration");
    assert!(released.is_empty(), "on choisit la perte plutôt que la duplication");
    assert!(!block_on(pool.get(id)).expect("lecture").expect("présente").is_claimable());
}

#[test]
fn une_entree_jamais_parvenue_a_un_module_expire_normalement() {
    let pool = InMemoryPool::new();
    let id = EntryId::from_u128(1);
    block_on(pool.insert(sample_entry(id))).expect("insertion");
    block_on(pool.claim(id, ReservationId::from_u128(10), at(1_000))).expect("réservation");
    // Pas d'accusé de réception : rien n'a jamais atteint de cartouche.

    let released = block_on(ExpireReservations::new(&pool, FixedClock::new(at(1_001))).run()).expect("expiration");
    assert_eq!(released, vec![id]);
}

#[test]
fn une_entree_rendue_peut_etre_reservee_a_nouveau() {
    let pool = InMemoryPool::new();
    let id = EntryId::from_u128(1);
    block_on(pool.insert(sample_entry(id))).expect("insertion");
    block_on(pool.claim(id, ReservationId::from_u128(10), at(1_000))).expect("première");
    block_on(ExpireReservations::new(&pool, FixedClock::new(at(2_000))).run()).expect("expiration");

    let outcome = block_on(pool.claim(id, ReservationId::from_u128(11), at(3_000))).expect("seconde");
    assert_eq!(outcome, relink_application::ports::ClaimOutcome::Claimed);
}
```

- [ ] **Step 2: Lancer le test et vérifier qu'il échoue**

Run: `cargo test -p relink-application --test expiry`
Expected: FAIL — `unresolved import relink_application::expiry`

- [ ] **Step 3: Écrire l'implémentation**

Aussi mince que la tâche 7, et pour la même raison : lire l'heure, appeler `pool.expire_due(now)`, rendre la liste. Toute la difficulté est dans le contrat du port.

Le troisième test fige la convention de borne (`expires_at <= now` expire) ; si `InMemoryPool` de la tâche 4 a implémenté l'inverse, **c'est la doublure qu'il faut corriger**, et alors le contrat de `expire_due` dans `ports.rs` doit être reformulé pour lever l'ambiguïté.

- [ ] **Step 4: Lancer les tests et vérifier qu'ils passent**

Run: `cargo test -p relink-application`
Expected: PASS, 6 tests de plus.

Run: `cargo clippy --workspace --all-targets` puis `cargo fmt --all --check`
Expected: aucun avertissement.

- [ ] **Step 5: Commit**

```bash
git add crates/application/
git commit -m "feat(application): expiration des réservations"
```

---

### Task 9: L'échange direct, par appariement

Spec §7.3, décision B : l'échange direct **n'est pas un protocole distinct**. C'est un dépôt et un retrait appariés, réservés l'un à l'autre. Il n'existe aucun chemin de commit à deux cartouches.

**Files:**
- Create: `crates/application/src/pairing.rs`
- Modify: `crates/application/src/lib.rs`
- Test: `crates/application/tests/pairing.rs`

**Interfaces:**
- Produces : `pub struct pairing::OfferDirectTrade<R, L, C, I>` avec `new(pool, legality, clock, ids)` et `async fn execute(&self, request: DirectTradeRequest) -> Result<EntryId, DepositError>`, `pub struct DirectTradeRequest { pub depositor: TrainerId, pub pokemon: Pokemon, pub reserved_for: TrainerId }`, plus `pub fn domain::PoolEntry::is_offered_to(&self, trainer: &TrainerId) -> bool` et le champ `pub reserved_for: Option<TrainerId>` sur `PoolEntry`

- [ ] **Step 1: Écrire le test qui échoue**

`crates/application/tests/pairing.rs` :

```rust
//! L'échange direct : un dépôt et un retrait appariés, rien d'autre.

use pollster::block_on;
use relink_application::domain::TrainerId;
use relink_application::pairing::{DirectTradeRequest, OfferDirectTrade};
use relink_application::ports::PoolRepository;
use relink_application::testing::{FixedClock, InMemoryPool, SequentialIds, StubLegality};
use relink_application::domain::Timestamp;

mod util;
use util::{some_name, some_pokemon};

fn trainer(number: u16) -> TrainerId {
    TrainerId { name: some_name(), number }
}

fn offer(to: u16) -> DirectTradeRequest {
    DirectTradeRequest {
        depositor: trainer(1),
        pokemon: some_pokemon(),
        reserved_for: trainer(to),
    }
}

fn use_case(pool: &InMemoryPool) -> OfferDirectTrade<'_, InMemoryPool, StubLegality, FixedClock, SequentialIds> {
    OfferDirectTrade::new(pool, StubLegality::accepting(), FixedClock::new(Timestamp::from_millis(0)), SequentialIds::new())
}

#[test]
fn une_offre_directe_entre_dans_le_pool_reservee_a_son_destinataire() {
    let pool = InMemoryPool::new();
    let id = block_on(use_case(&pool).execute(offer(2))).expect("offre");

    let stored = block_on(pool.get(id)).expect("lecture").expect("présente");
    assert!(stored.is_claimable(), "elle reste prenable, mais par une seule personne");
    assert!(stored.is_offered_to(&trainer(2)));
    assert!(!stored.is_offered_to(&trainer(3)));
}

#[test]
fn un_depot_ordinaire_est_offert_a_tout_le_monde() {
    use relink_application::deposit::{Deposit, DepositRequest};
    let pool = InMemoryPool::new();
    let uc = Deposit::new(&pool, StubLegality::accepting(), FixedClock::new(Timestamp::from_millis(0)), SequentialIds::new());
    let id = block_on(uc.execute(DepositRequest { depositor: trainer(1), pokemon: some_pokemon() })).expect("dépôt");

    let stored = block_on(pool.get(id)).expect("lecture").expect("présente");
    assert!(stored.is_offered_to(&trainer(2)));
    assert!(stored.is_offered_to(&trainer(3)));
}

#[test]
fn une_offre_directe_n_apparait_pas_dans_le_pool_ouvert_des_autres() {
    let pool = InMemoryPool::new();
    block_on(use_case(&pool).execute(offer(2))).expect("offre");

    let visible: Vec<_> = block_on(pool.list_claimable())
        .expect("liste")
        .into_iter()
        .filter(|e| e.is_offered_to(&trainer(3)))
        .collect();
    assert!(visible.is_empty());
}

#[test]
fn un_pokemon_illegal_est_refuse_meme_en_offre_directe() {
    use relink_application::deposit::DepositError;
    let pool = InMemoryPool::new();
    let uc = OfferDirectTrade::new(&pool, StubLegality::rejecting(), FixedClock::new(Timestamp::from_millis(0)), SequentialIds::new());
    assert_eq!(block_on(uc.execute(offer(2))), Err(DepositError::Illegal));
    assert_eq!(pool.len(), 0);
}

#[test]
fn on_ne_peut_pas_s_offrir_un_pokemon_a_soi_meme() {
    use relink_application::deposit::DepositError;
    let pool = InMemoryPool::new();
    let request = DirectTradeRequest { depositor: trainer(1), pokemon: some_pokemon(), reserved_for: trainer(1) };
    assert!(block_on(use_case(&pool).execute(request)).is_err(), "un échange avec soi-même n'en est pas un");
    assert_eq!(pool.len(), 0);
    let _ = DepositError::Illegal; // garde l'import utilisé si l'implémenteur choisit une autre variante
}
```

Note : le dernier test laisse à l'implémenteur le choix de la variante d'erreur, mais elle doit être **distincte** de `Illegal` — un Pokémon parfaitement légal offert à soi-même n'est pas un problème de légalité. Ajoute la variante à `DepositError`, documente-la, et resserre le test sur elle une fois choisie.

- [ ] **Step 2: Lancer le test et vérifier qu'il échoue**

Run: `cargo test -p relink-application --test pairing`
Expected: FAIL — `unresolved import relink_application::pairing`

- [ ] **Step 3: Écrire l'implémentation**

- Ajouter `pub reserved_for: Option<TrainerId>` à `PoolEntry` (tâche 2) et la méthode `is_offered_to`, qui rend vrai si `reserved_for` est `None` ou si elle désigne ce dresseur. Mettre à jour les constructeurs des tests et de `Deposit`, qui posent `None`.
- `OfferDirectTrade` réutilise le chemin du dépôt et ne s'en distingue que par ce champ et par le refus de l'auto-offre. **Ne duplique pas la logique de dépôt** : si les deux cas d'usage divergent au-delà d'un champ, c'est que le design a dérivé de la décision B.
- Le filtrage effectif du pool par destinataire est une affaire de requête, donc d'adaptateur. Le domaine fournit le prédicat ; il ne fait pas de requête.

- [ ] **Step 4: Lancer les tests et vérifier qu'ils passent**

Run: `cargo test -p relink-application`
Expected: PASS, 5 tests de plus, et **les tests des tâches 2, 5 et 6 continuent de passer**. Si l'ajout du champ les casse, corrige-les — mais leur intention ne doit pas changer.

Run: `cargo clippy --workspace --all-targets` puis `cargo fmt --all --check`
Expected: aucun avertissement.

- [ ] **Step 5: Commit**

```bash
git add crates/application/
git commit -m "feat(application): échange direct par appariement dans le pool"
```

---

### Task 10: Le test d'invariant — jamais de duplication

La tâche qui justifie tout le reste. Elle n'ajoute aucun code de production : elle **énumère les interruptions possibles** et vérifie une seule propriété.

**L'invariant, énoncé précisément :** à tout instant, pour toute entrée, la somme du nombre d'exemplaires détenus par des cartouches et du nombre d'exemplaires réservables dans le pool ne dépasse jamais 1.

**Files:**
- Create: `crates/application/tests/invariant.rs`

**Interfaces:**
- Consumes: toute l'API publique produite par les tâches 2 à 9.
- Produces: rien.

- [ ] **Step 1: Écrire le modèle et l'énumération**

`crates/application/tests/invariant.rs`. Structure imposée :

```rust
//! *Jamais de duplication* : l'unique propriété qui protège le pool.
//!
//! On simule le cycle de vie complet d'une entrée en insérant une interruption
//! à chaque point où le monde réel peut lâcher, et on vérifie qu'aucune
//! séquence ne produit deux exemplaires du même Pokémon.

use pollster::block_on;
use relink_application::commit::{Commit, CommitVerdict};
use relink_application::domain::{EntryId, EntryState, ReservationId, Timestamp};
use relink_application::expiry::ExpireReservations;
use relink_application::ports::PoolRepository;
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
    let mut world = World { module_acknowledged: false, cartridge_holds: false };

    for event in sequence {
        match event {
            Event::ModuleAcknowledged => {
                block_on(pool.record_delivery(res)).expect("accusé de réception");
                world.module_acknowledged = true;
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
    if matches!(entry.state, EntryState::Committed { .. } | EntryState::Abandoned { .. }) {
        assert!(!entry.is_claimable(), "une réservation tranchée ne revient jamais au pool");
    }
}
```

- [ ] **Step 2: Écrire l'énumération exhaustive**

Ajouter au même fichier une énumération de **toutes les séquences** d'événements jusqu'à une longueur de 4, répétitions comprises — c'est ce qui couvre les rejeux et les redémarrages en cascade :

```rust
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
    assert_eq!(sequences, 7 + 49 + 343 + 2401, "l'énumération doit être exhaustive");
}

#[test]
fn le_scenario_le_plus_dangereux_est_couvert_explicitement() {
    // Celui qui a fait corriger la spec §7.2 : la cartouche a reçu, le serveur
    // ne le saura jamais, et le temps passe.
    replay(&[Event::ModuleAcknowledged, Event::CartridgeCommitted, Event::TtlElapsed]);
    replay(&[Event::ModuleAcknowledged, Event::CartridgeCommitted, Event::Abandoned, Event::TtlElapsed]);
    replay(&[Event::ModuleAcknowledged, Event::CartridgeCommitted, Event::TtlElapsed, Event::Abandoned]);
    replay(&[Event::ModuleAcknowledged, Event::CartridgeCommitted, Event::ModuleRebooted, Event::Redelivered]);
}
```

- [ ] **Step 3: Lancer le test**

Run: `cargo test -p relink-application --test invariant`

**Si un scénario échoue, c'est un vrai bug**, et le correctif porte sur la tâche d'origine — jamais sur le test, jamais sur l'invariant. Le message d'assertion imprime la séquence fautive : c'est elle qu'il faut comprendre avant de toucher à quoi que ce soit. Documente dans ton rapport le scénario trouvé et pourquoi ta correction est la bonne.

Ce test a **déjà trouvé un défaut, avant même d'être écrit** : en le concevant, on s'est aperçu que la justification de l'expiration donnée par la spec §7.2 était fausse, et que la séquence « le module accuse réception, la cartouche commite, le TTL expire » produisait une duplication. La spec et ce plan ont été corrigés en conséquence — c'est l'objet de la contrainte globale sur l'expiration et du port `record_delivery`.

Si le test trouve **autre chose**, applique le même traitement : ce n'est ni le test ni l'invariant qui ont tort. Comprends la séquence imprimée, remonte-la comme un défaut de conception à arbitrer, et ne la corrige pas en silence.

- [ ] **Step 4: Vérification complète**

Run: `cargo test --workspace`, `cargo clippy --workspace --all-targets`, `cargo fmt --all --check`, `cargo doc --workspace --no-deps`
Expected: tout passe, aucun avertissement.

Run: `cargo build -p relink-protocol --target thumbv7em-none-eabihf`
Expected: succès — la tâche 1 a modifié `protocol`, la contrainte `no_std` sans allocateur doit toujours tenir.

- [ ] **Step 5: Commit**

```bash
git add crates/application/tests/invariant.rs
git commit -m "test(application): invariant jamais de duplication sur toutes les interruptions"
```

---

## Ce que ce plan ne fait pas

- **Aucun adaptateur.** Ni base de données, ni MQTT, ni PKHeX, ni HTTP. Chacun aura sa spec et son plan, et devra prouver qu'il honore les contrats d'atomicité de la tâche 3.
- **Aucun compte utilisateur.** L'identité s'arrête au dresseur lu sur la cartouche, spec §6.3.
- **Aucun traitement de litige.** L'état `Abandoned` existe pour qu'un traitement futur s'y accroche ; ce traitement est hors périmètre v1.
- **La machine à états de l'échange**, qui reste conditionnée au sourçage du handshake et de la phase de sélection.
