# relink — cœur métier : conception

- **Date** : 2026-08-27
- **Statut** : validé, prêt pour le plan d'implémentation
- **Périmètre** : les deux crates du cœur, `protocol` et `application`

## 1. Objectif

Permettre aux joueurs des Pokémon Game Boy et Game Boy Color d'échanger à
nouveau, via un module matériel qui se branche sur le port link et se fait
passer pour un partenaire d'échange.

Ce document ne spécifie que le **cœur métier**. Le firmware, l'API HTTP,
l'application mobile et les adaptateurs concrets auront chacun leur propre
spécification.

## 2. L'idée qui structure tout

Du point de vue de la cartouche il n'existe qu'une seule chose : un échange
1:1 avec un partenaire qui présente une équipe et qui confirme ou annule. Le
module est toujours ce partenaire. Les quatre parcours produit ne diffèrent
que par **qui fournit l'équipe du partenaire virtuel et qui prend ses
décisions** :

| Parcours | Équipe du partenaire | Décideur |
|---|---|---|
| Évolution par échange | synthétisée localement | automate, local |
| Dépôt dans le pool | un leurre | automate, local |
| Retrait du pool | la réservation | pré-décidé, hors session |
| Échange direct | l'équipe du joueur distant | le joueur distant, en différé |

Une seule machine à états, un seul jeu de codecs, quatre stratégies derrière
un port. C'est pourquoi couvrir les quatre parcours coûte à peine plus que
d'en couvrir un.

L'échange direct **ne peut pas** être relayé octet par octet : la cadence
série (~8192 Hz) est incompatible avec la latence d'internet. Il fonctionne
en **mode bufferisé** — le module détient l'équipe adverse complète et répond
à vitesse fil ; seules les décisions traversent le réseau.

## 3. Décisions structurantes

| Décision | Choix | Raison |
|---|---|---|
| Langage | Rust | Le même code de protocole tourne dans le backend et dans le firmware. Un seul codec, testé une fois : une divergence entre les deux corromprait des sauvegardes. |
| Architecture | Hexagonale, deux couches | Les deux moitiés du cœur n'ont pas les mêmes contraintes (cf. §4). |
| Générations v1 | Gen 1 + Gen 2 | Couche physique identique, machine à états identique, deux formats de bloc. |
| Nom | `relink` | Aucune marque déposée. Un dépôt public nommé `poke-*` est une cible gratuite. |
| Visibilité | Public dès le premier commit | La rétro-ingénierie doit être documentée en clean-room ; un historique public qui montre cette hygiène depuis le début ne se reconstitue pas après coup. |
| Licence `protocol` | MIT OU Apache-2.0 | Ce crate a vocation à être repris par des émulateurs et d'autres outils. Une licence permissive lui donne une chance de devenir la référence du protocole d'échange Gen 1/2. |
| Licence `application` | AGPL-3.0 | Le service ne doit pas pouvoir être repris en hébergé fermé. |

## 4. Architecture

Le cœur n'est pas un bloc. Ce sont deux couches aux contraintes disjointes :

```
crates/protocol      no_std, sans alloc   codecs Gen 1/2, machine à états    zéro I/O
crates/application   std              dépôt, réservation, provenance     ports en traits
```

