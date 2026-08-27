# Machine à états de l'échange Gen 1 — plan d'implémentation

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Doter `relink-protocol` d'une session d'échange Gen 1 complète : la cartouche branchée sur le module doit pouvoir mener un échange Trade Center de bout en bout, puis un second dans la foulée.

**Architecture:** Une `Session` qui possède ses tampons et joue toujours le suiveur. `step(u8) -> Step` est un aiguillage sur une phase interne : chaque phase sait quel octet présenter et ce qui la termine. Les décisions que le module n'a pas encore — quel Pokémon offrir, accepter ou refuser — n'arrêtent rien : la session présente l'octet neutre `0x00` et `supply()` la réveille. Le codec de patch list, sans état, est écrit d'abord parce que le transport du bloc en dépend.

**Tech Stack:** Rust 2024, `#![no_std]` sans `alloc`, `proptest` en dépendance de développement uniquement.

**Spec:** `docs/superpowers/specs/2026-08-27-gen1-machine-a-etats-design.md`

**Sourçage:** `docs/protocol/gen1-link-protocol.md` — toute valeur d'octet de ce plan en vient. Une constante qui n'y figure pas est un bug, même si elle marche.

## Global Constraints

- `crates/protocol` est `#![no_std]` **sans `alloc`**. Toutes les structures de ce plan sont de taille fixe.
- `unsafe_code = "forbid"` et `missing_docs = "warn"` sont hérités du workspace. Tout élément public porte un commentaire de documentation, en français, comme le reste du crate.
- **Aucune fonction publique ne peut paniquer, quelle que soit l'entrée.** Pas d'indexation de tranche non vérifiée, pas d'arithmétique susceptible de déborder en `debug`. La tâche 6 le prouve ; les tâches 1 à 5 sont écrites en le sachant.
- **`step()` est infaillible et O(1).** Pas de `Result`, pas de boucle sur les 415 octets, pas de copie de bloc — sauf les deux copies explicitement autorisées et documentées ci-dessous (réarmement dans `supply`, matérialisation du bloc partenaire dans `partner_block`), qui sont hors du chemin critique de l'octet.
- Rust 1.85 minimum, édition 2024.
- Les tests vivent dans `crates/protocol/tests/`, n'utilisent que l'API publique, et sont nommés en français comme les tests existants.
- **Les valeurs d'octets des tests sont recopiées depuis `docs/protocol/gen1-link-protocol.md`, pas depuis le code de production.** Un test qui importerait la constante qu'il vérifie ne vérifierait rien.
- Aucune ROM, aucune sauvegarde, aucune donnée capturée sur cartouche tierce. Fixtures construites par le code.

## Structure des fichiers

| Fichier | Responsabilité |
|---|---|
| `crates/protocol/src/gen1/patch_list.rs` | Codec de patch list : construire, appliquer. Sans état. |
| `crates/protocol/src/session/mod.rs` | `Session`, `Step`, `Effect`, `Decision`, l'état, l'aiguillage de `step()`. |
| `crates/protocol/src/session/link.rs` | Négociation des rôles, acquittement, menu du Cable Club. |
| `crates/protocol/src/session/transfer.rs` | Préambule, graine, bloc d'échange, patch list. |
| `crates/protocol/src/session/table.rs` | Sélection, verdict, échange en cours, retour à la table. |
| `crates/protocol/tests/util/mod.rs` | Outils de test : pilotage d'une suite d'octets, fixtures, cartouche simulée. |

---

### Task 1: Codec de patch list

Le transport du bloc en dépend : `0xFE` est l'octet « pas de câble » du port série et ne peut pas traverser le fil tel quel. Sans ce codec, tout Pokémon dont les données contiennent un `0xFE` arrive corrompu.

**Files:**
- Create: `crates/protocol/src/gen1/patch_list.rs`
- Modify: `crates/protocol/src/gen1/mod.rs`
- Test: `crates/protocol/tests/gen1_patch_list.rs`

**Interfaces:**
- Consumes: `PARTY_CAPACITY`, `PARTY_POKEMON_LEN` de `gen1`.
- Produces:
  - `pub const PARTY_DATA_LEN: usize = 264;`
  - `pub const PATCH_LIST_LEN: usize = 189;`
  - `pub const NO_DATA: u8 = 0xFE;`
  - `pub const PART_TERMINATOR: u8 = 0xFF;`
  - `pub fn build(party: &mut [u8; PARTY_DATA_LEN]) -> [u8; PATCH_LIST_LEN];`
  - `pub fn apply(party: &mut [u8; PARTY_DATA_LEN], list: &[u8; PATCH_LIST_LEN]);`

- [ ] **Step 1: Écrire les tests qui échouent**

Dans `crates/protocol/tests/gen1_patch_list.rs` :

```rust
//! Tests du codec de patch list Gen 1.
//!
//! Valeurs et découpage sourcés dans `docs/protocol/gen1-link-protocol.md`,
//! sections « Patch list : principe » et « Patch list : zone couverte ».

use relink_protocol::gen1::patch_list::{
    self, PARTY_DATA_LEN, PATCH_LIST_LEN, PART_TERMINATOR, NO_DATA,
};

/// Sans aucun octet à corriger, la liste n'est que ses deux terminateurs.
#[test]
fn une_equipe_sans_octet_special_donne_une_liste_vide() {
    let mut party = [0u8; PARTY_DATA_LEN];
    let list = patch_list::build(&mut party);

    assert_eq!(list[0], PART_TERMINATOR, "fin de la première partie");
    assert_eq!(list[1], PART_TERMINATOR, "fin de la seconde partie");
    assert!(list[2..].iter().all(|&b| b == 0), "le reste est du remplissage");
    assert_eq!(party, [0u8; PARTY_DATA_LEN], "rien à corriger, rien de touché");
}

/// Une position de la première partie est notée incrémentée de un, et
/// l'octet part sur le fil en 0xFF.
#[test]
fn la_premiere_partie_note_la_position_incrementee() {
    let mut party = [0u8; PARTY_DATA_LEN];
    party[0] = NO_DATA;
    party[0x0A] = NO_DATA;
    let list = patch_list::build(&mut party);

    assert_eq!(list[0], 0x01, "position 0 notée 1");
    assert_eq!(list[1], 0x0B, "position 0x0A notée 0x0B");
    assert_eq!(list[2], PART_TERMINATOR);
    assert_eq!(list[3], PART_TERMINATOR);
    assert_eq!(party[0], 0xFF, "l'octet corrigé part en 0xFF");
    assert_eq!(party[0x0A], 0xFF);
}

/// La frontière entre les deux parties : 0xFB est la dernière position de la
/// première, 0xFC la première de la seconde, notée par rapport à la base.
#[test]
fn la_frontiere_entre_les_deux_parties_est_a_la_bonne_position() {
    let mut party = [0u8; PARTY_DATA_LEN];
    party[0xFB] = NO_DATA;
    party[0xFC] = NO_DATA;
    let list = patch_list::build(&mut party);

    assert_eq!(list[0], 0xFC, "0xFB est la dernière position de la partie 1");
    assert_eq!(list[1], PART_TERMINATOR);
    assert_eq!(list[2], 0x01, "0xFC est la première de la partie 2, notée 1");
    assert_eq!(list[3], PART_TERMINATOR);
}

/// La dernière position couvrable, 0x107, est bien dans la seconde partie.
#[test]
fn la_derniere_position_de_l_equipe_est_couverte() {
    let mut party = [0u8; PARTY_DATA_LEN];
    party[PARTY_DATA_LEN - 1] = NO_DATA;
    let list = patch_list::build(&mut party);

    assert_eq!(list[0], PART_TERMINATOR, "rien dans la partie 1");
    assert_eq!(list[1], 0x0C, "0x107 notée 0x107 - 0xFB");
    assert_eq!(list[2], PART_TERMINATOR);
}

/// Aucune valeur écrite dans la liste ne peut valoir 0xFE : ce serait
/// indistinguable de l'octet « pas de câble ». C'est la raison d'être du
/// découpage en deux parties.
#[test]
fn aucune_valeur_de_liste_ne_vaut_l_octet_pas_de_cable() {
    let mut party = [NO_DATA; PARTY_DATA_LEN];
    let list = patch_list::build(&mut party);

    assert!(list.iter().all(|&b| b != NO_DATA));
}

/// L'aller-retour rend les octets d'origine à l'identique.
#[test]
fn l_aller_retour_rend_les_octets_d_origine() {
    let mut party = [0u8; PARTY_DATA_LEN];
    for (i, b) in party.iter_mut().enumerate() {
        *b = (i % 251) as u8;
    }
    party[3] = NO_DATA;
    party[0xFB] = NO_DATA;
    party[0xFC] = NO_DATA;
    party[PARTY_DATA_LEN - 1] = NO_DATA;
    let origine = party;

    let list = patch_list::build(&mut party);
    assert_ne!(party, origine, "les octets spéciaux ont été corrigés");

    patch_list::apply(&mut party, &list);
    assert_eq!(party, origine, "l'aller-retour est sans perte");
}

/// Une liste reçue absurde ne doit rien casser : les valeurs hors zone sont
/// ignorées, pas appliquées de travers.
#[test]
fn une_liste_recue_absurde_ne_casse_rien() {
    let mut party = [0u8; PARTY_DATA_LEN];
    let mut list = [0u8; PATCH_LIST_LEN];
    list[0] = 0x00; // remplissage prématuré, sans effet
    list[1] = PART_TERMINATOR;
    list[2] = 0xFD; // hors de la zone couverte par la seconde partie
    list[3] = PART_TERMINATOR;

    patch_list::apply(&mut party, &list);

    assert_eq!(party, [0u8; PARTY_DATA_LEN], "aucune position valide, rien de touché");
}
```

