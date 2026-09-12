#![no_std]
#![no_main]

//! Banc de la couche bit : des octets passent-ils vraiment, dans les deux sens ?
//!
//! Le banc de latence (`banc.rs`) a montré que l'ISR arrive à temps. Il n'a
//! jamais fait transiter un seul bit utile. Celui-ci monte la couche
//! au-dessus : un registre à décalage de chaque côté, cadencé comme le fait le
//! matériel.
//!
//! **Les fronts sont ceux du document sourcé** (`docs/protocol/link-couche-physique.md`,
//! « Fronts d'horloge ») : le bit est posé sur la ligne au front **descendant**
//! et lu au front **montant**. Les deux moitiés suivent la même règle, donc
//! chacune dispose d'une demi-période pleine — 61 µs à 8192 Hz — entre le
//! moment où elle doit présenter un bit et le moment où l'autre le lit. C'est
//! cette marge que le banc de latence chiffrait à 2,7 µs consommés.
//!
//! **Bit de poids fort en premier**, comme le registre à décalage du jeu
//! (même document, « Ordre des bits »).
//!
//! ## Les deux moitiés
//!
//! - **Le rôle cartouche** possède l'horloge. Il n'attend personne : il bat sa
//!   demi-période au compteur de cycles, pose son bit, lit celui d'en face.
//!   C'est ce qui rend le test honnête — un vrai jeu ne ralentit pas pour nous.
//! - **Le rôle module** ne possède rien. Il est réveillé par les deux fronts :
//!   au descendant il présente son bit, au montant il lit celui de la
//!   cartouche. Au huitième front montant l'octet est complet, et l'octet
//!   suivant est chargé pour que le front descendant d'après le trouve prêt.
//!
//! ## Ce qui est vérifié
//!
//! Chaque sens transporte une suite déterministe, de germes différents. À la
//! fin, les deux flux sont comparés à ce qu'ils auraient dû être, et le banc
//! annonce le premier écart s'il y en a un. Un octet dupliqué au milieu du
//! flux — le mode de panne exact que décrit `docs/protocol/link-couche-physique.md`
//! (« Octet non prêt : le précédent repart ») — ressort ici comme une
//! divergence, alors qu'il serait invisible sur le fil.
//!
//! ## Câblage
//!
//! Trois straps, tout en 3,3 V :
//!
//! | Ligne | Émetteur | Récepteur |
//! |---|---|---|
//! | SCK | `GPIO25` | `GPIO18` |
//! | cartouche → module | `GPIO26` | `GPIO19` |
//! | module → cartouche | `GPIO21` | `GPIO27` |

use core::cell::RefCell;

use critical_section::Mutex;
use esp_backtrace as _;
use esp_hal::clock::CpuClock;
use esp_hal::gpio::{Event, Input, InputConfig, Io, Level, Output, OutputConfig, Pull};
use esp_hal::xtensa_lx::timer::get_cycle_count;
use esp_hal::{handler, main, ram};
use log::{error, info};

esp_bootloader_esp_idf::esp_app_desc!();

const CPU_HZ: u32 = 240_000_000;
const CADENCE_HZ: u32 = 8_192;
/// Une demi-période : l'intervalle entre deux fronts successifs.
const DEMI_PERIODE: u32 = CPU_HZ / CADENCE_HZ / 2;
/// Un échange Gen 1 complet dépasse le millier d'octets — bloc d'échange de
/// 415 octets dans chaque sens, patch list, préambule. Passer 256 octets ne
/// prouverait pas qu'une dérive lente n'existe pas ; on vise donc au-delà de
/// la longueur réelle, tout en tenant en RAM sans allocateur.
const OCTETS: usize = 1_024;

/// Suite déterministe, calculée à l'identique des deux côtés. Un simple
/// congruentiel suffit : ce qu'on veut, c'est que deux octets voisins ne se
/// ressemblent pas, pour qu'un décalage d'un bit ou un octet répété saute aux
/// yeux au lieu de se fondre dans du remplissage.
fn suite(germe: u32, i: usize) -> u8 {
    let mut x = germe.wrapping_add(i as u32).wrapping_mul(2_654_435_761);
    x ^= x >> 15;
    x = x.wrapping_mul(2_246_822_519);
    (x >> 13) as u8
}

