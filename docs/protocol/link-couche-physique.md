# Couche physique du lien

Brochage, tensions, horloge et rythme des bits du port link Game Boy. Ce
document est commun à la Gen 1 et à la Gen 2 : les deux générations partagent
la même couche physique et ne diffèrent que par le format du bloc transporté.
Les valeurs d'octets de l'échange Gen 1 sont documentées à part dans
[`gen1-link-protocol.md`](gen1-link-protocol.md).

Ce document ne source aucune constante du crate `protocol` — celui-ci ne
connaît ni le temps ni les tensions, il ne voit que des octets. Il source le
futur lot firmware, et ferme les deux cases laissées ouvertes par le schéma
[`docs/diagrams/materiel.html`](../diagrams/materiel.html) : « brochage à
sourcer » et « tensions à confirmer ».

**Récapitulatif de confiance** (11 entrées « Confiance » dans ce document) :
**8 confirmées** (≥ 2 sources indépendantes convergentes, ou Pan Docs sur un
point de matériel non contesté), **1 confirmée assortie d'une question
ouverte** (les quatre cadences d'horloge du CGB existent, mais aucune source
ne dit laquelle l'échange Gen 2 emploie), **1 probable** (source unique
nommée), **1 conséquence de conception** (pas un fait du protocole, mais ce
que les faits imposent au module). Détail dans chaque section.

Sources principales :