- [ ] **Step 2: Lancer les tests pour les voir échouer**

Run: `cargo test -p relink-protocol --test gen1_patch_list`
Expected: échec de compilation, `patch_list` n'existe pas.

- [ ] **Step 3: Écrire le codec**

Dans `crates/protocol/src/gen1/patch_list.rs` :

```rust
//! Codec de la patch list Gen 1.
//!
//! `0xFE` est l'octet « pas de câble » du port série : une cartouche qui le
//! reçoit peut le prendre pour une déconnexion. Le jeu ne l'envoie donc
//! jamais tel quel. Il le remplace par `0xFF` dans les données d'équipe et
//! transmet, juste après le bloc, la liste des positions ainsi corrigées.
//!
//! Sourcé dans `docs/protocol/gen1-link-protocol.md`, sections « Pourquoi une
//! patch list », « Patch list : principe » et « Patch list : zone couverte ».

use crate::gen1::{PARTY_CAPACITY, PARTY_POKEMON_LEN};

/// Taille de la zone couverte : les six emplacements de données d'équipe.
pub const PARTY_DATA_LEN: usize = PARTY_CAPACITY * PARTY_POKEMON_LEN;

/// Nombre d'octets de liste que le module présente sur le fil. La cadence
/// est donnée par la cartouche : au-delà de ce que contient la liste, le
/// module présente du remplissage, et ce qu'il n'a pas le temps d'envoyer
/// n'est jamais réclamé. Les sources ne s'accordent pas à l'octet près sur
/// cette longueur — voir « Patch list : longueur transmise ».
pub const PATCH_LIST_LEN: usize = 189;

/// L'octet « pas de câble ».
pub const NO_DATA: u8 = 0xFE;

/// Marque la fin de chacune des deux parties de la liste.
pub const PART_TERMINATOR: u8 = 0xFF;

/// Dernière position couverte par la première partie.
const PART_ONE_LAST: usize = 0xFB;

/// Construit la patch list des données d'équipe et corrige celles-ci sur
/// place : chaque `0xFE` devient `0xFF` et sa position rejoint la liste.
///
/// Les positions sont notées incrémentées de un dans la première partie, et
/// relativement à `0xFB` dans la seconde : aucune valeur écrite ne peut ainsi
/// valoir `0xFE`.
///
/// Une équipe pathologique — plus de positions à corriger que la liste ne
/// peut en porter — voit les positions surnuméraires corrigées sans être
/// notées : le fil reste sain, ces octets-là arrivent en `0xFF`. Le cas ne se
/// produit pas sur des données réelles ; il est borné plutôt que faillible,
/// parce que `step()` ne peut pas échouer.
#[must_use]
pub fn build(party: &mut [u8; PARTY_DATA_LEN]) -> [u8; PATCH_LIST_LEN] {
    let mut list = [0u8; PATCH_LIST_LEN];
    let mut written = 0usize;

    // Deux emplacements sont réservés aux deux terminateurs.
    let capacity = PATCH_LIST_LEN - 2;

    for position in 0..=PART_ONE_LAST {
        if party[position] == NO_DATA {
            party[position] = PART_TERMINATOR;
            if written < capacity {
                list[written] = (position + 1) as u8;
                written += 1;
            }
        }
    }
    list[written] = PART_TERMINATOR;
    written += 1;

    for position in (PART_ONE_LAST + 1)..PARTY_DATA_LEN {
        if party[position] == NO_DATA {
            party[position] = PART_TERMINATOR;
            if written < capacity + 1 {
                list[written] = (position - PART_ONE_LAST) as u8;
                written += 1;
            }
        }
    }
    list[written] = PART_TERMINATOR;

    list
}

/// Applique une patch list reçue : remet `0xFE` aux positions qu'elle
/// désigne. Toute valeur hors de la zone couverte est ignorée.
pub fn apply(party: &mut [u8; PARTY_DATA_LEN], list: &[u8; PATCH_LIST_LEN]) {
    let mut second_part = false;

    for &value in list {
        match value {
            PART_TERMINATOR if !second_part => second_part = true,
            PART_TERMINATOR => return,
            0 => {}
            _ => {
                let position = if second_part {
                    PART_ONE_LAST + value as usize
                } else {
                    value as usize - 1
                };
                if position < PARTY_DATA_LEN {
                    party[position] = NO_DATA;
                }
            }
        }
    }
}
```

Dans `crates/protocol/src/gen1/mod.rs`, exposer le module :

```rust
pub mod patch_list;
```

- [ ] **Step 4: Lancer les tests**

Run: `cargo test -p relink-protocol --test gen1_patch_list`
Expected: les sept tests passent.

- [ ] **Step 5: Commit**

```bash
git add crates/protocol/src/gen1/patch_list.rs crates/protocol/src/gen1/mod.rs crates/protocol/tests/gen1_patch_list.rs
git commit -m "feat(protocol): codec de patch list Gen 1"
```

---

### Task 2: Session et phases de lien

La session, ses tampons, son aiguillage, et les trois premières phases : négociation des rôles, menu du Cable Club, attente dans la salle d'échange.

**Files:**
- Create: `crates/protocol/src/session/mod.rs`
- Create: `crates/protocol/src/session/link.rs`
- Create: `crates/protocol/tests/util/mod.rs`
- Modify: `crates/protocol/src/lib.rs`
- Test: `crates/protocol/tests/session_link.rs`

**Interfaces:**
- Consumes: `TradeBlock`, `TRADE_BLOCK_LEN` de `gen1` ; `patch_list::{build, PARTY_DATA_LEN, PATCH_LIST_LEN}` de la tâche 1.
- Produces:
  - `pub struct Session` avec `pub fn gen1(offered: TradeBlock) -> Self`, `pub fn step(&mut self, incoming: u8) -> Step`, `pub fn supply(&mut self, decision: Decision)`, `pub fn partner_block(&self) -> Option<TradeBlock>`.
  - `pub struct Step { pub outgoing: u8, pub effect: Option<Effect> }`
  - `pub enum Effect { LinkEstablished, PartnerBlockReceived, OfferNeeded, PartnerOffered { index: u8 }, VerdictNeeded, TradeAgreed { offered: u8, received: u8 }, TableLeft, LinkBroken }`
  - `pub enum Decision { Offer(u8), Accept, Reject, Leave, Party(TradeBlock) }`
  - `pub(crate) enum Phase` avec les variants `Negotiating`, `Menu`, `Waiting`, `Preamble`, `Seed`, `Block`, `PatchHeader`, `PatchList`, `Select`, `Verdict`, `Trading`, `Broken`.
  - Dans `tests/util/mod.rs` : `pub fn feed(session: &mut Session, bytes: &[u8]) -> Vec<u8>`, `pub fn effects(session: &mut Session, bytes: &[u8]) -> Vec<Effect>`, `pub fn bloc_fixture(marqueur: u8) -> TradeBlock`.

- [ ] **Step 1: Écrire les tests qui échouent**

Dans `crates/protocol/tests/util/mod.rs` :

```rust
//! Outils partagés par les tests de session.

use relink_protocol::gen1::{TRADE_BLOCK_LEN, TradeBlock};
use relink_protocol::session::{Effect, Session};

/// Fait consommer une suite d'octets à la session et rend ce qu'elle a
/// présenté en retour, octet pour octet.
pub fn feed(session: &mut Session, bytes: &[u8]) -> Vec<u8> {
    bytes.iter().map(|&b| session.step(b).outgoing).collect()
}

/// Fait consommer une suite d'octets et rend les effets émis en chemin.
pub fn effects(session: &mut Session, bytes: &[u8]) -> Vec<Effect> {
    bytes.iter().filter_map(|&b| session.step(b).effect).collect()
}

/// Un bloc d'échange reconnaissable : chaque octet dérive du marqueur, ce qui
/// rend une confusion entre bloc sortant et bloc entrant visible en test.
pub fn bloc_fixture(marqueur: u8) -> TradeBlock {
    let mut raw = [0u8; TRADE_BLOCK_LEN];
    for (i, b) in raw.iter_mut().enumerate() {
        *b = marqueur.wrapping_add((i % 97) as u8);
    }
    raw[11] = 1; // un Pokémon dans l'équipe
    TradeBlock::from_bytes(raw)
}
```

Dans `crates/protocol/tests/session_link.rs` :

