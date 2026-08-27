# Machine à états de l'échange Gen 1 : conception

- **Date** : 2026-08-27
- **Statut** : validé, prêt pour le plan d'implémentation
- **Périmètre** : `crates/protocol` uniquement — la machine à états de
  l'échange Trade Center Gen 1, et le codec de patch list qu'elle exige
- **Conception mère** :
  [`2026-08-27-relink-coeur-metier-design.md`](2026-08-27-relink-coeur-metier-design.md)
- **Sourçage** : [`docs/protocol/gen1-link-protocol.md`](../../protocol/gen1-link-protocol.md)

## 1. Objectif

Faire du module un partenaire d'échange que la cartouche accepte : conduire
un échange Trade Center complet, du premier octet de négociation jusqu'à la
poignée de main finale, puis en enchaîner un second dans la même session.

Ce lot ne livre que le cœur protocolaire. Rien de ce qui est écrit ici ne
touche `application`, ne connaît le réseau, ni ne suppose un matériel.

## 2. Ce que le sourçage a tranché avant le design

La conception mère laissait ouverte une question qui conditionnait
l'architecture (§11, risque 1) : le protocole tolère-t-il un octet de
remplissage signifiant « le partenaire n'a pas encore choisi » ? Sans lui,
l'échange direct entre joueurs distants était à repenser entièrement.

**Il le tolère.** Pendant la phase de sélection, la cartouche émet `0x00` en
boucle et accepte de le recevoir sans échéance. Le mécanisme d'attente du §5.2
de la conception mère — émettre un `Effect`, continuer à présenter l'octet
neutre, se réveiller sur `supply()` — s'applique tel qu'il a été écrit.

Trois autres acquis du sourçage dictent des choix de conception, chacun
contre une tentation naturelle :

- **Ne jamais compter les octets neutres.** Les sources se contredisent sur
  leur nombre après la négociation. La machine à états réagit à des marqueurs,
  pas à des compteurs, partout où elle le peut.
- **Ne jamais s'appuyer sur la longueur exacte de la patch list.** Les deux
  sources qui la donnent ne s'accordent pas à l'octet près. La lecture se
  termine sur ses deux terminateurs `0xFF`.
- **`0x61` est ambigu.** Il vaut « je propose le Pokémon d'index 1 » en phase
  de sélection et « je refuse » en phase de verdict. Seule la phase courante
  les distingue : une implémentation qui déciderait sur la valeur de l'octet
  seul serait fausse.

## 3. Architecture

**Une `Session` à phases, qui possède ses tampons.** Un seul type public, un
`enum Phase` interne, un module par groupe de phases, et environ un kilo-octet
d'état immobile : bloc sortant déjà corrigé, bloc entrant en cours de
réception, patch list sortante.

Deux alternatives ont été écartées :

- **Emprunter les tampons** (`Session<'a>`) plutôt que les posséder. Économise
  une copie de 415 octets au firmware, mais fait remonter une durée de vie
  dans tout ce qui touche la session, pour un gain qui n'est pas mesuré.
- **Une couche de tramage sous la machine métier.** Séduisant sur le papier,
  mais le découpage des trames dépend de la phase métier : les deux couches
  se reparleraient à chaque octet.

Le découpage retenu suit les groupes de phases, un fichier par groupe :

```
src/session/mod.rs        Session, Step, Effect, Decision, aiguillage de step()
src/session/link.rs       négociation des rôles, acquittement, menu du Cable Club
src/session/transfer.rs   préambule, graine, bloc d'échange, patch list
src/session/table.rs      sélection, verdict, retour à la table
src/gen1/patch_list.rs    codec de patch list, sans état
```

**La génération n'est pas encore un `enum`.** La conception mère prévoit un
dispatch `Generation { One, Two }` (§5.3). Ce lot ne livre pas la Gen 2 : un
variant qui ne mènerait à rien serait du code mort. Les valeurs qui changent
d'une génération à l'autre sont donc regroupées dès maintenant dans une table
de constantes (`LinkBytes`), et la session en reçoit une. Le lot Gen 2
ajoutera une seconde table, sa phase de courrier et l'`enum` d'aiguillage,
sans rien déplacer.

## 4. Surface publique

