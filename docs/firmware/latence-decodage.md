# Latence du décodage par interruption

Ce que coûte, sur silicium, le décodage du lien par interruption sur `SCK` —
et ce que ce coût autorise ou interdit au futur firmware.

Ce document ne source aucune constante du crate `protocol` : celui-ci ne
connaît ni le temps ni les tensions. Il source le lot firmware, et il répond à
la question laissée ouverte par `docs/protocol/link-couche-physique.md`,
section « Aucune ligne de sélection » : le repli logiciel tient-il ?

**Récapitulatif de confiance** (7 entrées) : **5 confirmées** (mesure directe,
montage décrit et rejouable), **2 conséquences de conception** (pas des faits
du matériel, mais ce que les mesures imposent au firmware). Une méthode
invalide est signalée en fin de document plutôt que passée sous silence.

## Le montage

- **Carte** — ESP32-WROOM-32 sur carte DevKit 38 broches, révision de puce
  v3.1, bi-cœur Xtensa LX6, 4 Mo de flash, `CpuClock::max()` soit 240 MHz.
- **Câblage** — un strap entre `GPIO25` et `GPIO18`. Rien d'autre : les deux
  broches sont sur la même carte, la masse est commune par construction, et
  tout tient en 3,3 V. Schéma : [`../diagrams/banc-esp32.html`](../diagrams/banc-esp32.html).
- **Banc** — [`tools/banc-esp32`](../../tools/banc-esp32). `GPIO25` bat
  l'horloge comme le ferait la cartouche ; `GPIO18` la reçoit et l'ISR
  horodate son entrée. Les deux moitiés lisent le même compteur de cycles
  (`CCOUNT`), donc la soustraction ne demande aucun étalonnage.
- **Ce qui est mesuré** — le temps entre la décision d'émettre un front et
  l'horodatage pris à l'entrée de l'ISR. Il inclut l'appel `set_high()` du
  HAL, la synchronisation de l'entrée, le contrôleur d'interruptions et
  l'aiguillage GPIO d'esp-hal. **Ce n'est donc pas la latence d'interruption
  pure**, mais la grandeur dont dépend réellement la question « le module
  peut-il répondre à temps ».
- **Volume** — 2000 échantillons par cadence et par série, deux séries par
  cadence, trois cadences (4096, 8192, 16384 Hz), trois phases.
- **Date** — 2026-09-11.

## Latence en régime établi, puce au repos

- **Ce que c'est** — le temps de réaction quand rien d'autre ne tourne.
- **Valeur** — plancher **2200 ns**, moyenne 2204 ns, pire **2291 ns**. Aucun
  échantillon au-dessus de 4 µs. Zéro front perdu.
- **Source** — mesure directe, montage ci-dessus, phase A du banc.
- **Confiance** — confirmée.

## Latence en régime établi, radio active

- **Ce que c'est** — la même grandeur avec la radio Wi-Fi allumée en point
  d'accès ouvert, **sur le même cœur que la mesure**, donc dans le pire cas.
- **Valeur** — plancher **2712 ns** (+23 %), et surtout une queue de
  distribution qui s'épaissit : **jusqu'à 6,7 µs**, avec 1 à 5 échantillons
  sur 2000 au-dessus de 4 µs, répartis dans la série (indices 101, 569…) et
  non groupés au début. Zéro front perdu.
- **Source** — mesure directe, phase C du banc.
- **Confiance** — confirmée. La queue est ce qui compte : un lien temps réel
  ne casse pas sur sa moyenne.

## L'ordonnanceur ne coûte rien

- **Ce que c'est** — le surcoût du tic d'`esp-rtos`, nécessaire dès qu'on veut
  la radio.
- **Valeur** — aucun écart mesurable. Les phases A et B donnent les mêmes
  chiffres à la dizaine de nanosecondes près.
- **Source** — mesure directe, phases A et B du même binaire.
- **Confiance** — confirmée.

## Reprise à froid du cache de flash