**Sens de la dépendance : `application` → `protocol`, jamais l'inverse.**
`application` consomme les *valeurs* de `protocol` (`Pokemon`, `TradeBlock`,
`Generation`, l'éligibilité Capsule Temporelle) et lui fournit une
`PartnerStrategy`. Il ne connaît rien de `Session` ni du fil.

Précision apportée par le plan des codecs : `protocol` tient **sans
allocateur**. Toutes ses structures sont de taille fixe — un bloc d'échange
fait 415 octets, une équipe six emplacements — et le codec est une *vue* sur
des octets plutôt qu'un analyseur qui reconstruit. Contrainte plus stricte que
prévu initialement, donc sûre, et vérifiable : le crate doit se compiler pour
une cible embarquée dépourvue d'allocateur.

`protocol` est écrit en cœur fonctionnel pur : `(état, événement) → (état,
effets)`, aucune I/O, aucun trait async. C'est ce qui le rend `no_std`,
partageable avec le firmware, et **rejouable** — une trace d'échange qui a
mal tourné se rejoue en test unitaire à l'identique.

`application` est en hexagonal canonique, avec des ports en traits. Ses use
cases n'ont aucune contrainte temps réel et gagnent à être lisibles.

Le port `PartnerStrategy` est la couture entre les deux : `protocol` le
déclare, `application` en fournit trois implémentations sur quatre. La
quatrième — l'évolution — vit entièrement dans `protocol`, ce qui est la
traduction technique de « l'évolution par échange marche sans réseau, sans
compte et sans serveur ».

## 5. Le crate `protocol`

### 5.1 La contrainte qui dicte l'API

En série synchrone Gen 1/2, la cartouche fournit l'horloge et l'octet sortant
doit être prêt **avant** le front. L'API ne peut donc ni allouer, ni attendre,
ni faillir :

```rust
pub struct Step {
    pub outgoing: u8,
    pub effect: Option<Effect>,
}

impl Session {
    /// O(1), sans allocation, infaillible. Appelée à chaque octet.
    pub fn step(&mut self, incoming: u8) -> Step;

    /// Débloque une session en attente d'une décision externe.
    pub fn supply(&mut self, decision: Decision);
}
```

### 5.2 Concilier temps réel dur et latence réseau

Quand le domaine a besoin d'une décision qu'il n'a pas — typiquement la
confirmation du joueur distant — **il n'attend pas**. Il émet un `Effect`,
passe en état d'attente, et continue de présenter l'octet de remplissage qui
signifie « le partenaire n'a pas encore choisi ». Le jeu l'interprète comme un
dresseur qui navigue dans ses menus et patiente sans broncher. `supply()`
réveille la session quand la réponse arrive.

Ce mécanisme rend l'échange direct possible et ne coûte rien aux trois autres
parcours, qui n'attendent jamais.

### 5.3 Générations

Dispatch par `enum Generation { One, Two }` et `match` — pas de trait objet,
pas d'allocation, compatible `no_std`. La séquence des phases (handshake,
choix de salle, échange du bloc, sélection, confirmation) est commune ; seul
le format du bloc diffère.

Le bloc Gen 1 fait 415 octets. Le format Gen 2 est plus large et inclut le
courrier ; ses dimensions exactes sont à établir par rétro-ingénierie.

### 5.4 Conversion Gen 1 ↔ Gen 2

**On n'invente aucune conversion.** Les jeux ont déjà tranché : c'est la
Capsule Temporelle, et ses règles sont strictes — pas d'espèce ni de capacité
postérieure à la Gen 1 dans le sens descendant. Le domaine implémente ces
règles telles quelles.

Conséquence directe : le domaine sait dire si un Pokémon donné est **éligible
ou non** à une cartouche donnée. C'est ce qui alimentera le filtrage côté
application mobile.

### 5.5 Courrier Gen 2

Le courrier fait partie du bloc d'échange, on ne peut pas ne pas le
transporter. Mais rien n'exige de l'interpréter. Il transite donc **opaque** :
octets conservés à l'identique, jamais lus, jamais validés.

## 6. Le crate `application`

### 6.1 Use cases

Trois seulement, l'évolution ne remontant pas jusqu'ici :

1. **Déposer** un Pokémon dans le pool
2. **Réserver puis retirer** un Pokémon du pool
3. **Relayer** un échange direct entre deux joueurs

### 6.2 Ports déclarés

`PoolRepository`, `LegalityChecker` (PKHeX.Core derrière), `ModuleTransport`
(MQTT), `Clock`, `Notifier`, et `IdSource` — ce dernier ajouté à
l'implémentation : le domaine ne tire jamais d'aléa lui-même, pas plus qu'il ne
lit l'horloge, et c'est ce qui rend le test d'invariant reproductible.

Aucun n'a d'implémentation dans ce lot. C'est le principe.

### 6.3 Identité

Le dresseur, c'est-à-dire le nom OT et l'identifiant de dresseur lus sur la
cartouche. Suffisant pour établir la provenance. Les comptes utilisateurs sont
un problème d'adaptateur et ne remontent pas dans le cœur.

