#![no_std]
#![no_main]

//! Un échange Gen 1 complet, sur le fil, contre le même joué en mémoire.
//!
//! C'est la première fois que le cœur métier du dépôt tourne sur du matériel.
//! `relink_protocol::session::Session` décode ici de vrais octets arrivés par
//! un vrai fil, cadencés par une horloge qui ne l'attend pas — et non plus des
//! octets poussés par une boucle de test complaisante.
//!
//! ## Le témoin est sur la même puce
//!
//! Le banc joue le MÊME échange deux fois : une fois en mémoire, une fois par
//! le fil, puis compare les deux flux sortants. Aucune valeur attendue n'est
//! figée dans le code : c'est la session elle-même qui fournit la référence.
//! Un écart ne peut donc venir que de ce qui sépare les deux exécutions — le
//! temps réel.
//!
//! La cartouche est celle de `relink_protocol::testing`, la même que les tests
//! d'intégration du crate. Une copie locale aurait divergé le jour où une
//! valeur sourcée change.
//!
//! ## Le décalage d'un créneau, qui n'est pas un bug
//!
//! `Session::step(entrant) -> sortant` modélise un échange simultané : les
//! deux registres à décalage se croisent. Le matériel ne peut pas faire ça.
//! Pendant le créneau `i`, le module décale un octet qu'il devait avoir chargé
//! **avant** que le créneau ne commence ; sa réponse à l'octet `i` ne peut
//! donc sortir qu'au créneau `i+1`.
//!
//! Le banc en tient compte explicitement plutôt que de le masquer : il compare
//! `observé[i+1]` à `référence[i]`. Le créneau 0 transporte l'octet préchargé,
//! qui ne répond à rien.
//!
//! C'est la contrainte du §5.1 de la conception vue depuis le fil : l'octet
//! sortant doit être prêt avant le front, et la marge pour le préparer est
//! d'un créneau entier — 122 µs à 8192 Hz.
//!
//! ## Câblage
//!
//! Les trois mêmes straps que `echange.rs` : `D25→D18` (SCK), `D26→D19`
//! (cartouche → module), `D21→D27` (module → cartouche).

use core::cell::RefCell;

use critical_section::Mutex;
use esp_backtrace as _;
use esp_hal::clock::CpuClock;
use esp_hal::gpio::{Event, Input, InputConfig, Io, Level, Output, OutputConfig, Pull};
use esp_hal::xtensa_lx::timer::get_cycle_count;
use esp_hal::{handler, main, ram};
use log::{error, info};
use relink_protocol::session::{Decision, Effect, Session};
use relink_protocol::testing::{Cartouche, bloc_fixture};

esp_bootloader_esp_idf::esp_app_desc!();

const CPU_HZ: u32 = 240_000_000;
const CADENCE_HZ: u32 = 8_192;
const DEMI_PERIODE: u32 = CPU_HZ / CADENCE_HZ / 2;

/// Un échange complet tient en 665 octets ; la marge couvre une sélection de
/// plus sans reprendre la capacité.
const CAPACITE: usize = 1_024;

/// Le Pokémon que la cartouche propose, et celui que le module offre en
/// retour. Mêmes valeurs que `session_echange.rs`, pour que l'échange joué ici
/// soit celui que les tests du crate connaissent.
const INDEX_PROPOSE_PAR_LA_CARTOUCHE: u8 = 3;
const INDEX_OFFERT_PAR_LE_MODULE: u8 = 0;

/// Ce que la session a émis en chemin, résumé pour l'affichage. Garder les
/// effets sous forme de drapeaux plutôt que d'un journal évite d'allouer et
/// suffit : c'est leur présence qui prouve que l'échange est allé au bout.
#[derive(Default, PartialEq, Eq)]
struct Jalons {
    lien: bool,
    bloc_partenaire: bool,
    offre_demandee: bool,
    partenaire_propose: Option<u8>,
    verdict_demande: bool,
    accord: Option<(u8, u8)>,
}

impl Jalons {
    fn noter(&mut self, effet: Effect) {
        match effet {
            Effect::LinkEstablished => self.lien = true,
            Effect::PartnerBlockReceived => self.bloc_partenaire = true,
            Effect::OfferNeeded => self.offre_demandee = true,
            Effect::PartnerOffered { index } => self.partenaire_propose = Some(index),
            Effect::VerdictNeeded => self.verdict_demande = true,
            Effect::TradeAgreed { offered, received } => self.accord = Some((offered, received)),
            _ => {}
        }
    }