```rust
pub struct Session { /* ~1 Ko, sans allocation */ }

pub struct Step {
    pub outgoing: u8,
    pub effect: Option<Effect>,
}

impl Session {
    /// Le module joue toujours le suiveur : la cartouche cadence.
    pub fn gen1(offered: TradeBlock) -> Self;

    /// O(1), sans allocation, infaillible. Appelée à chaque octet.
    pub fn step(&mut self, incoming: u8) -> Step;

    /// Débloque une session en attente d'une décision externe.
    pub fn supply(&mut self, decision: Decision);

    /// L'équipe du partenaire, dès que `PartnerBlockReceived` a été émis.
    pub fn partner_block(&self) -> Option<&TradeBlock>;
}

pub enum Effect {
    LinkEstablished,
    PartnerBlockReceived,
    OfferNeeded,
    PartnerOffered { index: u8 },
    VerdictNeeded,
    TradeAgreed { offered: u8, received: u8 },
    TableLeft,
    LinkBroken,
}

pub enum Decision {
    Offer(u8),
    Accept,
    Reject,
    Leave,
    Party(TradeBlock),
}
```

`step()` ne rend jamais d'erreur et ne panique pas : tout octet inattendu est
une transition, pas une faute. Au plus un effet par octet — les effets sont des
fronts, et le protocole envoie assez d'octets pour qu'aucun ne se perde.

`Decision::Party` réarme la session avec une nouvelle équipe. Elle est
indispensable entre deux échanges d'une même session : après un échange
réussi, l'équipe du module a changé, et la cartouche réémet aussitôt
préambule, bloc et patch list pour resynchroniser.

## 5. Les phases

| Phase | Ce qu'on émet | Ce qui en sort |
|---|---|---|
| `Negotiating` | `0x02` dès qu'on voit `0x01`, sinon on renvoie ce qu'on reçoit | `0x60` reçu → `Menu`, effet `LinkEstablished` |
| `Menu` | on renvoie ce qu'on reçoit, pour laisser le joueur choisir | `0xD4` → `Waiting` ; `0xD5` ou `0xD6` → `Broken`, effet `LinkBroken` |
| `Waiting` | on renvoie ce qu'on reçoit | premier `0xFD` → `Preamble` |
| `Preamble` | on renvoie ce qu'on reçoit | 10 × `0xFD` comptés → `Seed` |
| `Seed` | on renvoie ce qu'on reçoit | 19 octets consommés (10 d'aléa + 9 de préambule) → `Block` |
| `Block` | l'octet du bloc sortant à la position courante | 415 octets → `PatchHeader` |
| `PatchHeader` | on renvoie ce qu'on reçoit | 6 × `0xFD` comptés → `PatchList` |
| `PatchList` | les octets de patch list sortants, après les sept octets neutres d'en-tête | second terminateur `0xFF` reçu puis section close → `Select`, effet `PartnerBlockReceived` |
| `Select` | l'offre si elle est connue, `0x00` sinon | à l'entrée, effet `OfferNeeded` ; `0x60`+i reçu → effet `PartnerOffered` ; les deux offres connues → `Verdict`, effet `VerdictNeeded` ; `0x6F` → `Waiting`, effet `TableLeft` |
| `Verdict` | le verdict s'il est connu, `0x00` sinon | `0x62` des deux côtés → `Trading`, effet `TradeAgreed` ; `0x61` → retour à `Select` |
| `Trading` | `0x00` | premier `0xFD` → `Preamble`, pour l'échange suivant |
| `Broken` | `0xD6` | recevoir `0x01` relance la négociation |

Trois règles traversent toutes les phases :

1. **Recevoir `0x01` en cours de route** signifie que la cartouche a
   redémarré sa négociation. La session repart de `Negotiating` et répond
   `0x02`, quel que soit l'endroit où elle était.
2. **Les compteurs sont bornés.** Aucune phase ne peut faire déborder un
   index de tampon, quelle que soit la suite d'octets reçue.
3. **Le renvoi est le comportement par défaut** dans les phases de
   synchronisation, comme le fait la seule implémentation vérifiée sur
   matériel. C'est ce qui rend la machine tolérante aux longueurs que les
   sources ne s'accordent pas à fixer.

## 6. Le codec de patch list

Deux fonctions pures, sans état, dans `gen1::patch_list` :

- **Construire** — à partir des 264 octets de données d'équipe du bloc
  (offset 19, ce que `trade_block.rs` nomme déjà `OFF_PARTY_DATA`), produire
  la liste des positions occupées par un `0xFE`, incrémentées de un, découpée
  en deux parties (`0x00`–`0xFB` puis `0xFC`–`0x107`), chacune close par
  `0xFF`, le reste à `0x00`. Le bloc sortant est corrigé en conséquence :
  chaque `0xFE` devient `0xFF` sur le fil.