const GERME_CARTOUCHE: u32 = 0x1234_5678;
const GERME_MODULE: u32 = 0x0BAD_C0DE;

/// Tout ce que l'ISR du rôle module doit atteindre. Rangé d'un bloc plutôt
/// qu'en atomiques éparses : l'octet en cours et son compteur de bits doivent
/// avancer ensemble ou pas du tout.
struct Module {
    sck: Input<'static>,
    sout: Input<'static>,
    sin: Output<'static>,
    /// Ce qui arrive de la cartouche, en cours d'assemblage.
    entrant: u8,
    /// Ce qui part vers la cartouche, bit de poids fort en tête.
    sortant: u8,
    bits: u8,
    index: usize,
    recu: [u8; OCTETS],
    /// Faux tant que le module n'a pas fini ses `OCTETS` octets.
    fini: bool,
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
            // Front montant : on lit ce que la cartouche a posé au front
            // précédent.
            m.entrant = (m.entrant << 1) | u8::from(m.sout.is_high());
            m.bits += 1;
            if m.bits == 8 {
                m.bits = 0;
                if m.index < OCTETS {
                    m.recu[m.index] = m.entrant;
                    m.index += 1;
                }
                // L'octet suivant est chargé MAINTENANT, pas au front
                // descendant qui suit : c'est ce qui lui laisse une
                // demi-période d'avance au lieu de zéro.
                m.sortant = if m.index < OCTETS {
                    suite(GERME_MODULE, m.index)
                } else {
                    m.fini = true;
                    0
                };
            }
        } else {
            // Front descendant : on présente notre bit.
            let bit = m.sortant & 0x80 != 0;
            m.sin.set_level(if bit { Level::High } else { Level::Low });
            m.sortant <<= 1;
        }
    });
}

/// Attend l'échéance donnée en cycles absolus. La comparaison passe par une
/// soustraction pour survivre au débordement du compteur.
#[inline(always)]
fn attendre(echeance: u32) {
    while get_cycle_count().wrapping_sub(echeance) > u32::MAX / 2 {}
}

/// Le rôle cartouche : il bat l'horloge et n'attend personne.
fn cadencer(
    sck: &mut Output<'static>,
    sout: &mut Output<'static>,
    sin: &Input<'static>,
) -> [u8; OCTETS] {
    let mut recu = [0u8; OCTETS];
    let mut horloge = get_cycle_count();

    for (i, slot) in recu.iter_mut().enumerate() {
        let mut sortant = suite(GERME_CARTOUCHE, i);
        let mut entrant = 0u8;

        for _ in 0..8 {
            // Front descendant : on pose notre bit.
            horloge = horloge.wrapping_add(DEMI_PERIODE);
            attendre(horloge);
            sck.set_low();
            sout.set_level(if sortant & 0x80 != 0 {
                Level::High
            } else {
                Level::Low
            });
            sortant <<= 1;

            // Front montant : on lit le sien.
            horloge = horloge.wrapping_add(DEMI_PERIODE);
            attendre(horloge);
            sck.set_high();
            entrant = (entrant << 1) | u8::from(sin.is_high());
        }

        *slot = entrant;
    }

    recu
}

/// Compare un flux à ce qu'il aurait dû être et rend l'indice du premier
/// écart. Le premier suffit : au-delà, le flux est désynchronisé et les
/// suivants n'apprennent rien.
fn premier_ecart(recu: &[u8], germe: u32) -> Option<(usize, u8, u8)> {
    recu.iter()
        .enumerate()
        .map(|(i, &o)| (i, o, suite(germe, i)))
        .find(|&(_, recu, attendu)| recu != attendu)
}