    fn complet(&self) -> bool {
        self.lien
            && self.bloc_partenaire
            && self.offre_demandee
            && self.partenaire_propose == Some(INDEX_PROPOSE_PAR_LA_CARTOUCHE)
            && self.verdict_demande
            && self.accord
                == Some((
                    INDEX_OFFERT_PAR_LE_MODULE,
                    INDEX_PROPOSE_PAR_LA_CARTOUCHE,
                ))
    }
}

/// Fait avancer la session d'un octet et fournit sur-le-champ les décisions
/// qu'elle réclame.
///
/// Répondre dans le pas d'exécution est le parcours « évolution par échange »,
/// le seul qui n'a besoin ni de compte ni de réseau. Les trois autres
/// parcours passeraient par `supply()` plus tard, ce que la machine à états
/// sait faire sans arrêter le fil — mais ce banc ne teste pas ça.
fn avancer(session: &mut Session, entrant: u8, jalons: &mut Jalons) -> u8 {
    let pas = session.step(entrant);
    if let Some(effet) = pas.effect {
        jalons.noter(effet);
        match effet {
            Effect::OfferNeeded => session.supply(Decision::Offer(INDEX_OFFERT_PAR_LE_MODULE)),
            Effect::VerdictNeeded => session.supply(Decision::Accept),
            _ => {}
        }
    }
    pas.outgoing
}

/// L'état que l'ISR du rôle module doit atteindre, session comprise.
///
/// Appeler `Session::step()` depuis l'ISR n'est pas un raccourci de banc :
/// c'est ce que le firmware devra faire. La machine à états est écrite pour —
/// O(1), sans allocation, infaillible.
struct Module {
    sck: Input<'static>,
    sout: Input<'static>,
    sin: Output<'static>,
    entrant: u8,
    sortant: u8,
    bits: u8,
    session: Session,
    jalons: Jalons,
    octets_vus: usize,
}

static MODULE: Mutex<RefCell<Option<Module>>> = Mutex::new(RefCell::new(None));

#[handler]
#[ram]
fn sur_front() {
    critical_section::with(|cs| {
        let mut emprunt = MODULE.borrow_ref_mut(cs);
        let Some(m) = emprunt.as_mut() else { return };
        m.sck.clear_interrupt();

        if m.sck.is_high() {
            m.entrant = (m.entrant << 1) | u8::from(m.sout.is_high());
            m.bits += 1;
            if m.bits == 8 {
                m.bits = 0;
                let recu = m.entrant;
                m.octets_vus += 1;
                let mut jalons = core::mem::take(&mut m.jalons);
                m.sortant = avancer(&mut m.session, recu, &mut jalons);
                m.jalons = jalons;
            }
        } else {
            let bit = m.sortant & 0x80 != 0;
            m.sin
                .set_level(if bit { Level::High } else { Level::Low });
            m.sortant <<= 1;
        }
    });
}

#[inline(always)]
fn attendre(echeance: u32) {
    while get_cycle_count().wrapping_sub(echeance) > u32::MAX / 2 {}
}

/// Le rôle cartouche : il pousse son programme sur le fil et relève ce qui
/// revient, sans jamais ralentir.
fn cadencer(
    sck: &mut Output<'static>,
    sout: &mut Output<'static>,
    sin: &Input<'static>,
    programme: &[u8],
    observe: &mut [u8],
) {
    let mut horloge = get_cycle_count();

    for (octet, place) in programme.iter().zip(observe.iter_mut()) {
        let mut sortant = *octet;
        let mut entrant = 0u8;

        for _ in 0..8 {
            horloge = horloge.wrapping_add(DEMI_PERIODE);
            attendre(horloge);
            sck.set_low();
            sout.set_level(if sortant & 0x80 != 0 {
                Level::High
            } else {
                Level::Low
            });
            sortant <<= 1;

            horloge = horloge.wrapping_add(DEMI_PERIODE);
            attendre(horloge);
            sck.set_high();
            entrant = (entrant << 1) | u8::from(sin.is_high());
        }

        *place = entrant;
    }
}

/// Construit le programme de la cartouche : l'échange de `session_echange.rs`,
/// à l'octet près.
fn programme_de_la_cartouche(buffer: &mut [u8; CAPACITE]) -> usize {
    let mut cartouche = Cartouche::<CAPACITE>::nouvelle(bloc_fixture(0x80));
    cartouche.choisit(INDEX_PROPOSE_PAR_LA_CARTOUCHE);
    cartouche.accepte();
    let programme = cartouche.programme();
    buffer[..programme.len()].copy_from_slice(programme);
    programme.len()
}