```rust
//! Tests des phases de lien : négociation des rôles, acquittement, menu du
//! Cable Club.
//!
//! Valeurs d'octets recopiées de `docs/protocol/gen1-link-protocol.md`.

mod util;

use relink_protocol::session::{Effect, Session};
use util::{bloc_fixture, effects, feed};

const MASTER: u8 = 0x01;
const SLAVE: u8 = 0x02;
const BLANK: u8 = 0x00;
const CONNECTED: u8 = 0x60;
const TRADE_CENTRE: u8 = 0xD4;
const COLOSSEUM: u8 = 0xD5;
const BREAK_LINK: u8 = 0xD6;
const HIGHLIGHT_FIRST: u8 = 0xD0;

fn session() -> Session {
    Session::gen1(bloc_fixture(0x10))
}

/// Le module est suiveur : il répond 0x02 à l'octet de leader, toujours.
#[test]
fn repond_suiveur_a_l_octet_de_leader() {
    let mut s = session();
    assert_eq!(feed(&mut s, &[MASTER, MASTER]), vec![SLAVE, SLAVE]);
}

/// Les octets neutres de la négociation sont renvoyés tels quels : les
/// sources ne s'accordent pas sur leur nombre, on ne les compte pas.
#[test]
fn renvoie_les_octets_neutres_sans_les_compter() {
    let mut s = session();
    assert_eq!(feed(&mut s, &[MASTER, BLANK, BLANK, BLANK]), vec![SLAVE, BLANK, BLANK, BLANK]);
}

/// L'octet de connexion établit le lien.
#[test]
fn l_octet_de_connexion_etablit_le_lien() {
    let mut s = session();
    let sortis = effects(&mut s, &[MASTER, BLANK, CONNECTED]);
    assert_eq!(sortis, vec![Effect::LinkEstablished]);
}

/// Dans le menu, le module renvoie ce qu'il reçoit : c'est le joueur qui
/// choisit, pas le module.
#[test]
fn le_menu_laisse_choisir_le_joueur() {
    let mut s = session();
    feed(&mut s, &[MASTER, BLANK, CONNECTED]);
    assert_eq!(feed(&mut s, &[HIGHLIGHT_FIRST]), vec![HIGHLIGHT_FIRST]);
}

/// Le Trade Center est le seul parcours implémenté : il fait avancer sans
/// rompre le lien.
#[test]
fn le_trade_center_ne_rompt_pas_le_lien() {
    let mut s = session();
    feed(&mut s, &[MASTER, BLANK, CONNECTED]);
    let sortis = effects(&mut s, &[TRADE_CENTRE]);
    assert!(sortis.is_empty(), "aucun effet, on entre simplement dans la salle");
}

/// Le Colosseum est reconnu et refusé proprement : les combats ne sont pas
/// dans le projet.
#[test]
fn le_colosseum_est_refuse_proprement() {
    let mut s = session();
    feed(&mut s, &[MASTER, BLANK, CONNECTED]);
    let mut s2 = Session::gen1(bloc_fixture(0x10));
    feed(&mut s2, &[MASTER, BLANK, CONNECTED]);

    assert_eq!(effects(&mut s, &[COLOSSEUM]), vec![Effect::LinkBroken]);
    assert_eq!(feed(&mut s2, &[COLOSSEUM]), vec![BREAK_LINK]);
}

/// L'annulation depuis le menu rompt le lien elle aussi.
#[test]
fn l_annulation_rompt_le_lien() {
    let mut s = session();
    feed(&mut s, &[MASTER, BLANK, CONNECTED]);
    assert_eq!(effects(&mut s, &[BREAK_LINK]), vec![Effect::LinkBroken]);
}

/// Une cartouche qui redémarre sa négociation retrouve un module qui la
/// suit, quel que soit l'endroit où il en était.
#[test]
fn une_negociation_qui_repart_est_suivie() {
    let mut s = session();
    feed(&mut s, &[MASTER, BLANK, CONNECTED, TRADE_CENTRE]);
    assert_eq!(feed(&mut s, &[MASTER]), vec![SLAVE]);
    assert_eq!(effects(&mut s, &[CONNECTED]), vec![Effect::LinkEstablished]);
}
```

- [ ] **Step 2: Lancer les tests pour les voir échouer**

Run: `cargo test -p relink-protocol --test session_link`
Expected: échec de compilation, `session` n'existe pas.

- [ ] **Step 3: Écrire la session et les phases de lien**

Dans `crates/protocol/src/session/mod.rs` :

```rust
//! Machine à états de l'échange par câble link.
//!
//! Le module joue toujours le suiveur : la cartouche fournit l'horloge et
//! dicte le rythme. `step()` est donc appelée à chaque octet reçu et doit
//! présenter l'octet sortant sans allouer, sans attendre et sans faillir.
//!
//! Déroulé et valeurs sourcés dans `docs/protocol/gen1-link-protocol.md`.

mod link;
mod table;
mod transfer;

use crate::gen1::patch_list::{self, PARTY_DATA_LEN, PATCH_LIST_LEN};
use crate::gen1::{TRADE_BLOCK_LEN, TradeBlock};

/// Décalage des données d'équipe dans le bloc d'échange : la zone que la
/// patch list couvre.
pub(crate) const OFF_PARTY_DATA: usize = 19;

/// Octet neutre : « rien à dire pour l'instant ». C'est lui que la session
/// présente tant qu'une décision manque.
pub(crate) const BLANK: u8 = 0x00;

pub(crate) const MASTER: u8 = 0x01;
pub(crate) const SLAVE: u8 = 0x02;
pub(crate) const PREAMBLE: u8 = 0xFD;

/// Les valeurs qui changent d'une génération à l'autre. La Gen 2 ajoutera sa
/// table sans rien déplacer.
#[derive(Clone, Copy)]
pub(crate) struct LinkBytes {
    pub connected: u8,
    pub trade_centre: u8,
    pub colosseum: u8,
    pub break_link: u8,
    pub select_base: u8,
    pub table_leave: u8,
    pub trade_reject: u8,
    pub trade_accept: u8,
}

pub(crate) const GEN1: LinkBytes = LinkBytes {
    connected: 0x60,
    trade_centre: 0xD4,
    colosseum: 0xD5,
    break_link: 0xD6,
    select_base: 0x60,
    table_leave: 0x6F,
    trade_reject: 0x61,
    trade_accept: 0x62,
};

/// Ce que la session présente en réponse à un octet, et ce qu'elle a à dire
/// au passage.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Step {
    /// L'octet à présenter avant le prochain front d'horloge.
    pub outgoing: u8,
    /// Au plus un événement par octet.
    pub effect: Option<Effect>,
}

/// Ce que la session a à signaler à l'application.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Effect {
    /// Le lien est établi, le menu du Cable Club s'affiche.
    LinkEstablished,
    /// L'équipe du partenaire est reçue et lisible par `partner_block`.
    PartnerBlockReceived,
    /// Il faut annoncer quel Pokémon le module propose.
    OfferNeeded,
    /// Le joueur a annoncé le sien.
    PartnerOffered { index: u8 },
    /// Il faut accepter ou refuser.
    VerdictNeeded,
    /// Les deux côtés ont accepté : l'échange a lieu.
    TradeAgreed { offered: u8, received: u8 },
    /// Le joueur a quitté la table et regagné la salle.
    TableLeft,
    /// Le lien est rompu.
    LinkBroken,
}

/// Ce que l'application fournit à une session qui attend.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Decision {
    /// Le Pokémon proposé, par sa position dans l'équipe.
    Offer(u8),
    /// Accepter l'échange annoncé.
    Accept,
    /// Le refuser, et retourner à la sélection.
    Reject,
    /// Quitter la table.
    Leave,
    /// Réarmer la session avec une nouvelle équipe. À fournir entre deux
    /// échanges — après `TradeAgreed` ou `TableLeft` — jamais pendant le
    /// transfert d'un bloc.
    Party(TradeBlock),
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Phase {
    Negotiating,
    Menu,
    Waiting,
    Preamble,
    Seed,
    Block,
    PatchHeader,
    PatchList,
    Select,
    Verdict,
    Trading,
    Broken,
}

/// Une session d'échange. Environ un kilo-octet, immobile, sans allocation.
pub struct Session {
    pub(crate) phase: Phase,
    pub(crate) bytes: LinkBytes,
    pub(crate) outgoing: [u8; TRADE_BLOCK_LEN],
    pub(crate) outgoing_patch: [u8; PATCH_LIST_LEN],
    pub(crate) incoming: [u8; TRADE_BLOCK_LEN],
    pub(crate) incoming_patch: [u8; PATCH_LIST_LEN],
    pub(crate) partner_ready: bool,
    pub(crate) cursor: u16,
    pub(crate) offer: Option<u8>,
    pub(crate) partner_offer: Option<u8>,
    pub(crate) verdict: Option<bool>,
    pub(crate) partner_verdict: Option<bool>,
    pub(crate) leaving: bool,
    pub(crate) announced: bool,
}

impl Session {
    /// Ouvre une session de première génération, avec l'équipe que le module
    /// présentera au joueur.
    #[must_use]
    pub fn gen1(offered: TradeBlock) -> Self {
        let mut session = Self {
            phase: Phase::Negotiating,
            bytes: GEN1,
            outgoing: [0u8; TRADE_BLOCK_LEN],
            outgoing_patch: [0u8; PATCH_LIST_LEN],
            incoming: [0u8; TRADE_BLOCK_LEN],
            incoming_patch: [0u8; PATCH_LIST_LEN],
            partner_ready: false,
            cursor: 0,
            offer: None,
            partner_offer: None,
            verdict: None,
            partner_verdict: None,
            leaving: false,
            announced: false,
        };
        session.arm(offered);
        session
    }

    /// Consomme un octet et présente le suivant. O(1), sans allocation,
    /// infaillible : un octet inattendu est une transition, jamais une faute.
    pub fn step(&mut self, incoming: u8) -> Step {
        match self.phase {
            Phase::Negotiating => self.step_negotiating(incoming),
            Phase::Menu => self.step_menu(incoming),
            Phase::Waiting => self.step_waiting(incoming),
            Phase::Preamble => self.step_preamble(incoming),
            Phase::Seed => self.step_seed(incoming),
            Phase::Block => self.step_block(incoming),
            Phase::PatchHeader => self.step_patch_header(incoming),
            Phase::PatchList => self.step_patch_list(incoming),
            Phase::Select => self.step_select(incoming),
            Phase::Verdict => self.step_verdict(incoming),
            Phase::Trading => self.step_trading(incoming),
            Phase::Broken => self.step_broken(incoming),
        }
    }

    /// Fournit une décision à une session qui attend.
    pub fn supply(&mut self, decision: Decision) {
        match decision {
            Decision::Offer(index) => self.offer = Some(index.min(5)),
            Decision::Accept => self.verdict = Some(true),
            Decision::Reject => self.verdict = Some(false),
            Decision::Leave => self.leaving = true,
            Decision::Party(block) => self.arm(block),
        }
    }

    /// L'équipe du partenaire, dès que `PartnerBlockReceived` a été émis.
    ///
    /// Rend une copie : 415 octets, hors du chemin critique de l'octet.
    #[must_use]
    pub fn partner_block(&self) -> Option<TradeBlock> {
        self.partner_ready.then(|| TradeBlock::from_bytes(self.incoming))
    }

    /// Charge l'équipe à présenter : corrige les octets « pas de câble » des
    /// données d'équipe et construit la patch list correspondante.
    fn arm(&mut self, block: TradeBlock) {
        self.outgoing = *block.as_bytes();
        let mut party = [0u8; PARTY_DATA_LEN];
        party.copy_from_slice(&self.outgoing[OFF_PARTY_DATA..OFF_PARTY_DATA + PARTY_DATA_LEN]);
        self.outgoing_patch = patch_list::build(&mut party);
        self.outgoing[OFF_PARTY_DATA..OFF_PARTY_DATA + PARTY_DATA_LEN].copy_from_slice(&party);
    }

    /// Présente un octet sans rien signaler.
    pub(crate) fn plain(&self, outgoing: u8) -> Step {
        Step { outgoing, effect: None }
    }

    /// Présente un octet et signale un événement.
    pub(crate) fn with(&self, outgoing: u8, effect: Effect) -> Step {
        Step { outgoing, effect: Some(effect) }
    }

    /// Repart d'une négociation : la cartouche a redémarré la sienne.
    pub(crate) fn restart(&mut self) -> Step {
        self.phase = Phase::Negotiating;
        self.cursor = 0;
        self.partner_ready = false;
        self.reset_round();
        self.plain(SLAVE)
    }

    /// Oublie tout ce qui appartenait à l'échange en cours.
    pub(crate) fn reset_round(&mut self) {
        self.offer = None;
        self.partner_offer = None;
        self.verdict = None;
        self.partner_verdict = None;
        self.leaving = false;
        self.announced = false;
    }
}
```