fn rapporter(sens: &str, recu: &[u8], germe: u32, ligne: &str) -> bool {
    // Une ligne de données non câblée ne produit pas du bruit : l'entrée est
    // tirée au bas et ne lit que des zéros. Le dire ainsi plutôt que d'annoncer
    // une divergence d'octet, qui enverrait chercher un bug de protocole là où
    // il manque un fil.
    if recu.iter().all(|&o| o == 0) {
        error!("{sens} — que des zéros : la ligne {ligne} est-elle câblée ?");
        return false;
    }
    match premier_ecart(recu, germe) {
        None => {
            info!("{sens} — {} octets, identiques", recu.len());
            true
        }
        Some((i, r, a)) => {
            error!(
                "{sens} — divergence au premier octet à l'indice {i} : reçu {r:#04x}, attendu {a:#04x}"
            );
            false
        }
    }
}

#[main]
fn main() -> ! {
    esp_println::logger::init_logger_from_env();

    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    // Rôle cartouche.
    let mut sck_out = Output::new(peripherals.GPIO25, Level::High, OutputConfig::default());
    let mut sout_out = Output::new(peripherals.GPIO26, Level::Low, OutputConfig::default());
    let sin_in = Input::new(
        peripherals.GPIO27,
        InputConfig::default().with_pull(Pull::Down),
    );

    // Rôle module.
    let mut sck_in = Input::new(
        peripherals.GPIO18,
        InputConfig::default().with_pull(Pull::Down),
    );
    let sout_in = Input::new(
        peripherals.GPIO19,
        InputConfig::default().with_pull(Pull::Down),
    );
    let sin_out = Output::new(peripherals.GPIO21, Level::Low, OutputConfig::default());

    // Les deux fronts réveillent le module : le descendant pour présenter,
    // le montant pour lire.
    sck_in.listen(Event::AnyEdge);

    let mut io = Io::new(peripherals.IO_MUX);
    io.set_interrupt_handler(sur_front);

    info!("relink — banc d'échange — couche bit à {CADENCE_HZ} Hz, {OCTETS} octets par sens");
    info!(
        "câblage attendu : D25→D18 (SCK), D26→D19 (cartouche→module), D21→D27 (module→cartouche)"
    );

    critical_section::with(|cs| {
        MODULE.borrow_ref_mut(cs).replace(Module {
            sck: sck_in,
            sout: sout_in,
            sin: sin_out,
            entrant: 0,
            // Le premier octet est chargé avant que l'horloge ne démarre :
            // sinon le tout premier front descendant trouverait le registre
            // vide et le flux partirait décalé d'un octet.
            sortant: suite(GERME_MODULE, 0),
            bits: 0,
            index: 0,
            recu: [0; OCTETS],
            fini: false,
        });
    });

    loop {
        // Remise à zéro entre deux passes : le module garde son état d'une
        // passe à l'autre, et repartir avec un compteur de bits non nul
        // décalerait tout le flux.
        critical_section::with(|cs| {
            let mut emprunt = MODULE.borrow_ref_mut(cs);
            let m = emprunt.as_mut().unwrap();
            m.entrant = 0;
            m.sortant = suite(GERME_MODULE, 0);
            m.bits = 0;
            m.index = 0;
            m.recu = [0; OCTETS];
            m.fini = false;
        });

        let cote_cartouche = cadencer(&mut sck_out, &mut sout_out, &sin_in);

        let (cote_module, index, fini) = critical_section::with(|cs| {
            let emprunt = MODULE.borrow_ref(cs);
            let m = emprunt.as_ref().unwrap();
            (m.recu, m.index, m.fini)
        });

        if index == 0 {
            error!("le module n'a reçu AUCUN octet — les trois straps sont-ils en place ?");
        } else {
            if !fini {
                error!(
                    "le module n'a assemblé que {index} octets sur {OCTETS} : des fronts ont été perdus"
                );
            }
            let a = rapporter(
                "cartouche → module",
                &cote_module[..index],
                GERME_CARTOUCHE,
                "D26 → D19",
            );
            let b = rapporter(
                "module → cartouche",
                &cote_cartouche,
                GERME_MODULE,
                "D21 → D27",
            );
            if a && b && fini {
                info!("les deux sens sont intacts");
            }
        }

        info!("--- passe terminée ---");
    }
}
