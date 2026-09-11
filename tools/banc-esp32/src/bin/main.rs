#![no_std]
#![no_main]

//! Banc de mesure : le lien Game Boy se décode-t-il par interruption ?
//!
//! Le port link n'a aucune ligne de sélection (cf.
//! `docs/protocol/link-couche-physique.md`, « Aucune ligne de sélection »), ce
//! qui écarte le SPI esclave matériel de l'ESP32 : il cadre ses transferts sur
//! CS. Le repli est un décodage sur interruption de SCK. Ce banc mesure si ce
//! repli tient, et à quelles conditions.
//!
//! **Ce qu'il mesure.** Le temps entre la décision d'émettre un front et
//! l'horodatage pris à l'entrée de l'ISR. Une moitié de la carte joue la
//! cartouche et bat l'horloge sur GPIO25 ; l'autre la reçoit sur GPIO18. Les
//! deux vivent sur la même puce, donc sur le même compteur de cycles : la
//! soustraction est directement exploitable, sans étalonnage.
//!
//! **Trois phases dans le même binaire**, pour qu'aucun écart ne puisse être
//! mis sur le dos d'une compilation différente. Chaque phase ajoute une seule
//! chose, donc un écart désigne son coupable :
//!
//!   A — rien.
//!   B — ordonnanceur esp-rtos démarré.
//!   C — radio active en point d'accès ouvert.
//!
//! La phase C fait tourner la mesure sur le MÊME cœur que la radio, donc dans
//! le pire cas. Qu'elle passe est ce qui a permis de conclure que l'épinglage
//! « lien sur un cœur, Wi-Fi sur l'autre » est une optimisation et non une
//! nécessité.
//!
//! **Deux séries d'affilée par cadence, et l'indice du pire échantillon.** Ce
//! n'est pas de la coquetterie : c'est ce qui a montré que les pics de 10 à
//! 37 µs tombent toujours à l'indice 0, juste après une ligne de journal — une
//! reprise à froid du cache de flash, pas une gigue d'interruption. Retirer
//! l'un ou l'autre rendrait le banc incapable de refaire la distinction.
//!
//! **Câblage** : un strap entre GPIO25 et GPIO18. Rien d'autre, tout est en
//! 3,3 V. Voir `docs/diagrams/banc-esp32.html`.
//!
//! Résultats et interprétation : `docs/firmware/latence-decodage.md`.

use core::cell::RefCell;
use core::sync::atomic::{AtomicU32, Ordering};

use critical_section::Mutex;
use esp_backtrace as _;
use esp_hal::clock::CpuClock;
use esp_hal::gpio::{Event, Input, InputConfig, Io, Level, Output, OutputConfig, Pull};
use esp_hal::timer::timg::TimerGroup;
use esp_hal::xtensa_lx::timer::get_cycle_count;
use esp_hal::{handler, main, ram};
use esp_radio::wifi::Config as WifiConfig;
use esp_radio::wifi::ap::AccessPointConfig;
use log::{error, info};

extern crate alloc;

esp_bootloader_esp_idf::esp_app_desc!();

const CPU_HZ: u32 = 240_000_000;
const RATES_HZ: [u32; 3] = [4_096, 8_192, 16_384];
const SAMPLES: usize = 2_000;
const GIVE_UP_CYCLES: u32 = CPU_HZ / 100;

static ISR_CCOUNT: AtomicU32 = AtomicU32::new(0);
static ISR_HITS: AtomicU32 = AtomicU32::new(0);
static SCK_IN: Mutex<RefCell<Option<Input<'static>>>> = Mutex::new(RefCell::new(None));

#[handler]
#[ram]
fn isr() {
    ISR_CCOUNT.store(get_cycle_count(), Ordering::Relaxed);
    ISR_HITS.fetch_add(1, Ordering::Relaxed);
    critical_section::with(|cs| {
        if let Some(pin) = SCK_IN.borrow_ref_mut(cs).as_mut() {
            pin.clear_interrupt();
        }
    });
}

struct Stats {
    min: u32,
    max: u32,
    idx_max: usize,
    mean: u32,
    /// Combien d'échantillons dépassent 4 µs, soit près du double du plancher.
    hauts: u32,
    perdus: u32,
}