Dans `crates/protocol/src/session/link.rs` :

```rust
//! Négociation des rôles, acquittement de connexion, menu du Cable Club.
//!
//! Ces trois phases ont une règle commune : le module renvoie ce qu'il
//! reçoit. Les sources ne s'accordent pas sur le nombre d'octets neutres
//! échangés, et c'est le joueur qui choisit dans le menu — le module ne
//! compte rien et ne décide rien.

use super::{Effect, MASTER, PREAMBLE, Phase, SLAVE, Session, Step};

impl Session {
    pub(crate) fn step_negotiating(&mut self, incoming: u8) -> Step {
        if incoming == MASTER {
            return self.plain(SLAVE);
        }
        if incoming == self.bytes.connected {
            self.phase = Phase::Menu;
            return self.with(incoming, Effect::LinkEstablished);
        }
        self.plain(incoming)
    }

    pub(crate) fn step_menu(&mut self, incoming: u8) -> Step {
        if incoming == MASTER {
            return self.restart();
        }
        if incoming == self.bytes.trade_centre {
            self.phase = Phase::Waiting;
            return self.plain(incoming);
        }
        if incoming == self.bytes.colosseum || incoming == self.bytes.break_link {
            self.phase = Phase::Broken;
            return self.with(self.bytes.break_link, Effect::LinkBroken);
        }
        self.plain(incoming)
    }

    pub(crate) fn step_waiting(&mut self, incoming: u8) -> Step {
        if incoming == MASTER {
            return self.restart();
        }
        if incoming == PREAMBLE {
            self.phase = Phase::Preamble;
            self.cursor = 1;
            return self.plain(incoming);
        }
        self.plain(incoming)
    }

    pub(crate) fn step_broken(&mut self, incoming: u8) -> Step {
        if incoming == MASTER {
            return self.restart();
        }
        self.plain(self.bytes.break_link)
    }
}
```

Les phases de transfert et de table arrivent aux tâches 3 et 4 ; pour que le crate compile dès maintenant, créer `crates/protocol/src/session/transfer.rs` et `crates/protocol/src/session/table.rs` avec les huit méthodes restantes réduites à leur signature, chacune présentant l'octet neutre :

```rust
//! Phases de transfert. Corps écrits à la tâche 3.

use super::{BLANK, Session, Step};

impl Session {
    pub(crate) fn step_preamble(&mut self, _incoming: u8) -> Step { self.plain(BLANK) }
    pub(crate) fn step_seed(&mut self, _incoming: u8) -> Step { self.plain(BLANK) }
    pub(crate) fn step_block(&mut self, _incoming: u8) -> Step { self.plain(BLANK) }
    pub(crate) fn step_patch_header(&mut self, _incoming: u8) -> Step { self.plain(BLANK) }
    pub(crate) fn step_patch_list(&mut self, _incoming: u8) -> Step { self.plain(BLANK) }
}
```

```rust
//! Phases de table. Corps écrits à la tâche 4.

use super::{BLANK, Session, Step};

impl Session {
    pub(crate) fn step_select(&mut self, _incoming: u8) -> Step { self.plain(BLANK) }
    pub(crate) fn step_verdict(&mut self, _incoming: u8) -> Step { self.plain(BLANK) }
    pub(crate) fn step_trading(&mut self, _incoming: u8) -> Step { self.plain(BLANK) }
}
```

Dans `crates/protocol/src/lib.rs`, déclarer le module : `pub mod session;`

- [ ] **Step 4: Lancer les tests**

Run: `cargo test -p relink-protocol --test session_link`
Expected: les huit tests passent.

- [ ] **Step 5: Commit**

```bash
git add crates/protocol/src/session/ crates/protocol/src/lib.rs crates/protocol/tests/session_link.rs crates/protocol/tests/util/
git commit -m "feat(protocol): session d'échange et phases de lien"
```

---

### Task 3: Phases de transfert

Préambule, graine aléatoire, bloc d'échange, patch list. C'est la partie où les octets comptent vraiment : le bloc sortant part position par position, le bloc entrant se remplit, et la patch list reçue est appliquée à l'arrivée.

**Files:**
- Modify: `crates/protocol/src/session/transfer.rs`
- Modify: `crates/protocol/tests/util/mod.rs`
- Test: `crates/protocol/tests/session_transfer.rs`

**Interfaces:**
- Consumes: `Session`, `Phase`, `Effect`, `Step` de la tâche 2 ; `patch_list::apply` de la tâche 1.
- Produces: dans `tests/util/mod.rs`, `pub fn jusqu_a_la_table(session: &mut Session)` — amène une session fraîche jusqu'au premier octet de préambule inclus.

- [ ] **Step 1: Écrire les tests qui échouent**

Ajouter à `crates/protocol/tests/util/mod.rs` :

```rust
/// Amène une session fraîche au bord du transfert : lien établi, Trade
/// Center choisi, table utilisée. Le premier octet de préambule est consommé.
pub fn jusqu_a_la_table(session: &mut Session) {
    feed(session, &[0x01, 0x00, 0x60, 0xD4, 0x60, 0xFD]);
}
```

Dans `crates/protocol/tests/session_transfer.rs` :