/// Le même échange, joué en mémoire. C'est la référence, et elle est produite
/// par la session elle-même : rien n'est figé en dur.
fn reference(programme: &[u8], sortie: &mut [u8]) -> (Jalons, bool) {
    let mut session = Session::gen1(bloc_fixture(0x10));
    let mut jalons = Jalons::default();
    for (entrant, place) in programme.iter().zip(sortie.iter_mut()) {
        *place = avancer(&mut session, *entrant, &mut jalons);
    }
    let bloc = session.partner_block() == Some(bloc_fixture(0x80));
    (jalons, bloc)
}

#[main]
fn main() -> ! {
    esp_println::logger::init_logger_from_env();

    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    let mut sck_out = Output::new(peripherals.GPIO25, Level::High, OutputConfig::default());
    let mut sout_out = Output::new(peripherals.GPIO26, Level::Low, OutputConfig::default());
    let sin_in = Input::new(
        peripherals.GPIO27,
        InputConfig::default().with_pull(Pull::Down),
    );

    let mut sck_in = Input::new(
        peripherals.GPIO18,
        InputConfig::default().with_pull(Pull::Down),
    );
    let sout_in = Input::new(
        peripherals.GPIO19,
        InputConfig::default().with_pull(Pull::Down),
    );
    let sin_out = Output::new(peripherals.GPIO21, Level::Low, OutputConfig::default());

    sck_in.listen(Event::AnyEdge);
    let mut io = Io::new(peripherals.IO_MUX);
    io.set_interrupt_handler(sur_front);

    let mut programme = [0u8; CAPACITE];
    let longueur = programme_de_la_cartouche(&mut programme);

    let mut attendu = [0u8; CAPACITE];
    let (jalons_memoire, bloc_memoire) = reference(&programme[..longueur], &mut attendu[..longueur]);

    info!("relink — échange Gen 1 sur le fil, {longueur} octets à {CADENCE_HZ} Hz");
    if !jalons_memoire.complet() || !bloc_memoire {
        error!("l'échange de référence, en mémoire, ne va PAS au bout — le banc ne vaut rien");
    }

    let mut observe = [0u8; CAPACITE];

    critical_section::with(|cs| {
        MODULE.borrow_ref_mut(cs).replace(Module {
            sck: sck_in,
            sout: sout_in,
            sin: sin_out,
            entrant: 0,
            sortant: 0,
            bits: 0,
            session: Session::gen1(bloc_fixture(0x10)),
            jalons: Jalons::default(),
            octets_vus: 0,
        });
    });

    loop {
        // Une session fraîche à chaque passe : la machine à états accepte un
        // second échange dans la même session, mais ce banc n'en joue qu'un.
        critical_section::with(|cs| {
            let mut emprunt = MODULE.borrow_ref_mut(cs);
            let m = emprunt.as_mut().unwrap();
            m.entrant = 0;
            m.sortant = 0;
            m.bits = 0;
            m.session = Session::gen1(bloc_fixture(0x10));
            m.jalons = Jalons::default();
            m.octets_vus = 0;
        });

        cadencer(
            &mut sck_out,
            &mut sout_out,
            &sin_in,
            &programme[..longueur],
            &mut observe[..longueur],
        );

        let (jalons_fil, bloc_fil, vus) = critical_section::with(|cs| {
            let emprunt = MODULE.borrow_ref(cs);
            let m = emprunt.as_ref().unwrap();
            (
                m.jalons.complet(),
                m.session.partner_block() == Some(bloc_fixture(0x80)),
                m.octets_vus,
            )
        });

        if vus == 0 {
            error!("le module n'a vu aucun octet — les trois straps sont-ils en place ?");
        } else if vus != longueur {
            error!("le module n'a vu que {vus} octets sur {longueur} : des fronts sont perdus");
        } else {
            // Décalage d'un créneau : la réponse à l'octet i sort au créneau
            // i+1. Le créneau 0 transporte l'octet préchargé, qui ne répond à
            // rien et n'est donc pas comparé.
            let ecart = (1..longueur).find(|&i| observe[i] != attendu[i - 1]);
            match ecart {
                None => info!("flux sortant identique sur {} octets", longueur - 1),
                Some(i) => error!(
                    "divergence au créneau {i} : le fil a porté {:#04x}, la mémoire attendait {:#04x}",
                    observe[i],
                    attendu[i - 1]
                ),
            }

            if jalons_fil && bloc_fil && ecart.is_none() {
                info!("échange Gen 1 complet : lien, bloc, offre, verdict, accord — et le bloc du partenaire est intact");
            } else {
                if !jalons_fil {
                    error!("l'échange par le fil n'a PAS atteint tous ses jalons");
                }
                if !bloc_fil {
                    error!("le bloc du partenaire reçu par le fil est faux");
                }
            }
        }

        info!("--- passe terminée ---");
    }
}