## 7. Le commit : là où l'on peut détruire des données

Un échange se conclut **physiquement** sur la cartouche. Il n'y a pas de
rollback : une fois l'animation passée, le Pokémon est dans la sauvegarde, que
le serveur l'ait su ou non. C'est une transaction distribuée dont l'un des
participants ne sait pas annuler.

Le titre de cette section disait initialement « le **seul** endroit où l'on peut
détruire des données ». Le §7.4, écrit plus tard, l'a démenti : au dépôt aussi
la cartouche perd le Pokémon irréversiblement. Le commit reste l'endroit le plus
dangereux, il n'est pas le seul.

La réponse tient en trois pièces :

1. **Identifiant émis à la réservation.** Le serveur génère l'ID avant que
   quoi que ce soit ne parte vers le module. Toute déduplication s'appuie
   dessus.
2. **Journal d'intention côté module.** Le module écrit en flash « je vais
   commiter la réservation *R* » **avant** d'entrer en phase de confirmation,
   puis « *R* est commitée » juste après. À la reconnexion il rejoue ce qui n'a
   pas été acquitté. Le serveur, idempotent par ID, absorbe les doublons — ce
   qui rend MQTT QoS 1 suffisant.
3. **L'entrée quitte le pool à la réservation, pas au commit.** Sinon deux
   joueurs peuvent réserver la même.

### 7.1 La fenêtre irréductible

Si le module est détruit ou sa flash perdue pendant l'échange, **personne ne
saura jamais** si la cartouche a reçu. Deux issues, il faut en choisir une :

- rendre l'entrée au pool → risque de **duplication** ;
- la laisser consommée → risque de **perte**.

**Décision : on choisit la perte.** Une duplication non détectée contamine le
pool de façon permanente et irrattrapable — c'est exactement ce qui a ruiné la
crédibilité des GTS non officielles. Une perte est bornée, visible, et
rattrapable par un traitement de litige manuel.

En cas d'ambiguïté, l'entrée reste donc consommée et l'incident est journalisé
pour ce traitement futur, hors périmètre v1.

### 7.2 Expiration des réservations

TTL obligatoire : sans lui, un joueur qui ne branche jamais son module gèle le
pool.

**Corrigé le 2026-08-27, en écrivant le plan du crate `application`.** Ce
paragraphe disait : « à l'expiration l'entrée revient au pool — et c'est le seul
cas où elle revient, puisqu'aucun commit n'a alors jamais été tenté. » C'était
faux, et d'une façon qui ouvrait précisément le trou que tout le §7 cherche à
fermer.

Le serveur ne sait pas qu'aucun commit n'a été tenté. Il sait qu'il n'en a
**pas entendu parler**. Un module qui a reçu la réservation, remis le Pokémon à
la cartouche, puis perdu le réseau, est indiscernable d'un module qui n'a jamais
rien reçu. Rendre l'entrée au pool dans ce cas produit une duplication.

**La règle corrigée :** le TTL ne protège que contre une réservation **qui n'est
jamais parvenue à un module**. Une entrée porte donc, en plus de son échéance,
le fait qu'un module en ait accusé réception :

- **Réservée, non remise** — le module n'a jamais accusé réception. À
  l'échéance, l'entrée revient au pool. Aucun commit n'a pu être tenté, puisque
  rien n'est jamais arrivé jusqu'à une cartouche.
- **Réservée et remise** — un module a accusé réception. L'entrée ne revient
  **jamais** automatiquement. Seul le module peut la trancher, en confirmant ou
  en signalant l'abandon. Un module détruit avant d'avoir parlé laisse l'entrée
  bloquée, ce qui relève du traitement de litige manuel.

C'est le même arbitrage qu'au §7.1, appliqué au bon endroit : on choisit la
perte. Un Pokémon bloqué est un incident visible et rattrapable ; un Pokémon
dupliqué ne l'est pas.

L'accusé de réception vient du **module**, pas du courtier de messages : qu'un
courtier ait accepté un message ne dit rien de ce que le module en a fait.

