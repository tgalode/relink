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
    assert!(effets.contains(&Effect::TradeAgreed {
        offered: 0,
        received: 3
    }));
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
        *accords[1],
        Effect::TradeAgreed {
            offered: 0,
            received: 1
        }
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