```rust
//! Tests des phases de transfert : préambule, graine, bloc, patch list.
//!
//! Longueurs et valeurs recopiées de `docs/protocol/gen1-link-protocol.md`,
//! sections « Préambule et graine aléatoire » à « Patch list : longueur
//! transmise ».

mod util;

use relink_protocol::gen1::patch_list::{NO_DATA, PART_TERMINATOR};
use relink_protocol::gen1::{TRADE_BLOCK_LEN, TradeBlock};
use relink_protocol::session::{Effect, Session};
use util::{bloc_fixture, effects, feed, jusqu_a_la_table};

const PREAMBLE: u8 = 0xFD;
const BLANK: u8 = 0x00;
const OFF_PARTY_DATA: usize = 19;
const PARTY_DATA_LEN: usize = 264;

/// Le préambule complet : 10 octets, puis 10 d'aléa, puis 9 de préambule.
fn en_tete() -> Vec<u8> {
    let mut v = vec![PREAMBLE; 10];
    v.extend_from_slice(&[0x2A; 10]);
    v.extend_from_slice(&[PREAMBLE; 9]);
    v
}

/// Le module ne présente son bloc qu'après l'en-tête complet, pas avant.
#[test]
fn le_bloc_ne_part_qu_apres_l_en_tete() {
    let mut s = Session::gen1(bloc_fixture(0x10));
    jusqu_a_la_table(&mut s);

    // L'octet de préambule initial est déjà consommé : il en reste neuf,
    // puis l'aléa, puis les neuf derniers.
    let mut reste = vec![PREAMBLE; 9];
    reste.extend_from_slice(&[0x2A; 10]);
    reste.extend_from_slice(&[PREAMBLE; 9]);
    let sortis = feed(&mut s, &reste);
    assert!(
        sortis.iter().all(|&b| b == PREAMBLE || b == 0x2A),
        "pendant l'en-tête, le module renvoie ce qu'il reçoit"
    );

    let premier = feed(&mut s, &[BLANK])[0];
    assert_eq!(premier, bloc_fixture(0x10).as_bytes()[0], "le bloc commence ici");
}

/// Le bloc entrant est reçu en entier et rendu par `partner_block`.
#[test]
fn le_bloc_du_partenaire_est_recu_en_entier() {
    let mut s = Session::gen1(bloc_fixture(0x10));
    jusqu_a_la_table(&mut s);

    let mut octets = en_tete();
    octets.remove(0); // le premier préambule est déjà consommé
    let partenaire = bloc_fixture(0x80);
    octets.extend_from_slice(partenaire.as_bytes());
    feed(&mut s, &octets);

    assert!(s.partner_block().is_none(), "rien tant que la patch list n'est pas passée");

    // Fin de bloc et en-tête de patch list : six octets de préambule.
    feed(&mut s, &[0xDF, 0xFE, 0x15]);
    feed(&mut s, &[PREAMBLE; 6]);
    // Sept octets neutres d'en-tête, puis la liste vide et son remplissage.
    let mut liste = vec![BLANK; 8];
    liste.push(PART_TERMINATOR);
    liste.push(PART_TERMINATOR);
    liste.extend(std::iter::repeat(BLANK).take(200));
    let sortis = effects(&mut s, &liste);

    assert!(sortis.contains(&Effect::PartnerBlockReceived));
    assert_eq!(s.partner_block(), Some(partenaire));
}

/// Le module présente son propre bloc, corrigé : aucun octet « pas de
/// câble » ne part sur le fil.
#[test]
fn aucun_octet_pas_de_cable_ne_part_sur_le_fil() {
    let mut raw = [0u8; TRADE_BLOCK_LEN];
    raw[11] = 1;
    raw[OFF_PARTY_DATA] = NO_DATA;
    raw[OFF_PARTY_DATA + PARTY_DATA_LEN - 1] = NO_DATA;
    let mut s = Session::gen1(TradeBlock::from_bytes(raw));
    jusqu_a_la_table(&mut s);

    let mut octets = en_tete();
    octets.remove(0);
    octets.extend_from_slice(&[BLANK; TRADE_BLOCK_LEN]);
    let sortis = feed(&mut s, &octets);

    assert!(!sortis.contains(&NO_DATA), "le fil ne porte jamais 0xFE");
}

/// La patch list reçue est appliquée : l'octet « pas de câble » est remis en
/// place dans l'équipe du partenaire.
#[test]
fn la_patch_list_recue_est_appliquee() {
    let mut s = Session::gen1(bloc_fixture(0x10));
    jusqu_a_la_table(&mut s);

    let mut attendu = [0u8; TRADE_BLOCK_LEN];
    attendu[11] = 1;
    attendu[OFF_PARTY_DATA + 5] = NO_DATA;

    // Sur le fil, cette position porte 0xFF et sa correction est annoncée.
    let mut sur_le_fil = attendu;
    sur_le_fil[OFF_PARTY_DATA + 5] = PART_TERMINATOR;

    let mut octets = en_tete();
    octets.remove(0);
    octets.extend_from_slice(&sur_le_fil);
    octets.extend_from_slice(&[0xDF, 0xFE, 0x15]);
    octets.extend_from_slice(&[PREAMBLE; 6]);
    octets.extend_from_slice(&[BLANK; 8]);
    octets.push(0x06); // position 5, notée incrémentée de un
    octets.push(PART_TERMINATOR);
    octets.push(PART_TERMINATOR);
    octets.extend(std::iter::repeat(BLANK).take(200));
    feed(&mut s, &octets);

    let recu = s.partner_block().expect("le bloc doit être reçu");
    assert_eq!(recu.as_bytes()[OFF_PARTY_DATA + 5], NO_DATA, "0xFE remis en place");
}

/// Limitation documentée : une cartouche qui redémarre sa négociation en
/// plein transfert n'est pas suivie. Les phases de données transportent des
/// octets arbitraires — 0x01 y est une donnée, pas un signal — et `protocol`
/// n'a pas d'horloge pour trancher. C'est au firmware de détruire la session
/// et d'en ouvrir une neuve.
#[test]
fn une_renegociation_en_plein_transfert_n_est_pas_suivie() {
    let mut s = Session::gen1(bloc_fixture(0x10));
    jusqu_a_la_table(&mut s);
    let mut octets = en_tete();
    octets.remove(0);
    octets.extend_from_slice(&[BLANK; TRADE_BLOCK_LEN]);
    feed(&mut s, &octets);

    // En attente des six octets de préambule de la patch list : l'octet de
    // leader y est renvoyé comme n'importe quel autre.
    assert_eq!(feed(&mut s, &[0x01]), vec![0x01]);
}

/// Un second échange réutilise le même chemin : après la table, un nouveau
/// préambule relance le transfert.
#[test]
fn un_nouveau_preambule_relance_le_transfert() {
    let mut s = Session::gen1(bloc_fixture(0x10));
    jusqu_a_la_table(&mut s);
    let mut octets = en_tete();
    octets.remove(0);
    octets.extend_from_slice(bloc_fixture(0x80).as_bytes());
    octets.extend_from_slice(&[0xDF, 0xFE, 0x15]);
    octets.extend_from_slice(&[PREAMBLE; 6]);
    octets.extend_from_slice(&[BLANK; 8]);
    octets.push(PART_TERMINATOR);
    octets.push(PART_TERMINATOR);
    octets.extend(std::iter::repeat(BLANK).take(200));
    feed(&mut s, &octets);

    // On est en phase de sélection : le module présente l'octet neutre.
    assert_eq!(feed(&mut s, &[BLANK]), vec![BLANK]);
}
```

- [ ] **Step 2: Lancer les tests pour les voir échouer**

Run: `cargo test -p relink-protocol --test session_transfer`
Expected: échec — les phases de transfert présentent l'octet neutre et n'avancent pas.

- [ ] **Step 3: Écrire les phases de transfert**

Remplacer `crates/protocol/src/session/transfer.rs` :

```rust
//! Préambule, graine aléatoire, bloc d'échange, patch list.
//!
//! Deux principes tirés du sourçage :
//!
//! - **On ne compte que là où les sources s'accordent.** Le nombre d'octets
//!   neutres n'est pas fixé ; le nombre d'octets de préambule l'est.
//! - **On ne sort pas de la patch list trop tôt.** Après ses deux
//!   terminateurs viennent des octets de remplissage à zéro, identiques à
//!   ceux de la phase de sélection : rien ne les distingue. Sortir en avance
//!   ferait présenter une offre pendant que la cartouche lit encore sa liste,
//!   et cette offre serait prise pour une position à corriger — le bloc
//!   présenté arriverait faux. On suit donc le compte de la seule
//!   implémentation vérifiée sur matériel, et sortir en retard est sans
//!   conséquence : on y présente l'octet neutre, celui-là même que la
//!   cartouche attend.

use super::{BLANK, OFF_PARTY_DATA, PREAMBLE, Phase, Session, Step};
use crate::gen1::TRADE_BLOCK_LEN;
use crate::gen1::patch_list::{self, PARTY_DATA_LEN, PATCH_LIST_LEN};

/// Octets de préambule qui ouvrent la graine.
const SEED_PREAMBLE: u16 = 10;
/// Octets d'aléa, puis les 9 octets de préambule qui ferment la section.
const SEED_LEN: u16 = 19;
/// Octets de préambule entre le bloc et la patch list.
const PATCH_PREAMBLE: u16 = 6;
/// Octets d'en-tête neutres avant les données de liste. L'octet qui a
/// complété le sixième préambule est déjà consommé par la phase précédente :
/// il en reste sept.
const PATCH_HEADER_LEN: u16 = 7;
/// Longueur de la section, comptée depuis son premier octet. Décalée de deux
/// par rapport au compte de la source (196), qui démarre le sien un octet
/// plus tôt et compte à partir de un.
const PATCH_SECTION_LEN: u16 = 195;

impl Session {
    pub(crate) fn step_preamble(&mut self, incoming: u8) -> Step {
        if incoming == PREAMBLE {
            self.cursor = self.cursor.saturating_add(1);
            if self.cursor >= SEED_PREAMBLE {
                self.phase = Phase::Seed;
                self.cursor = 0;
            }
        }
        self.plain(incoming)
    }

    pub(crate) fn step_seed(&mut self, incoming: u8) -> Step {
        self.cursor = self.cursor.saturating_add(1);
        if self.cursor >= SEED_LEN {
            self.phase = Phase::Block;
            self.cursor = 0;
        }
        self.plain(incoming)
    }

    pub(crate) fn step_block(&mut self, incoming: u8) -> Step {
        let position = self.cursor as usize;
        let outgoing = if position < TRADE_BLOCK_LEN {
            self.incoming[position] = incoming;
            self.outgoing[position]
        } else {
            BLANK
        };

        self.cursor = self.cursor.saturating_add(1);
        if self.cursor as usize >= TRADE_BLOCK_LEN {
            self.phase = Phase::PatchHeader;
            self.cursor = 0;
        }
        self.plain(outgoing)
    }

    pub(crate) fn step_patch_header(&mut self, incoming: u8) -> Step {
        if incoming == PREAMBLE {
            self.cursor = self.cursor.saturating_add(1);
            if self.cursor >= PATCH_PREAMBLE {
                self.phase = Phase::PatchList;
                self.cursor = 0;
                self.incoming_patch = [0u8; PATCH_LIST_LEN];
            }
        }
        self.plain(incoming)
    }

    pub(crate) fn step_patch_list(&mut self, incoming: u8) -> Step {
        let position = self.cursor;
        self.cursor = self.cursor.saturating_add(1);

        let outgoing = if position < PATCH_HEADER_LEN {
            incoming
        } else {
            let index = (position - PATCH_HEADER_LEN) as usize;
            if index < PATCH_LIST_LEN {
                self.incoming_patch[index] = incoming;
                self.outgoing_patch[index]
            } else {
                BLANK
            }
        };

        if self.cursor >= PATCH_SECTION_LEN {
            self.finish_transfer();
            return self.with(outgoing, super::Effect::PartnerBlockReceived);
        }
        self.plain(outgoing)
    }

    /// Applique la patch list reçue à l'équipe entrante et passe la main à la
    /// phase de sélection.
    fn finish_transfer(&mut self) {
        let mut party = [0u8; PARTY_DATA_LEN];
        party.copy_from_slice(&self.incoming[OFF_PARTY_DATA..OFF_PARTY_DATA + PARTY_DATA_LEN]);
        patch_list::apply(&mut party, &self.incoming_patch);
        self.incoming[OFF_PARTY_DATA..OFF_PARTY_DATA + PARTY_DATA_LEN].copy_from_slice(&party);

        self.partner_ready = true;
        self.phase = Phase::Select;
        self.cursor = 0;
        self.reset_round();
    }
}
```