fn mesurer(sck: &mut Output<'static>, periode: u32, echantillons: &mut [u32]) -> Stats {
    ISR_HITS.store(0, Ordering::Relaxed);

    let mut perdus = 0u32;
    let mut recus = 0u32;
    let base = get_cycle_count();

    for (i, slot) in echantillons.iter_mut().enumerate() {
        if recus == 0 && perdus >= 20 {
            return Stats {
                min: 0,
                max: 0,
                idx_max: 0,
                mean: 0,
                hauts: 0,
                perdus: SAMPLES as u32,
            };
        }

        let echeance = base.wrapping_add((i as u32 + 1).wrapping_mul(periode));
        while get_cycle_count().wrapping_sub(echeance) > u32::MAX / 2 {}

        let avant = ISR_HITS.load(Ordering::Relaxed);
        let t_front = get_cycle_count();
        sck.set_high();

        let attente = get_cycle_count();
        loop {
            if ISR_HITS.load(Ordering::Relaxed) != avant {
                *slot = ISR_CCOUNT.load(Ordering::Relaxed).wrapping_sub(t_front);
                recus += 1;
                break;
            }
            if get_cycle_count().wrapping_sub(attente) > GIVE_UP_CYCLES {
                *slot = u32::MAX;
                perdus += 1;
                break;
            }
        }

        sck.set_low();
    }

    let seuil = CPU_HZ / 250_000; // 4 µs en cycles
    let mut min = u32::MAX;
    let mut max = 0u32;
    let mut idx_max = 0usize;
    let mut somme = 0u64;
    let mut n = 0u64;
    let mut hauts = 0u32;
    for (i, &c) in echantillons.iter().enumerate() {
        if c == u32::MAX {
            continue;
        }
        min = min.min(c);
        if c > max {
            max = c;
            idx_max = i;
        }
        if c > seuil {
            hauts += 1;
        }
        somme += u64::from(c);
        n += 1;
    }

    Stats {
        min: if n == 0 { 0 } else { min },
        max,
        idx_max,
        mean: somme.checked_div(n).unwrap_or(0) as u32,
        hauts,
        perdus,
    }
}

fn ns(cycles: u32) -> u32 {
    ((u64::from(cycles) * 1_000_000_000) / u64::from(CPU_HZ)) as u32
}

fn passe(phase: &str, sck: &mut Output<'static>, echantillons: &mut [u32]) {
    for hz in RATES_HZ {
        let periode = CPU_HZ / hz;
        let a = mesurer(sck, periode, echantillons);
        let b = mesurer(sck, periode, echantillons);
        for (nom, s) in [("1re", &a), ("2e ", &b)] {
            if s.perdus == SAMPLES as u32 {
                error!("{phase} {hz} Hz {nom} — AUCUN front. Strap GPIO25 → GPIO18 ?");
                continue;
            }
            info!(
                "{phase} {hz:>5} Hz {nom} — min {:>5} | moy {:>5} | max {:>6} ns à l'indice {:>4} | >4µs : {:>3}",
                ns(s.min),
                ns(s.mean),
                ns(s.max),
                s.idx_max,
                s.hauts
            );
        }
    }
}

#[main]
fn main() -> ! {
    esp_println::logger::init_logger_from_env();

    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    esp_alloc::heap_allocator!(#[esp_hal::ram(reclaimed)] size: 98768);

    let mut sck = Output::new(peripherals.GPIO25, Level::Low, OutputConfig::default());
    let mut sck_in = Input::new(
        peripherals.GPIO18,
        InputConfig::default().with_pull(Pull::Down),
    );
    sck_in.listen(Event::RisingEdge);
    critical_section::with(|cs| SCK_IN.borrow_ref_mut(cs).replace(sck_in));

    let mut io = Io::new(peripherals.IO_MUX);
    io.set_interrupt_handler(isr);

    let mut echantillons = [0u32; SAMPLES];

    info!("relink — banc T4-bis — origine du pic dans un gros binaire");

    passe("A repos ", &mut sck, &mut echantillons);

    let timg0 = TimerGroup::new(peripherals.TIMG0);
    let sw_int =
        esp_hal::interrupt::software::SoftwareInterruptControl::new(peripherals.SW_INTERRUPT);
    esp_rtos::start(timg0.timer0, sw_int.software_interrupt0);
    passe("B ordonn", &mut sck, &mut echantillons);

    let (mut controller, _interfaces) =
        esp_radio::wifi::new(peripherals.WIFI, Default::default()).expect("initialisation Wi-Fi");
    match controller.set_config(&WifiConfig::AccessPoint(AccessPointConfig::default())) {
        Ok(()) => info!("radio démarrée en point d'accès"),
        Err(e) => error!("la radio n'a pas démarré : {e:?} — la phase C ne vaut rien"),
    }

    loop {
        passe("C radio ", &mut sck, &mut echantillons);
        info!("--- passe terminée ---");
    }
}