### 7.3 L'échange direct : un commit à deux cartouches

Les §7.1 et §7.2 traitent d'un échange où une seule cartouche s'engage. L'échange
direct en met deux, et le raisonnement ne s'y transpose pas.

Si la cartouche de A commite et que celle de B est débranchée juste après :
A a donné son Pokémon et reçu celui de B, mais B a toujours le sien. Une
**duplication et une perte simultanées**, le pire cas du §7.1 en une seule
opération.

Le mécanisme d'attente du §5.2 offre déjà une phase de préparation : le module
de A refuse de laisser sa cartouche atteindre la confirmation tant que B n'a
pas confirmé. C'est un commit en deux phases, où l'état d'attente est la phase
de préparation. Il réduit la fenêtre à l'intervalle entre les deux
libérations — il ne la supprime pas.

Deux options ont été examinées :

**A — Commit en deux phases.** Les deux modules préparent, le serveur libère
les deux le plus simultanément possible. Les joueurs échangent vraiment en même
temps. Fenêtre résiduelle non nulle, et un cas de duplication qui échappe à
l'arbitrage « on choisit la perte » du §7.1.

**B — Appariement par le pool.** L'échange direct n'est pas un
protocole distinct : c'est un dépôt et un retrait appariés, réservés l'un à
l'autre. Deux commits indépendants, chacun à une seule cartouche, chacun
couvert par l'arbitrage déjà validé. **Le commit à deux phases disparaît
entièrement**, et avec lui son cas de duplication.

Ce que B coûte : les deux joueurs n'échangent plus au même instant. B reçoit
quand il rebranche son module. Pour deux amis dans des fuseaux différents, ce
n'est pas une régression — et la cartouche, elle, vit un échange live dans les
deux cas.

Ce que B ne change pas : le tableau du §2 reste valide, l'équipe du partenaire
virtuel vient toujours du joueur distant et sa décision est toujours prise en
différé.

**Décision : B**, retenue le 2026-08-27. A ajoutait le seul mécanisme du
projet capable de produire une duplication non arbitrable, pour une
simultanéité dont personne n'a besoin.

Conséquence pour l'implémentation : il n'existe **aucun** chemin de commit à
deux cartouches dans le code. `application` ne connaît qu'un seul use case de
commit, celui du §7, et l'échange direct se réduit à un dépôt et un retrait
liés par un appariement.

### 7.4 L'autre direction : le dépôt

**Ajouté le 2026-08-27, après relecture adversariale des contrats de ports.**
Tout ce qui précède protège la direction **retrait** : identifiant émis avant
que quoi que ce soit ne parte, journal d'intention côté module, déduplication
par cet identifiant. La direction **dépôt** n'était protégée par rien — alors
que la cartouche y perd le Pokémon tout aussi irréversiblement.

Le scénario, sans aucune faute d'implémentation :

1. La cartouche cède le Mew. L'animation est passée, la sauvegarde ne l'a plus.
2. Le module journalise et publie le dépôt.
3. Le serveur crée l'entrée et la commite en base. **L'acquittement se perd.**
4. Le module rejoue son journal, exactement comme il rejoue un commit.
5. Le serveur crée une **seconde** entrée, avec un identifiant neuf.
6. Deux joueurs les réservent. Deux cartouches reçoivent le Mew. Duplication.

**La règle : qui agit le premier émet l'identifiant.**

C'est la formulation générale dont le §7 point 1 n'était qu'un cas particulier.
Au retrait, c'est le serveur qui agit le premier — il réserve avant que rien ne
parte — donc c'est lui qui émet l'identifiant. Au dépôt, c'est le **module**
qui agit le premier : la cartouche a déjà cédé le Pokémon quand le serveur
apprend son existence. C'est donc au module d'émettre l'identifiant, en même
temps qu'il écrit son entrée de journal, et de le rejouer inchangé à chaque
tentative.

Le serveur déduplique dessus. Un dépôt rejoué dix fois produit une entrée et
une seule, et rend la même que la première fois.

Un identifiant frappé côté serveur ne peut pas jouer ce rôle : par
construction, il est neuf à chaque tentative. C'est précisément ce qui rendait
le rejeu dangereux.