Note pour l'implémenteur : l'octet de leader n'est **pas** traité dans ces phases, et c'est délibéré. Le bloc, la graine et la patch list transportent des octets arbitraires — `0x01` y est une donnée, pas une demande de renégociation. La règle de renégociation ne vaut que dans les phases de synchronisation, et la conséquence est assumée : une cartouche qui redémarre en plein transfert laisse la session bloquée. `protocol` ne connaît pas le temps et ne peut pas s'en sortir seul ; c'est au firmware, qui a une horloge, de détruire la session et d'en ouvrir une neuve. Le test de l'étape 1 fige ce comportement pour qu'il reste une décision et non un accident.

- [ ] **Step 4: Lancer les tests**

Run: `cargo test -p relink-protocol --test session_transfer`
Expected: les six tests passent.

- [ ] **Step 5: Commit**

```bash
git add crates/protocol/src/session/transfer.rs crates/protocol/tests/session_transfer.rs crates/protocol/tests/util/mod.rs
git commit -m "feat(protocol): transfert du bloc et de la patch list"
```

---

### Task 4: Sélection, verdict, échange

Les deux points d'attente du §5.2 de la conception mère, et la seule chose qui distingue les quatre parcours produit : quand la décision arrive.

**Files:**
- Modify: `crates/protocol/src/session/table.rs`
- Modify: `crates/protocol/tests/util/mod.rs`
- Test: `crates/protocol/tests/session_table.rs`

**Interfaces:**
- Consumes: tout ce que les tâches 2 et 3 produisent.
- Produces: dans `tests/util/mod.rs`, `pub fn jusqu_a_la_selection(session: &mut Session, partenaire: TradeBlock)` — amène une session jusqu'à la phase de sélection, bloc du partenaire échangé.

- [ ] **Step 1: Écrire les tests qui échouent**

Ajouter à `crates/protocol/tests/util/mod.rs` :

```rust
/// Amène une session jusqu'à la phase de sélection, le bloc du partenaire
/// ayant été échangé.
pub fn jusqu_a_la_selection(session: &mut Session, partenaire: TradeBlock) {
    jusqu_a_la_table(session);
    let mut octets = vec![0xFD; 9];
    octets.extend_from_slice(&[0x2A; 10]);
    octets.extend_from_slice(&[0xFD; 9]);
    octets.extend_from_slice(partenaire.as_bytes());
    octets.extend_from_slice(&[0xDF, 0xFE, 0x15]);
    octets.extend_from_slice(&[0xFD; 6]);
    octets.extend_from_slice(&[0x00; 8]);
    octets.push(0xFF);
    octets.push(0xFF);
    // La section de patch list fait 195 octets comptés depuis son premier :
    // huit d'en-tête, les deux terminateurs, puis le remplissage. On s'arrête
    // pile à la frontière, pour laisser la phase de sélection intacte.
    octets.extend(core::iter::repeat_n(0x00, 185));
    feed(session, &octets);
}
```

Dans `crates/protocol/tests/session_table.rs` :

```rust
//! Tests des phases de table : sélection, verdict, échange, sortie.
//!
//! Valeurs recopiées de `docs/protocol/gen1-link-protocol.md`, sections
//! « Sélection du Pokémon », « Verdict » et « L'ambiguïté de 0x61 ».

mod util;

use relink_protocol::session::{Decision, Effect, Session};
use util::{bloc_fixture, effects, feed, jusqu_a_la_selection};

const BLANK: u8 = 0x00;
const SELECT_BASE: u8 = 0x60;
const TABLE_LEAVE: u8 = 0x6F;
const REJECT: u8 = 0x61;
const ACCEPT: u8 = 0x62;

fn a_la_selection() -> Session {
    let mut s = Session::gen1(bloc_fixture(0x10));
    jusqu_a_la_selection(&mut s, bloc_fixture(0x80));
    s
}

/// En entrant dans la phase, la session réclame une offre.
#[test]
fn reclame_une_offre_en_entrant() {
    let mut s = a_la_selection();
    assert_eq!(effects(&mut s, &[BLANK]), vec![Effect::OfferNeeded]);
}

/// Tant que l'offre n'est pas fournie, la session présente l'octet neutre —
/// indéfiniment. C'est ce qui rend l'échange direct possible.
#[test]
fn attend_sans_echeance_tant_que_l_offre_manque() {
    let mut s = a_la_selection();
    assert_eq!(feed(&mut s, &[BLANK; 500]), vec![BLANK; 500]);
}

/// L'offre fournie est annoncée par sa position.
#[test]
fn annonce_l_offre_fournie() {
    let mut s = a_la_selection();
    feed(&mut s, &[BLANK]);
    s.supply(Decision::Offer(2));
    assert_eq!(feed(&mut s, &[BLANK]), vec![SELECT_BASE + 2]);
}

/// L'offre du joueur est signalée avec sa position.
#[test]
fn signale_l_offre_du_joueur() {
    let mut s = a_la_selection();
    let sortis = effects(&mut s, &[BLANK, SELECT_BASE + 4]);
    assert!(sortis.contains(&Effect::PartnerOffered { index: 4 }));
}

/// Les deux offres connues, la session réclame un verdict.
#[test]
fn reclame_un_verdict_quand_les_deux_offres_sont_connues() {
    let mut s = a_la_selection();
    feed(&mut s, &[BLANK]);
    s.supply(Decision::Offer(0));
    let sortis = effects(&mut s, &[SELECT_BASE + 1, BLANK]);
    assert!(sortis.contains(&Effect::VerdictNeeded));
}

/// L'accord des deux côtés conclut l'échange, et dit lequel part et lequel
/// arrive.
#[test]
fn l_accord_des_deux_cotes_conclut_l_echange() {
    let mut s = a_la_selection();
    feed(&mut s, &[BLANK]);
    s.supply(Decision::Offer(0));
    feed(&mut s, &[SELECT_BASE + 3, BLANK]);
    s.supply(Decision::Accept);

    let sortis = effects(&mut s, &[ACCEPT]);
    assert_eq!(sortis, vec![Effect::TradeAgreed { offered: 0, received: 3 }]);
}

/// L'accord se présente sur le fil, pas seulement en interne.
#[test]
fn l_accord_est_presente_sur_le_fil() {
    let mut s = a_la_selection();
    feed(&mut s, &[BLANK]);
    s.supply(Decision::Offer(0));
    feed(&mut s, &[SELECT_BASE + 3, BLANK]);
    s.supply(Decision::Accept);
    assert_eq!(feed(&mut s, &[BLANK]), vec![ACCEPT]);
}

/// Un refus du joueur ramène à la sélection : c'est là que 0x61 veut dire
/// « je refuse » et non « je propose le deuxième ».
#[test]
fn le_refus_du_joueur_ramene_a_la_selection() {
    let mut s = a_la_selection();
    feed(&mut s, &[BLANK]);
    s.supply(Decision::Offer(0));
    feed(&mut s, &[SELECT_BASE + 3, BLANK]);

    let sortis = effects(&mut s, &[REJECT, BLANK]);
    assert!(sortis.contains(&Effect::OfferNeeded), "on redemande une offre");
}

/// En phase de sélection, le même octet veut dire « je propose le deuxième ».
#[test]
fn en_selection_le_meme_octet_designe_le_deuxieme_pokemon() {
    let mut s = a_la_selection();
    let sortis = effects(&mut s, &[BLANK, REJECT]);
    assert!(sortis.contains(&Effect::PartnerOffered { index: 1 }));
}

/// Le joueur qui quitte la table ramène la session dans la salle.
#[test]
fn quitter_la_table_ramene_dans_la_salle() {
    let mut s = a_la_selection();
    let sortis = effects(&mut s, &[BLANK, TABLE_LEAVE]);
    assert!(sortis.contains(&Effect::TableLeft));
}

/// Le module aussi peut quitter la table.
#[test]
fn le_module_peut_quitter_la_table() {
    let mut s = a_la_selection();
    feed(&mut s, &[BLANK]);
    s.supply(Decision::Leave);
    assert_eq!(feed(&mut s, &[BLANK]), vec![TABLE_LEAVE]);
}

/// Un index d'offre absurde est borné, jamais transmis tel quel.
#[test]
fn un_index_absurde_est_borne() {
    let mut s = a_la_selection();
    feed(&mut s, &[BLANK]);
    s.supply(Decision::Offer(200));
    assert_eq!(feed(&mut s, &[BLANK]), vec![SELECT_BASE + 5]);
}
```

- [ ] **Step 2: Lancer les tests pour les voir échouer**

Run: `cargo test -p relink-protocol --test session_table`
Expected: échec — les phases de table présentent l'octet neutre et ne signalent rien.

- [ ] **Step 3: Écrire les phases de table**

Remplacer `crates/protocol/src/session/table.rs` :