- **Appliquer** — à partir d'une patch list reçue, remettre `0xFE` aux
  positions indiquées dans l'équipe entrante.

Sans ce codec, tout Pokémon dont les données contiennent un octet `0xFE`
arriverait corrompu : `0xFE` est l'octet « pas de câble » du port série, et
n'est jamais transmis tel quel.

**Un désaccord de sources est assumé ici.** Les deux sources qui documentent
la patch list ne couvrent pas la même zone : l'une la limite aux données
d'équipe, l'autre balaie le bloc entier. L'implémentation suit la première,
seule vérifiée contre une cartouche physique. La conséquence est identifiée :
un surnom contenant le caractère « 8 », codé `0xFE`, tombe dans la zone de
désaccord. Le détail et le test à faire passer dès qu'un montage matériel
existe sont dans
[`docs/protocol/gen1-link-protocol.md`](../../protocol/gen1-link-protocol.md).

## 7. L'attente et les décisions

Deux points d'attente, et deux seulement : `OfferNeeded` et `VerdictNeeded`.
Tant que la `Decision` correspondante n'est pas arrivée, la session présente
`0x00` — exactement ce que la cartouche envoie elle-même dans ces phases. Le
jeu y lit un dresseur qui hésite dans ses menus.

Les quatre parcours produit s'y branchent sans que la machine à états les
distingue :

| Parcours | `Offer` | `Accept` |
|---|---|---|
| Évolution par échange | immédiate, index 0 | immédiate |
| Dépôt dans le pool | immédiate, le leurre | immédiate |
| Retrait du pool | immédiate, la réservation | immédiate |
| Échange direct | après aller-retour réseau | après aller-retour réseau |

Seul le dernier attend réellement. C'est le sens du §5.2 de la conception
mère : le mécanisme coûte zéro aux trois autres.

## 8. Tests

**Une cartouche simulée.** Une doublure de test qui joue le côté jeu à partir
des constantes sourcées, branchée sur la session octet par octet. Elle permet
d'écrire des échanges complets comme des scénarios lisibles : lien établi,
Trade Center, bloc échangé, offres croisées, accord, puis **un second échange
dans la même session** — le cas que le protocole impose et qu'un test unitaire
par phase ne verrait jamais.

Elle vaut ce que vaut le sourçage, et ne remplace pas une trace réelle. C'est
assumé : elle est là pour attraper les régressions et les incohérences de
transition, pas pour prouver l'accord avec le matériel.

**Property-based sur l'infaillibilité.** Depuis chaque phase, une suite
d'octets arbitraires ne doit jamais paniquer, jamais déborder, jamais bloquer
la session dans un état dont `0x01` ne la sortirait pas. Même forme que
`tests/robustesse.rs`, qui couvre déjà les codecs.

**Aller-retour du codec de patch list.** Pour des données d'équipe
arbitraires : construire, corriger, appliquer, et retrouver les octets
d'origine à l'identique.

**`no_std` sans allocateur.** `cargo build -p relink-protocol --target
thumbv7em-none-eabihf` reste la seule preuve que la contrainte tient.

## 9. Hors périmètre

- **Le Colosseum.** Reconnu dans le menu et refusé proprement ; les combats
  ne sont pas dans le projet.
- **La Gen 2.** Ses valeurs de machine à états sont sourcées et consignées,
  mais rien ne les exerce tant que le codec du bloc Gen 2 n'existe pas.
- **Le rôle de leader.** Le module reste suiveur ; il n'a donc ni horloge, ni
  délai d'attente, ni élection à gérer.
- **Tout ce qui touche `application`.** Le branchement des parcours sur les
  décisions viendra avec le lot qui en a besoin.

## 10. Risques

| Risque | Portée | Traitement |
|---|---|---|
| L'étendue de la patch list est contredite entre sources. | Un surnom contenant un « 8 » pourrait être corrompu dans un sens ou dans l'autre. | Suivre la source vérifiée sur matériel, documenter le désaccord, écrire le test qui le tranchera dès qu'un montage existe. |
| Les longueurs exactes (patch list, octets neutres, octets de fin de bloc) ne sont pas fermes. | Machine à états. | Ne compter que là où les sources s'accordent ; ailleurs, renvoyer et attendre le marqueur suivant. |
| La cartouche simulée reproduit les mêmes erreurs que le sourçage. | Tests. | Assumé et écrit. Seul un montage matériel lèvera ce risque ; le lot firmware apportera des traces réelles, rejouables en test. |