**La clé ne s'oublie jamais.** La fenêtre de rejeu d'un module n'est pas
bornée — c'est la prémisse même du §7.2, un module peut dormir des mois dans un
tiroir. Une déduplication adossée à la seule existence de l'entrée serait donc
défaite par une politique de rétention ordinaire : purger les entrées tranchées
au bout de quatre-vingt-dix jours suffirait à faire renaître le dépôt d'un
Pokémon déjà remis à quelqu'un d'autre. Ce qui doit survivre, c'est le registre
des clés, pas l'entrée.

Asymétrie à noter : seule la direction dépôt est critique à cet égard. Un commit
rejoué après purge ne trouve plus sa réservation et ne rend rien au pool — c'est
une perte, pas une duplication.

**La clé est unique globalement et pour toujours**, tous modules confondus. Un
compteur de journal local partant de 1 — l'implémentation réflexe côté firmware —
ferait entrer en collision le premier dépôt de chaque module. Le serveur ne peut
pas distinguer une collision d'un rejeu : il avalerait silencieusement un dépôt
légitime, la cartouche ayant déjà cédé son Pokémon. C'est une perte systématique
et sans coupable. La clé doit donc mêler l'identité du module au compteur, ou
tenir dans cent vingt-huit bits tirés au hasard.

## 8. Tests

Tout se teste sans matériel. C'est l'intérêt principal de la découpe.

- **`protocol`** — aller-retour de codec en property-based (`proptest`), plus
  des vecteurs hexadécimaux écrits à la main et sourcés dans `docs/protocol/`.
  Les traces d'échange se rejouent : une session qui a mal tourné sur du vrai
  matériel devient un test de non-régression, octet pour octet.
- **`application`** — chaque use case contre des doublures en mémoire de tous
  les ports.
- **Test d'invariant du commit** — on énumère les interruptions possibles
  (coupure avant l'écriture du journal, après, pendant l'acquittement,
  redémarrage du module, rejeu du message MQTT) et on assert sur une seule
  propriété : *jamais de duplication*. Ce test doit exister **avant** le code
  qu'il protège.

**Fixtures synthétiques, jamais capturées.** Aucune ROM ni sauvegarde tierce
dans le dépôt. Reproductible, et cela évite une discussion inutile.

## 9. Hygiène de rétro-ingénierie

Non négociable, et c'est la raison d'être de `docs/protocol/` :

- chaque constante et chaque champ du bloc d'échange cite sa source ;
- aucun code Nintendo, aucune ROM, aucun extrait désassemblé n'entre dans le
  dépôt ;
- aucune reprise de marque dans le nom, le domaine ou les ressources ;
- pas de monétisation du service.

C'est le modèle Pretendo, et c'est ce qui a permis à ce projet d'exister.

## 10. Hors périmètre

**De ce lot** — chacun aura sa propre spécification : le firmware, l'API HTTP,
l'application mobile, les adaptateurs MQTT et PKHeX, les comptes utilisateurs
et l'authentification.

**Du projet en v1** : la Gen 3 et le Multi-Play GBA, les combats link,
l'adaptateur Mobile GBC, la détection de triche, le traitement des litiges.

## 11. Risques et incertitudes

| Risque | Portée | Traitement |
|---|---|---|
| Les valeurs d'octets exactes du handshake et de la phase de sélection ne sont pas connues à ce jour. | Elles conditionnent le mécanisme d'attente du §5.2 : s'il s'avérait qu'aucun octet de remplissage n'est toléré, l'échange direct devrait être repensé. | Établies par rétro-ingénierie et sourcées une par une avant l'implémentation de la machine à états. C'est le seul endroit où ce design peut devoir bouger. |
| Le format exact du bloc Gen 2, courrier inclus. | Codec Gen 2 uniquement. | Idem ; le courrier étant transporté opaque, seul son décalage et sa taille importent. |
| Contrainte temps réel non tenue sur la cible embarquée. | Firmware. | Le style du §5.1 (O(1), sans allocation) est précisément ce qui rend la contrainte vérifiable ; à mesurer lors du lot firmware. |