```rust
//! Sélection du Pokémon, verdict, échange, sortie de table.
//!
//! C'est ici que vivent les deux seuls points d'attente de la session. Tant
//! que la décision manque, on présente l'octet neutre : la cartouche en
//! envoie autant, et l'attend sans échéance. Le jeu y lit un dresseur qui
//! hésite dans ses menus.
//!
//! `0x61` vaut « je propose le Pokémon d'index 1 » en sélection et « je
//! refuse » en verdict. Seule la phase les distingue.

use super::{BLANK, Effect, MASTER, PREAMBLE, Phase, Session, Step};

/// Position maximale dans une équipe.
const LAST_INDEX: u8 = 5;

impl Session {
    pub(crate) fn step_select(&mut self, incoming: u8) -> Step {
        if incoming == MASTER {
            return self.restart();
        }

        if incoming == self.bytes.table_leave {
            self.phase = Phase::Waiting;
            self.reset_round();
            return self.with(self.bytes.table_leave, Effect::TableLeft);
        }

        // Ce que le joueur annonce passe avant la demande d'offre : sinon
        // l'annonce avalerait l'octet, et son offre serait perdue. La demande
        // reste due, et sort au premier octet qui ne dit rien d'autre.
        if let Some(index) = self.partner_index(incoming) {
            self.partner_offer = Some(index);
            return self.with(self.select_outgoing(), Effect::PartnerOffered { index });
        }

        if !self.announced {
            self.announced = true;
            return self.with(self.select_outgoing(), Effect::OfferNeeded);
        }

        if incoming == BLANK && self.offer.is_some() && self.partner_offer.is_some() {
            self.phase = Phase::Verdict;
            return self.with(BLANK, Effect::VerdictNeeded);
        }

        self.plain(self.select_outgoing())
    }

    pub(crate) fn step_verdict(&mut self, incoming: u8) -> Step {
        if incoming == MASTER {
            return self.restart();
        }

        if incoming == self.bytes.trade_reject {
            self.phase = Phase::Select;
            self.reset_round();
            return self.plain(BLANK);
        }

        if incoming == self.bytes.trade_accept {
            self.partner_verdict = Some(true);
            if self.verdict == Some(true) {
                let offered = self.offer.unwrap_or(0);
                let received = self.partner_offer.unwrap_or(0);
                self.phase = Phase::Trading;
                return self.with(
                    self.bytes.trade_accept,
                    Effect::TradeAgreed { offered, received },
                );
            }
        }

        self.plain(self.verdict_outgoing())
    }

    pub(crate) fn step_trading(&mut self, incoming: u8) -> Step {
        if incoming == PREAMBLE {
            self.phase = Phase::Preamble;
            self.cursor = 1;
            self.partner_ready = false;
            self.reset_round();
            return self.plain(incoming);
        }
        self.plain(BLANK)
    }

    /// L'octet à présenter en sélection : l'offre si elle est connue, la
    /// sortie de table si elle est demandée, l'octet neutre sinon.
    fn select_outgoing(&self) -> u8 {
        if self.leaving {
            return self.bytes.table_leave;
        }
        match self.offer {
            Some(index) => self.bytes.select_base.wrapping_add(index),
            None => BLANK,
        }
    }

    /// L'octet à présenter en verdict.
    fn verdict_outgoing(&self) -> u8 {
        match self.verdict {
            Some(true) => self.bytes.trade_accept,
            Some(false) => self.bytes.trade_reject,
            None => BLANK,
        }
    }

    /// La position annoncée par le joueur, si l'octet en désigne une.
    fn partner_index(&self, incoming: u8) -> Option<u8> {
        let base = self.bytes.select_base;
        if incoming < base || incoming > base + LAST_INDEX {
            return None;
        }
        Some(incoming - base)
    }
}
```

Ajouter dans `step_select` le traitement de la sortie demandée par le module : lorsque `self.leaving` est vrai et que l'octet présenté vaut `table_leave`, la session doit aussi repasser en `Phase::Waiting` et émettre `TableLeft`. Implémenter en tête de méthode, juste après le traitement de `MASTER` :

```rust
        if self.leaving {
            self.phase = Phase::Waiting;
            self.reset_round();
            return self.with(self.bytes.table_leave, Effect::TableLeft);
        }
```

- [ ] **Step 4: Lancer les tests**

Run: `cargo test -p relink-protocol --test session_table`
Expected: les douze tests passent.

- [ ] **Step 5: Commit**

```bash
git add crates/protocol/src/session/table.rs crates/protocol/tests/session_table.rs crates/protocol/tests/util/mod.rs
git commit -m "feat(protocol): sélection, verdict et conclusion de l'échange"
```

---

### Task 5: Cartouche simulée et échange complet

Les tâches 2 à 4 vérifient des transitions. Celle-ci vérifie ce qu'aucune d'elles ne voit : un échange entier, puis un second dans la même session — le cas que le protocole impose et qu'un test par phase ne parcourt jamais.

**Files:**
- Modify: `crates/protocol/tests/util/mod.rs`
- Test: `crates/protocol/tests/session_echange.rs`

**Interfaces:**
- Consumes: tout ce que les tâches 1 à 4 produisent.
- Produces: dans `tests/util/mod.rs`, `pub struct Cartouche` avec `pub fn nouvelle(equipe: TradeBlock) -> Self`, `pub fn choisit(&mut self, index: u8)`, `pub fn accepte(&mut self)`, `pub fn octet_suivant(&mut self, recu: u8) -> Option<u8>`.

- [ ] **Step 1: Écrire la cartouche simulée et le test qui échoue**

Ajouter à `crates/protocol/tests/util/mod.rs` :

```rust
/// Une cartouche simulée : elle joue le côté jeu à partir des valeurs
/// sourcées dans `docs/protocol/gen1-link-protocol.md`, et cadence l'échange
/// comme le ferait le matériel.
///
/// Elle vaut ce que vaut le sourçage et ne remplace pas une trace réelle.
/// Elle est là pour attraper les régressions de transition, pas pour prouver
/// l'accord avec une console.
pub struct Cartouche {
    programme: Vec<u8>,
    position: usize,
    equipe: TradeBlock,
}

impl Cartouche {
    /// Une cartouche qui va jusqu'au bord de la sélection, avec cette équipe.
    pub fn nouvelle(equipe: TradeBlock) -> Self {
        let mut programme = vec![0x01, 0x00, 0x00, 0x60, 0xD0, 0xD4, 0x60];
        programme.extend_from_slice(&[0xFD; 10]);
        programme.extend_from_slice(&[0x2A; 10]);
        programme.extend_from_slice(&[0xFD; 9]);
        programme.extend_from_slice(equipe.as_bytes());
        programme.extend_from_slice(&[0xDF, 0xFE, 0x15]);
        programme.extend_from_slice(&[0xFD; 6]);
        programme.extend_from_slice(&[0x00; 8]);
        programme.push(0xFF);
        programme.push(0xFF);
        programme.extend(core::iter::repeat_n(0x00, 185));
        Self { programme, position: 0, equipe }
    }

    /// Le joueur annonce le Pokémon qu'il propose. Une poignée d'octets
    /// neutres suit, comme sur le fil réel.
    pub fn choisit(&mut self, index: u8) {
        self.programme.push(0x60 + index);
        self.programme.extend_from_slice(&[0x00; 4]);
    }

    /// Le joueur accepte l'échange, suivi de la même poignée d'octets
    /// neutres.
    pub fn accepte(&mut self) {
        self.programme.push(0x62);
        self.programme.extend_from_slice(&[0x00; 4]);
    }

    /// Le joueur revient à la table pour un second échange : tout le
    /// transfert recommence.
    pub fn revient_a_la_table(&mut self) {
        self.programme.extend_from_slice(&[0x00; 4]);
        self.programme.extend_from_slice(&[0xFD; 10]);
        self.programme.extend_from_slice(&[0x2A; 10]);
        self.programme.extend_from_slice(&[0xFD; 9]);
        self.programme.extend_from_slice(self.equipe.as_bytes());
        self.programme.extend_from_slice(&[0xDF, 0xFE, 0x15]);
        self.programme.extend_from_slice(&[0xFD; 6]);
        self.programme.extend_from_slice(&[0x00; 8]);
        self.programme.push(0xFF);
        self.programme.push(0xFF);
        self.programme.extend(core::iter::repeat(0x00).take(200));
    }

    /// L'octet suivant que la cartouche présente, ou `None` quand son
    /// programme est épuisé. L'octet reçu est ignoré : la cartouche déroule,
    /// c'est la session qui doit suivre.
    pub fn octet_suivant(&mut self, _recu: u8) -> Option<u8> {
        let octet = self.programme.get(self.position).copied();
        self.position += 1;
        octet
    }
}
```

Dans `crates/protocol/tests/session_echange.rs` :