- **Pan Docs**, [« Serial Data Transfer (Link Cable) »](https://gbdev.io/pandocs/Serial_Data_Transfer_(Link_Cable).html)
  et [« External Connectors »](https://gbdev.io/pandocs/External_Connectors.html)
  — la documentation de référence du matériel Game Boy, maintenue par gbdev
  et recoupée depuis des décennies par les auteurs d'émulateurs. Reprise à
  l'identique par le [wiki gbdev](https://gbdev.gg8.se/wiki/articles/Serial_Data_Transfer_(Link_Cable)).
- **Dhole**, [« Sniffing Game Boy serial traffic with an STM32F4 »](https://dhole.github.io/post/gameboy_serial_1/)
  — capture du trafic link sur matériel réel avec un analyseur logique, avec
  relevé des fronts.
- **badd10de**, [« Game Boy Link Cables »](https://badd10de.dev/notes/gb-link-cables.html)
  — relevés au multimètre sur plusieurs câbles du commerce, et tensions
  mesurées par modèle de console.
- **kbembedded**, [« Flipper-Zero-Game-Boy-Pokemon-Trading »](https://github.com/kbembedded/Flipper-Zero-Game-Boy-Pokemon-Trading)
  — déjà source de [`gen1-link-protocol.md`](gen1-link-protocol.md) ; reprise
  ici pour son câblage, qui échange réellement avec une cartouche physique.
- **iamjackg**, [« d1mini-gb-shield »](https://github.com/iamjackg/d1mini-gb-shield)
  — carte qui relie un port link à un ESP8266, donc au même 3,3 V qu'un ESP32.

## Brochage du connecteur

- **Ce que c'est** — les six broches du port link, côté console.
- **Valeur / disposition** — 1 `VCC` (+5 V), 2 `SOUT` (données sortantes,
  fil rouge), 3 `SIN` (données entrantes, fil orange), 4 `P14` (inutilisée),
  5 `SCK` (horloge de décalage, fil vert), 6 `GND` (fil bleu). Disposition
  physique : 2, 4, 6 sur la rangée du haut, 1, 3, 5 sur celle du bas, vue de
  l'extérieur de la prise console, méplat vers le haut.
- **Source** — Pan Docs, « External Connectors », table « Link Port » : « 1
  VCC — +5V DC ; 2 SOUT red Data Out ; 3 SIN orange Data In ; 4 P14 — Not
  used ; 5 SCK green Shift Clock ; 6 GND blue Ground », et « Pin numbers are
  arranged as 2,4,6 in upper row, 1,3,5 in lower row; outside view of Game Boy
  socket; flat side of socket upside. » Recoupé par le
  [wiki ConsoleMods](https://consolemods.org/wiki/Game_Boy:Connector_Pinouts)
  et par badd10de, qui donne le même brochage relevé au multimètre.
- **Confiance** — confirmée.

## Le câble croise SIN et SOUT

- **Ce que c'est** — un câble link standard n'est pas droit : la sortie d'une
  console arrive sur l'entrée de l'autre. Une carte qui se branche à la place
  d'une console doit donc câbler `SOUT` console → entrée du module, et sortie
  du module → `SIN` console.
- **Valeur / disposition** — croisement des broches 2 et 3 à une extrémité.
- **Source** — Pan Docs : « because SIN and SOUT are crossed, colors Red and
  Orange are exchanged at one cable end. » kbembedded, dans sa table de
  câblage : « it's a crossover cable SI-SO ». badd10de recommande par ailleurs
  de ne rien supposer : « use a multimeter to do continuity check for each
  pin », les câblages variant selon les fabricants — surtout côté GBA.
- **Confiance** — confirmée (trois sources). **Conséquence de conception** :
  vérifier au multimètre chaque câble avant de souder, plutôt que de se fier
  à la couleur des fils.

## Deux tailles de fiche

- **Ce que c'est** — la DMG d'origine n'a pas le même connecteur physique que
  les modèles suivants, alors que le signal est identique.
- **Valeur / disposition** — grande fiche sur DMG, petite fiche sur Game Boy
  Pocket et au-delà. Des câbles à une fiche de chaque taille existent.
- **Source** — Pan Docs : « The original Game Boy used larger plugs than Game
  Boy Pocket and newer. Linking between older/newer Game Boy systems is
  possible by using cables with one large and one small plug though. »
- **Confiance** — confirmée. **Conséquence de conception** : viser la petite
  fiche et laisser un câble adaptateur traiter la DMG, plutôt que de dessiner
  deux connecteurs.

## Tension logique

- **Ce que c'est** — le niveau haut des signaux du port link, qui décide s'il
  faut adapter avant d'entrer dans le microcontrôleur.
- **Valeur / disposition** — **5 V sur Game Boy et Game Boy Color**, 3,3 V
  seulement quand un GBA exécute un jeu GBA.
- **Source** — Pan Docs donne la broche 1 à « +5V DC ». badd10de, mesures à
  l'appui : « When operating on a classic GB or GBC the voltage of these
  signals is of 5V, as opposed to the 3.3V while running a GBA game. »
- **Confiance** — confirmée. **La Gen 1 et la Gen 2 tournent toutes deux sur
  des consoles en 5 V** : il n'existe pas, dans le périmètre de ce projet, de
  cas où le port link est en 3,3 V.

## Adaptation de niveau : obligatoire

- **Ce que c'est** — la conséquence directe de l'entrée précédente pour un
  microcontrôleur en 3,3 V.
- **Valeur / disposition** — aucune broche d'ESP32 n'est tolérante 5 V ; le
  maximum absolu est Vdd + 0,3 V, soit 3,6 V. Une entrée à 5 V fait conduire
  les diodes de protection et dégrade la broche. Il faut donc un étage
  d'adaptation sur les trois signaux.
- **Source** — [Espressif, Hardware Design FAQ](https://docs.espressif.com/projects/esp-faq/en/latest/hardware-related/hardware-design.html)
  et le [forum ESP32](https://esp32.com/viewtopic.php?t=877). Côté précédent
  matériel, iamjackg est explicite : « The Game Boy runs at 5V, but both the
  ESP8266 and the ESP32 use 3.3V », et sa carte monte un convertisseur de
  niveau au format SparkFun.
- **Confiance** — **conséquence de conception**, appuyée sur une
  caractéristique constructeur.
- **Piège documenté** — les deux montages les plus cités branchent leur MCU
  *en direct* sur le port link : kbembedded sur Flipper Zero (« 33kΩ resistor
  on CLK » optionnel, rien d'autre) et Dhole sur NUCLEO-F411RE, qui note
  « most of the pins of the NUCLEO-F411RE are 5V tolerant ». Les deux
  reposent sur des STM32 à broches tolérantes 5 V. **Leur câblage ne se
  transpose pas à un ESP32.**
- **Choix retenu pour le cadrage** — les trois signaux sont unidirectionnels
  (`SCK` et `SOUT` entrent dans le module, `SIN` en sort), donc un tampon
  unidirectionnel suffit et un convertisseur bidirectionnel à MOSFET n'est pas
  nécessaire. Un `74LVC245` alimenté en 3,3 V accepte du 5 V en entrée ; sa
  limite connue — inutilisable sur un bus à pull-up type I²C — ne concerne pas
  ce lien. Rien n'est arrêté ici : c'est un cadrage, comme le schéma matériel.

## Ordre des bits

- **Ce que c'est** — le sens dans lequel le registre à décalage sort les bits.
- **Valeur / disposition** — bit de poids fort en premier.
- **Source** — Pan Docs, registre `FF01` (SB) : « Each cycle, the leftmost bit
  is shifted out (and over the wire) and the incoming bit is shifted in from
  the other side », suivi de la table de décalage qui montre `o.7` sortir en
  premier et `i.7` entrer en premier.
- **Confiance** — confirmée.

## Fronts d'horloge

- **Ce que c'est** — sur quel front le bit est présenté, et sur quel front il
  est lu.
- **Valeur / disposition** — le bit est placé sur la ligne au front
  **descendant**, et lu au front **montant**.
- **Source** — Dhole, relevé à l'analyseur logique : le bit sortant est posé
  à la chute du signal d'horloge, la lecture se fait au front montant.
- **Confiance** — probable (source unique explicite sur les fronts ; Pan Docs
  décrit le décalage sans nommer les fronts).

## Fréquence d'horloge

- **Ce que c'est** — la cadence imposée par la cartouche, qui est toujours
  celle qui fournit l'horloge dans notre montage (voir « Le rôle du module :
  suiveur, toujours » dans [`gen1-link-protocol.md`](gen1-link-protocol.md)).
- **Valeur / disposition** — **8192 Hz** hors mode CGB, soit environ 1 Ko/s et
  **~122 µs par bit**, donc ~977 µs par octet.
- **Source** — Pan Docs : « In Non-CGB Mode the Game Boy supplies an internal
  clock of 8192Hz only (allowing to transfer about 1 KByte per second minus
  overhead for delays). » Dhole mesure la même valeur sur matériel.
- **Confiance** — confirmée.

## Horloge rapide CGB — question ouverte

- **Ce que c'est** — le Game Boy Color sait cadencer le lien bien plus vite
  que la DMG. Savoir si l'échange Gen 2 s'en sert décide du budget de temps
  du firmware, et donc de la faisabilité d'un décodage logiciel.
- **Valeur / disposition** — en mode CGB, quatre cadences existent selon le
  bit 1 de `SC` et le mode double vitesse : 8192 Hz (1 Ko/s), 16384 Hz
  (2 Ko/s), 262144 Hz (32 Ko/s) et 524288 Hz (64 Ko/s). À 262144 Hz, un bit
  dure ~3,8 µs au lieu de ~122 µs — **32 fois moins de marge**.
- **Source** — Pan Docs, registre `FF02` (SC) : « Clock speed [CGB Mode only]
  […] If set to 1, enable high speed serial clock (~256 kHz in normal-speed
  mode) », et la table des quatre cadences.
- **Confiance** — confirmée pour l'existence des quatre cadences. **Ouvert :
  aucune source consultée ne dit laquelle l'échange Gen 2 emploie
  réellement.** kbembedded échange en Gen 2 sur matériel réel sans mentionner
  de cadence particulière, ce qui suggère 8192 Hz sans le démontrer.
- **Conséquence** — à trancher avant de figer la stratégie de décodage du
  firmware. Un décodage par interruption sur `SCK` est confortable à 122 µs
  par bit et hasardeux à 3,8 µs.

## Octet non prêt : le précédent repart

- **Ce que c'est** — ce qui arrive quand un côté n'a pas chargé son octet
  sortant à temps. C'est le mode de panne exact que la contrainte du §5.1 de
  la conception cherche à éviter.
- **Valeur / disposition** — l'octet précédent est réémis. Aucune erreur n'est
  signalée, aucun bit n'est perdu : la trame reste alignée, seule la donnée
  est fausse.
- **Source** — Pan Docs : « If it hasn't gotten around to loading up the next
  data byte at the time the transfer begins, the last one will go out again.
  Alternately, if it's ready to send the next byte but the last one hasn't
  gone out yet, it has no choice but to wait. »
- **Confiance** — confirmée. **Conséquence de conception** : une gigue
  d'interruption ne se manifestera pas par une désynchronisation visible mais
  par un octet dupliqué au milieu d'un bloc — silencieux, et indétectable sans
  vérification de bout en bout.

## Aucune ligne de sélection

- **Ce que c'est** — le port link n'expose pas d'équivalent du `CS` d'un bus
  SPI. Rien, électriquement, ne délimite un octet du suivant.
- **Valeur / disposition** — les six broches sont `VCC`, `SOUT`, `SIN`, `P14`,
  `SCK`, `GND` ; aucune n'est une sélection d'esclave.
- **Source** — Pan Docs, table « Link Port » (voir « Brochage du connecteur »
  ci-dessus). La broche 4 (`P14`) est marquée « Not used ».
- **Confiance** — confirmée. **Conséquence de conception** : le périphérique
  SPI esclave d'un ESP32 cadre ses transferts sur `CS` et se prête donc mal à
  ce lien. Un décodage sur interruption de `SCK` est la voie à instruire en
  premier — sous réserve de la question ouverte sur l'horloge rapide CGB.

## Ce que ce document ne dit pas

- **L'alimentation du module.** La broche 1 porte bien +5 V, mais aucune
  source consultée ne donne le courant que la console accepte d'y débiter.
  Tant que ce chiffre n'est pas établi, l'alimentation reste une case ouverte
  du schéma matériel — d'autant qu'un ESP32 avec Wi-Fi actif tire des pointes
  sans rapport avec ce qu'un port de liaison est censé fournir.
- **Le rythme entre octets côté module.** Il est déjà consigné dans
  [`gen1-link-protocol.md`](gen1-link-protocol.md), section « Rythme des
  octets ».
- **Le choix des broches de l'ESP32.** Il dépend de la carte retenue et
  relève de la spécification du lot firmware, pas du protocole.