- **Ce que c'est** — un pic isolé, de loin le plus grand chiffre de tout le
  jeu de mesures, et le seul qui menaçait les conclusions.
- **Valeur** — **10 à 37 µs**, sur **un seul échantillon**, **toujours à
  l'indice 0**, et **toujours dans la série qui suit une ligne de journal**.
  Jamais dans la seconde série, qui ne suit aucune journalisation.
- **Source** — mesure directe. C'est le relevé de l'*indice* du pire
  échantillon, et le fait de mesurer deux fois d'affilée sans rien journaliser
  entre les deux, qui l'ont établi ; sans ces deux précautions le pic passait
  pour de la gigue d'interruption.
- **Confiance** — confirmée pour l'observation. L'interprétation — le code de
  journalisation évince le chemin du lien du cache de flash, et le front
  suivant paie le rechargement — est **probable** : elle est appuyée par le
  fait que le pic ne survient qu'une fois par démarrage dans un binaire de
  92 Ko, et à chaque série dans un binaire de 474 Ko une fois la pile radio
  liée, mais aucune mesure ne l'atteste directement.

## Marge disponible par cadence

- **Ce que c'est** — ce que les chiffres ci-dessus laissent au firmware.
- **Valeur** — rapport de la latence au temps d'un bit :

  | Cadence | Temps par bit | Régime établi, radio | Pire cas mesuré |
  |---|---|---|---|
  | 8192 Hz (Gen 1, confirmé) | 122 µs | 5,5 % | 30 % |
  | 16384 Hz | 61 µs | 11 % | 61 % |
  | 262144 Hz (horloge rapide CGB) | 3,8 µs | **176 %** | — |

- **Source** — les mesures ci-dessus, et la cadence de 8192 Hz sourcée dans
  [`../protocol/link-couche-physique.md`](../protocol/link-couche-physique.md).
- **Confiance** — **conséquence de conception.** Le décodage par interruption
  sur `SCK` tient confortablement à la cadence de la Gen 1 et s'effondre à
  l'horloge rapide du CGB, où le plancher seul dépasse la durée d'un bit.
  **La question ouverte « quelle cadence l'échange Gen 2 emploie-t-il
  réellement » cesse donc d'être documentaire : elle décide de la faisabilité
  du décodage logiciel.**

## Ce que le firmware doit en retenir

- **Ce que c'est** — les règles que ces mesures imposent.
- **Valeur** —
  1. **Aucune journalisation depuis la tâche du lien.** Le coût n'est pas le
     temps d'écriture : c'est le cache évincé, et la facture tombe sur le
     front suivant.
  2. **Épingler tout le chemin chaud en IRAM, pas seulement l'ISR.** L'ISR du
     banc est déjà `#[ram]` et le pic persiste ; ce qui coûte est le reste du
     chemin — aiguillage GPIO du HAL, accès aux registres.
  3. **L'épinglage « lien sur un cœur, Wi-Fi sur l'autre » n'est pas
     nécessaire.** La phase C mesure délibérément le pire cas — les deux sur
     le même cœur — et il passe. C'est une optimisation disponible, plus une
     contrainte d'architecture.
- **Source** — les entrées précédentes.
- **Confiance** — conséquence de conception.

## Une méthode invalide, signalée

La première version du banc comparait une ISR épinglée en IRAM à la même ISR
laissée en flash. **Les deux colonnes sont ressorties identiques à 4 ns près :
ce n'était pas un résultat, c'était un test raté.** Les ~2,2 µs sont dominés
par l'aiguillage d'interruption GPIO d'esp-hal, commun aux deux chemins ;
l'emplacement de la fonction terminale n'y pèse presque rien.

La comparaison a été retirée du banc plutôt que corrigée : la question « faut-il
l'IRAM » a trouvé sa réponse par un autre chemin — la reprise à froid ci-dessus
— et une mesure honnête de l'écart IRAM/flash demanderait d'invalider le cache
de flash entre deux passes, ce que ce banc ne fait pas.