```rust
//! Un échange complet, puis un second dans la même session.

mod util;

use relink_protocol::session::{Decision, Effect, Session};
use util::{Cartouche, bloc_fixture};

/// Déroule la cartouche jusqu'au bout de son programme et rend les effets
/// émis, en fournissant les décisions du module dès qu'elles sont réclamées.
fn derouler(session: &mut Session, cartouche: &mut Cartouche, offre: u8) -> Vec<Effect> {
    let mut effets = Vec::new();
    while let Some(octet) = cartouche.octet_suivant(0) {
        if let Some(effet) = session.step(octet).effect {
            match effet {
                Effect::OfferNeeded => session.supply(Decision::Offer(offre)),
                Effect::VerdictNeeded => session.supply(Decision::Accept),
                _ => {}
            }
            effets.push(effet);
        }
    }
    effets
}

#[test]
fn un_echange_complet_se_deroule_de_bout_en_bout() {
    let mienne = bloc_fixture(0x10);
    let sienne = bloc_fixture(0x80);
    let mut session = Session::gen1(mienne);
    let mut cartouche = Cartouche::nouvelle(sienne);
    cartouche.choisit(3);
    cartouche.accepte();

    let effets = derouler(&mut session, &mut cartouche, 0);

    assert!(effets.contains(&Effect::LinkEstablished));
    assert!(effets.contains(&Effect::PartnerBlockReceived));
    assert!(effets.contains(&Effect::OfferNeeded));
    assert!(effets.contains(&Effect::PartnerOffered { index: 3 }));
    assert!(effets.contains(&Effect::VerdictNeeded));
    assert!(effets.contains(&Effect::TradeAgreed { offered: 0, received: 3 }));
    assert_eq!(session.partner_block(), Some(sienne));
}

#[test]
fn un_second_echange_suit_le_premier_dans_la_meme_session() {
    let mut session = Session::gen1(bloc_fixture(0x10));
    let mut cartouche = Cartouche::nouvelle(bloc_fixture(0x80));
    cartouche.choisit(3);
    cartouche.accepte();
    cartouche.revient_a_la_table();
    cartouche.choisit(1);
    cartouche.accepte();

    let effets = derouler(&mut session, &mut cartouche, 0);

    let accords: Vec<_> = effets
        .iter()
        .filter(|e| matches!(e, Effect::TradeAgreed { .. }))
        .collect();
    assert_eq!(accords.len(), 2, "deux échanges dans la même session");
    assert_eq!(
        accords[1],
        &&Effect::TradeAgreed { offered: 0, received: 1 }
    );
}

#[test]
fn une_equipe_rearmee_est_celle_qui_part_au_second_echange() {
    let mut session = Session::gen1(bloc_fixture(0x10));
    let mut cartouche = Cartouche::nouvelle(bloc_fixture(0x80));
    cartouche.choisit(0);
    cartouche.accepte();
    cartouche.revient_a_la_table();

    let mut vus = Vec::new();
    let mut rearme = false;
    while let Some(octet) = cartouche.octet_suivant(0) {
        let pas = session.step(octet);
        vus.push(pas.outgoing);
        match pas.effect {
            Some(Effect::OfferNeeded) => session.supply(Decision::Offer(0)),
            Some(Effect::VerdictNeeded) => session.supply(Decision::Accept),
            Some(Effect::TradeAgreed { .. }) => {
                session.supply(Decision::Party(bloc_fixture(0xC0)));
                rearme = true;
            }
            _ => {}
        }
    }

    assert!(rearme, "l'échange doit avoir eu lieu");
    let nouvelle = bloc_fixture(0xC0);
    let attendu = &nouvelle.as_bytes()[..20];
    assert!(
        vus.windows(20).any(|f| f == attendu),
        "le second transfert présente la nouvelle équipe"
    );
}
```

- [ ] **Step 2: Lancer les tests pour les voir échouer**

Run: `cargo test -p relink-protocol --test session_echange`
Expected: échec de compilation, `Cartouche` n'existe pas encore ; puis échec des assertions si une transition manque.

**Si un test échoue après l'écriture de la cartouche, c'est un vrai défaut** : le corriger dans la machine à états, jamais dans le test, et jamais en assouplissant la cartouche. Documenter dans le rapport la séquence fautive et pourquoi la correction est la bonne.

- [ ] **Step 3: Corriger ce que le déroulé complet révèle**

Cette tâche n'ajoute pas de code de production a priori : les tâches 2 à 4 doivent suffire. Si ce n'est pas le cas, la correction porte sur la phase concernée dans `src/session/`.

- [ ] **Step 4: Lancer les tests**

Run: `cargo test -p relink-protocol`
Expected: tout passe.

- [ ] **Step 5: Commit**

```bash
git add crates/protocol/tests/session_echange.rs crates/protocol/tests/util/mod.rs crates/protocol/src/session/
git commit -m "test(protocol): un échange complet, puis un second dans la même session"
```

---

### Task 6: Infaillibilité, `no_std`, et mise à jour de l'état du dépôt

La propriété qui protège les sauvegardes : quels que soient les octets reçus, la session ne panique pas, ne déborde pas, et reste récupérable.

**Files:**
- Modify: `crates/protocol/tests/robustesse.rs`
- Modify: `crates/protocol/src/lib.rs`
- Modify: `README.md`

**Interfaces:**
- Consumes: tout ce qui précède.
- Produces: rien de nouveau.

- [ ] **Step 1: Écrire les propriétés qui échouent**

Ajouter à `crates/protocol/tests/robustesse.rs` :

```rust
use relink_protocol::gen1::patch_list::{self, PARTY_DATA_LEN, PATCH_LIST_LEN};
use relink_protocol::session::{Decision, Session};

proptest! {
    /// Quels que soient les octets reçus, la session ne panique jamais et
    /// présente toujours un octet.
    #[test]
    fn la_session_ne_panique_sur_aucune_suite(octets in prop::collection::vec(any::<u8>(), 0..3000)) {
        let mut bloc = [0u8; TRADE_BLOCK_LEN];
        bloc[11] = 1;
        let mut session = Session::gen1(TradeBlock::from_bytes(bloc));
        for octet in octets {
            let _ = session.step(octet);
        }
    }

    /// Une décision fournie à contretemps ne casse rien.
    #[test]
    fn une_decision_a_contretemps_ne_casse_rien(
        index in any::<u8>(),
        octets in prop::collection::vec(any::<u8>(), 0..500),
    ) {
        let mut bloc = [0u8; TRADE_BLOCK_LEN];
        bloc[11] = 1;
        let mut session = Session::gen1(TradeBlock::from_bytes(bloc));
        session.supply(Decision::Offer(index));
        session.supply(Decision::Accept);
        for octet in octets {
            let _ = session.step(octet);
        }
        session.supply(Decision::Leave);
        let _ = session.partner_block();
    }

    /// L'aller-retour de la patch list est sans perte, quelles que soient
    /// les données d'équipe.
    #[test]
    fn l_aller_retour_de_patch_list_est_sans_perte(raw in prop::array::uniform32(any::<u8>())) {
        let mut party = [0u8; PARTY_DATA_LEN];
        for (i, b) in party.iter_mut().enumerate() {
            *b = raw[i % raw.len()];
        }
        let origine = party;
        let list = patch_list::build(&mut party);
        patch_list::apply(&mut party, &list);
        prop_assert_eq!(party, origine);
    }

    /// Une patch list reçue arbitraire ne fait jamais déborder l'équipe.
    #[test]
    fn une_patch_list_arbitraire_ne_deborde_pas(raw in prop::array::uniform32(any::<u8>())) {
        let mut party = [0u8; PARTY_DATA_LEN];
        let mut list = [0u8; PATCH_LIST_LEN];
        for (i, b) in list.iter_mut().enumerate() {
            *b = raw[i % raw.len()];
        }
        patch_list::apply(&mut party, &list);
    }
}
```

- [ ] **Step 2: Lancer les tests**

Run: `cargo test -p relink-protocol --test robustesse`
Expected: tout passe. **Si `proptest` trouve un contre-exemple**, il l'écrit dans `crates/protocol/proptest-regressions/`. Corriger la machine à états, jamais le test, et versionner le fichier de régression : il documente un vrai bug.

- [ ] **Step 3: Vérifier que le crate tient toujours sans `std` ni allocateur**

```bash
cargo build -p relink-protocol --target thumbv7em-none-eabihf
```

Expected: succès. C'est la seule preuve que la contrainte `no_std` sans `alloc` est tenue.

- [ ] **Step 4: Mettre à jour l'état annoncé**

Dans `crates/protocol/src/lib.rs`, le module `session` rejoint la liste de ce qui est livré, et la phrase « Ce qui manque encore : la machine à états de l'échange, et les codecs de deuxième génération » ne garde que les codecs Gen 2.

Dans `README.md`, le bloc « État » annonce encore que `crates/application` n'a aucun cas d'usage implémenté, ce qui est faux depuis le lot précédent. Le réécrire : les codecs Gen 1, le cœur métier de `application` et la machine à états de l'échange Gen 1 sont livrés ; restent les codecs Gen 2, le firmware et les adaptateurs.

- [ ] **Step 5: Vérification complète**

Run: `cargo test --workspace`, `cargo clippy --workspace --all-targets`, `cargo fmt --all --check`, `cargo doc --workspace --no-deps`
Expected: tout passe, aucun avertissement.

- [ ] **Step 6: Commit**

```bash
git add crates/protocol/tests/robustesse.rs crates/protocol/src/lib.rs README.md crates/protocol/proptest-regressions/ 2>/dev/null || git add crates/protocol/tests/robustesse.rs crates/protocol/src/lib.rs README.md
git commit -m "test(protocol): infaillibilité de la session sur octets arbitraires"
```

---

## Ce que ce plan ne fait pas

- **Le Colosseum.** Reconnu dans le menu et refusé ; les combats ne sont pas dans le projet.
- **La Gen 2.** Ses valeurs de machine à états sont sourcées et consignées, mais rien ne les exerce tant que le codec du bloc Gen 2 n'existe pas. La table `LinkBytes` est l'endroit prévu pour les accueillir.
- **Le branchement sur `application`.** Les quatre parcours produit se distinguent par le moment où la décision arrive, pas par la machine à états. Ils viendront avec le lot qui en a besoin.
- **Le rôle de leader.** Le module reste suiveur : ni horloge, ni délai d'attente, ni élection.
- **Toute preuve d'accord avec du matériel réel.** La cartouche simulée reproduit le sourçage, y compris ses erreurs éventuelles. Seul le lot firmware apportera des traces réelles, rejouables en test.
