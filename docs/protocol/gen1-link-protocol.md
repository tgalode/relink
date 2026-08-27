# Protocole d'échange Gen 1

Déroulé de l'échange par câble link entre deux cartouches Pokémon
Rouge/Bleu/Jaune, et valeurs d'octets de chaque phase. Ce document source la
machine à états de l'échange ; la disposition des 415 octets transportés est
documentée à part dans [`gen1-trade-block.md`](gen1-trade-block.md).

**Récapitulatif de confiance** (18 entrées « Confiance » dans ce document) :
**11 confirmées** (≥ 2 sources indépendantes convergentes), **6 probables**
(source unique nommée, ou sources qui se contredisent sur un détail
non déterminant), **1 conflit de sources non tranché** (l'étendue exacte de
la zone couverte par la patch list, voir la section correspondante). Détail
dans chaque section.

Sources principales, recoupées entre elles :

- GBPlay, [« Emulating a Pokemon Trade with Generated Link Cable Data »](https://blog.gbplay.io/2021/05/11/Emulating-a-Pokemon-Trade-with-Generated-Link-Cable-Data.html)
  — description phase par phase du protocole, établie en journalisant le
  trafic du câble link entre deux instances de l'émulateur BGB, puis rejouée
  contre une cartouche réelle. C'est la seule des trois sources à décrire le
  menu du Cable Club dans le détail de ses valeurs.
- nitwhiz, [« Spoofing a Pokémon (Red) Trade »](https://blog.nitwhiz.dev/posts/002-pokemon-red-trade/)
  — capture du trafic série via l'émulateur SameBoy instrumenté, avec un
  dump annoté du trafic entrant et une réimplémentation en Go. Déjà utilisée
  dans ce dossier pour le jeu de caractères et le bloc d'échange.
- kbembedded, [« Flipper-Zero-Game-Boy-Pokemon-Trading »](https://github.com/kbembedded/Flipper-Zero-Game-Boy-Pokemon-Trading)
  — application Flipper Zero qui échange réellement avec une cartouche
  physique. Son fichier `src/views/trade.c` porte en tête une description du
  protocole étape par étape, et le reste du fichier est une machine à états
  qui la met en œuvre : les deux se recoupent l'une l'autre, et l'ensemble
  est vérifié par des échanges réels. Déjà utilisée dans ce dossier.
  Seules les valeurs et le déroulé décrits sont repris ici ; aucun extrait de
  son code n'est recopié dans ce dépôt.

Les trois sources se citent partiellement entre elles — nitwhiz et kbembedded
mentionnent tous deux des travaux antérieurs sur le sujet — mais chacune
repose sur une observation qui lui est propre (journal BGB, capture SameBoy,
matériel réel), et leurs implémentations sont indépendantes. Aucune des trois
n'est un désassemblage.

## Le rôle du module : suiveur, toujours

- **Ce que c'est** — en série synchrone Game Boy, un côté fournit l'horloge
  (leader, « master ») et l'autre la subit (suiveur, « slave »). Le module
  relink se place systématiquement en suiveur : c'est la cartouche qui
  cadence, et le module n'a jamais à décider seul d'émettre.
- **Valeur / disposition** — pas de valeur d'octet ; c'est une contrainte de
  conception, pas une donnée du protocole.
- **Source** — les trois sources font ce choix. kbembedded :
  « This setup always forces the flipper to the follower/slave role in the
  link. This just makes our logic consistent and since we're going to be gobs
  faster than a real Game Boy, we can be guaranteed to always be ready to
  respond. » nitwhiz : « To spoof the trade, we're always sending `0x02` right
  away, and let the Game Boy handle the clocking. » GBPlay : « we will only
  ever allow the connected emulator to operate in master mode […] since real
  hardware will always need to use an external clock signal due to the
  inherent latency of TCP. »
- **Confiance** — confirmée.

## Négociation des rôles

- **Ce que c'est** — les deux cartouches exécutent le même code et doivent
  s'accorder sur qui fournit l'horloge. Chacune commence en suiveur et
  attend une horloge externe ; celle qui n'en voit pas passer bascule en
  leader et émet.
- **Valeur / disposition** — le leader émet `0x01`. Le suiveur répond `0x02`.
  Le délai d'attente avant bascule en leader est d'environ 90 trames, soit
  1,5 seconde.
- **Source** — GBPlay : « it will respond with the value `2` […] then the game
  will switch to master mode and send the value `1` to the other Game Boy […]
  This is done in a loop until the connection is successful or enough failed
  attempts take place. » nitwhiz : « The game waits for 90 frames (1.5 seconds)
  to receive `0x01` or `0x02`. If there is nothing received in these 90 frames,
  the game sends `0x01`. » kbembedded : `#define PKMN_MASTER 0x01`,
  `#define PKMN_SLAVE 0x02`, et une réponse `PKMN_SLAVE` dès réception de
  `PKMN_MASTER`.
- **Confiance** — confirmée pour `0x01` et `0x02` (trois sources). Le délai de
  90 trames est **probable** : source unique (nitwhiz). Il n'a aucune
  incidence sur un module qui reste suiveur — il décrit le comportement de la
  cartouche, pas une obligation du module.

## Acquittement de connexion

- **Ce que c'est** — une fois les rôles fixés, les deux côtés échangent des
  octets neutres puis un octet de connexion, avant d'afficher le menu du
  Cable Club. La cartouche sauvegarde la partie pendant cette phase.
- **Valeur / disposition** — octet neutre `0x00`, octet de connexion `0x60`.
  Le nombre d'octets neutres échangés n'est pas fixé par les sources.
- **Source** — GBPlay : « both instances will send two `0` bytes, save the
  game, exchange a `0x60` byte for synchronization, and then display the
  in-game link type selection menu. » nitwhiz : « After receiving `0x02`, the
  two Game Boys exchange a bunch of `0x00`, followed by some `0x60`. This is
  done to acknowledge the connection. » kbembedded : `#define PKMN_BLANK 0x00`,
  `#define PKMN_CONNECTED 0x60`, et l'étape 3-4 de sa description : « they both
  respond with `PKMN_BLANK(0x00)` for a bit. Next, the leader/master sends
  `CONNECTED(0x60)` bytes that the follower/slave repeats back. Then a bunch
  of BLANK bytes. »
- **Confiance** — confirmée pour les deux valeurs (trois sources). Le nombre
  d'octets `0x00` est **probable, et contredit entre sources** : GBPlay dit
  deux, nitwhiz et kbembedded parlent d'une quantité indéterminée. Conséquence
  de conception : **ne jamais compter ces octets**. Le module renvoie ce qu'il
  reçoit jusqu'au marqueur suivant, comme le fait kbembedded.

## Menu du Cable Club

- **Ce que c'est** — les deux joueurs voient le même menu (Trade Center,
  Colosseum, annuler) et l'un ou l'autre peut choisir. Comme seul le leader
  peut lancer un transfert, il interroge en continu en envoyant l'entrée
  actuellement survolée ; le suiveur répond dans le même format.
- **Valeur / disposition** — survol : `0xD0`, `0xD1`, `0xD2` pour les trois
  entrées, soit la forme `0xDx` dont les bits bas portent l'index survolé.
  Sélection : le bit 2 est mis, ce qui donne `0xD4` (Trade Center), `0xD5`
  (Colosseum), `0xD6` (annuler et rompre le lien).
- **Source** — GBPlay : « the master will continually send values of the form
  `0xDx` (where the bottom 2 bits of `x` store the index of the highlighted
  option) […] Whichever side the selection is made on first will indicate it by
  setting bit 2 of its value: `0xD4` for the Trade Center, `0xD5` for the
  Colosseum, or `0xD6` to cancel. » kbembedded : `ITEM_1_HIGHLIGHTED 0xD0`,
  `ITEM_2_HIGHLIGHTED 0xD1`, `ITEM_3_HIGHLIGHTED 0xD2`, `ITEM_1_SELECTED 0xD4`,
  `ITEM_2_SELECTED 0xD5`, `ITEM_3_SELECTED 0xD6`, avec
  `PKMN_TRADE_CENTRE ITEM_1_SELECTED`, `PKMN_COLOSSEUM ITEM_2_SELECTED`,
  `PKMN_BREAK_LINK ITEM_3_SELECTED`.
- **Confiance** — confirmée (deux sources indépendantes, valeurs identiques).
  nitwhiz ne donne pas les valeurs : son spoofer se contente de renvoyer ce
  qu'il reçoit dans cette phase (« Both Game Boys are now sending which item in
  the Cable Club menu is currently selected over and over again. We just echo
  this data. »), ce qui est cohérent sans les confirmer.

## Annulation par le bouton B

- **Ce que c'est** — une seconde manière de signaler l'annulation dans le menu
  ci-dessus.
- **Valeur / disposition** — bit 3 mis sur la valeur du menu.
- **Source** — GBPlay, seul : « A cancel can also be signaled by setting bit 3,
  which indicates that the B button was pressed on the associated menu
  entry. »
- **Confiance** — probable (source unique). Conséquence de conception : traiter
  toute valeur `0xDx` inattendue comme une rupture du lien plutôt que de
  supposer une valeur précise.

## Entrée sur la table d'échange

- **Ce que c'est** — après le choix du Trade Center, les deux joueurs sont
  téléportés dans la salle mais plus rien ne circule tant que les deux n'ont
  pas interagi avec la table d'échange. Il n'existe aucun moyen de quitter le
  Trade Center autrement qu'en réinitialisant la console.
- **Valeur / disposition** — `0x60`, le même octet que l'acquittement de
  connexion.
- **Source** — GBPlay : « No further transfers occur until both players
  interact with the trade machine, which is signaled by sending the value
  `0x60`. After this, the main transfer can begin. […] Interestingly there's no
  way to cancel or otherwise exit the Trade Center. To leave, the game must be
  reset. » kbembedded décrit la même attente et attend, pour sa part, les
  octets de préambule qui suivent.
- **Confiance** — probable pour la valeur `0x60` dans ce rôle précis (source
  unique explicite : GBPlay). Conséquence de conception : ne pas s'appuyer sur
  cet octet pour changer d'état — c'est l'arrivée du préambule qui marque
  réellement le début du transfert, et les trois sources s'accordent là-dessus.

## Préambule et graine aléatoire

- **Ce que c'est** — le bloc d'échange est précédé d'une suite d'octets de
  préambule qui marque la frontière de section, puis de dix octets aléatoires
  qui synchronisent le générateur pseudo-aléatoire des deux cartouches. Ces
  octets servent aux combats du Colosseum ; le code du lien étant commun aux
  deux parcours, ils sont émis même pour un échange, où ils ne servent à rien.
- **Valeur / disposition** — octet de préambule `0xFD`. Séquence : 10 × `0xFD`,
  puis 10 octets aléatoires, puis 9 × `0xFD`, puis le bloc d'échange.
- **Source** — kbembedded : `#define SERIAL_PREAMBLE_BYTE 0xFD`,
  `SERIAL_RNS_LENGTH 10`, `SERIAL_TRADE_PREAMBLE_LENGTH 9`, et sa description :
  « This starts with 10x `PREAMBLE(0xFD)` bytes, 10x random bytes (to […] sync
  the RNG between two devices, unused at this time) […] I missed another 9x fd
  bytes after rand? State machine below confirms these bytes ». nitwhiz : « The
  leader sends seeds for random number generation. This data starts with 10x
  […] preamble and is terminated with 9x […] preamble bytes », et la légende de
  son dump de trafic nomme `0xFD` comme octet de préambule. GBPlay confirme
  l'existence de la phase sans en donner les valeurs : « Before the actual
  trainer data is sent, some random bytes are exchanged. These are used to
  ensure consistency in link cable battles. »
- **Confiance** — confirmée pour `0xFD`, pour les 10 × `0xFD` et pour les 10
  octets aléatoires (deux sources concordantes, plus GBPlay sur l'existence de
  la phase). Les 9 × `0xFD` finaux sont **probables** : les deux mêmes sources
  les donnent, mais kbembedded note lui-même les avoir manqués dans sa
  première lecture et ne les avoir retrouvés qu'en relisant sa propre machine
  à états. Le corps du texte de nitwhiz écrit par ailleurs « `0xDF` » là où sa
  légende de dump écrit `0xFD` : c'est une inversion de chiffres dans la
  prose, la valeur retenue est `0xFD`.

## Bloc d'échange

- **Ce que c'est** — les 415 octets décrits dans
  [`gen1-trade-block.md`](gen1-trade-block.md) : nom du dresseur, équipe
  complète, noms de dresseur d'origine, surnoms. Les deux côtés les
  transmettent simultanément, un octet sortant pour un octet entrant.
- **Valeur / disposition** — 415 octets, sans en-tête ni longueur ; c'est le
  préambule qui en marque le début et le compte qui en marque la fin.
- **Source** — déjà sourcée dans [`gen1-trade-block.md`](gen1-trade-block.md).
  Pour le déroulé : kbembedded, état `TRADE_DATA`, qui recopie exactement
  `trade_block_sz` octets avant de passer à la suite.
- **Confiance** — confirmée.

## Fin de bloc

- **Ce que c'est** — trois octets de fin après le bloc, puis trois octets de
  préambule.
- **Valeur / disposition** — `DF FE 15`, puis 3 × `0xFD`.
- **Source** — kbembedded, seul : « At the end of this is 3 ending bytes,
  DF FE 15. And, weirdly, 3 PREAMBLE(0xFD) bytes. »
- **Confiance** — probable (source unique, et l'auteur lui-même s'en étonne).
  Conséquence de conception : **ne pas attendre ces valeurs**. La machine à
  états compte six octets de préambule au total entre le bloc et la patch
  list, ce que fait aussi kbembedded, et ignore ce qui se trouve entre eux.

## Pourquoi une patch list : l'octet `0xFE`

- **Ce que c'est** — quand aucun câble n'est branché, le port série d'une Game
  Boy tend à lire `0xFE`. Les jeux s'en servent comme marqueur « pas de
  données ». Un octet `0xFE` légitime au milieu des données d'équipe serait
  donc indistinguable d'une déconnexion : le jeu ne l'envoie jamais tel quel.
- **Valeur / disposition** — `0xFE`.
- **Source** — nitwhiz : « You shouldn't send `0xFE`. If there is no serial
  cable connected, the port tends to read `0xFE`, so many games use this as an
  indicator of an unplugged serial cable. » kbembedded :
  `#define SERIAL_NO_DATA_BYTE 0xFE`, valeur qu'il déclare aussi au pilote de
  lien comme octet « pas de données ».
- **Confiance** — confirmée.

## Patch list : principe

- **Ce que c'est** — pour transporter malgré tout un `0xFE` légitime,
  l'émetteur le remplace par `0xFF` dans les données et note sa position dans
  une liste transmise juste après le bloc. Le récepteur remet `0xFE` aux
  positions listées.
- **Valeur / disposition** — la liste contient les positions **incrémentées de
  un** (une position 0 deviendrait indistinguable du remplissage `0x00`). Elle
  est découpée en deux parties, chacune terminée par `0xFF`, et le reste est
  rempli de `0x00`. Le découpage en deux parties existe parce qu'une position
  ne tient pas sur un octet au-delà de 254 : la première partie couvre le début
  de la zone, la seconde prend le relais avec une base décalée.
- **Source** — nitwhiz : « A Patch List is a list of indexes inside the Trade
  Block data, where `0xFE` is expected. In the received data, these indexes
  will be `0xFF`. Both Patch Lists are terminated with `0xFF` and the data is
  padded with `0x00`. […] All indexes are incremented by 1 when they're put
  into the Patch List. » kbembedded : « To patch outgoing data, if a byte is
  `0xFE`, it is changed to `0xFF`, and the index+1 is added to the patch list.
  There are two parts to the patch list as the data it covers is longer than
  `0xFC`. After each part is complete, `0xFF` is added to the patch list. […]
  After both terminators, it is expected all remaining bytes are `0x00`. »
- **Confiance** — confirmée.

## Patch list : zone couverte

- **Ce que c'est** — sur quelle portion du bloc de 415 octets porte la
  correction.
- **Valeur / disposition** — **les deux sources se contredisent.**
  kbembedded couvre les seules données d'équipe, soit les 264 octets
  (6 × 44 = `0x108`) situés à l'offset 19 du bloc : première partie pour les
  positions `0x00`–`0xFB`, seconde pour `0xFC`–`0x107`. nitwhiz balaie le bloc
  entier et annonce une seconde partie allant jusqu'à `0x19E`, soit 414 —
  l'index du dernier octet du bloc.
- **Source** — kbembedded : « The patch list is specifically for the party data
  of the trade_block. […] The first part of the patch list can patch
  `0x00:0xFB` of the party, the second part can patch `0xFC:0x107`. » Sa
  fonction de construction parcourt bien `party_sz`, qui vaut `44 × 6`, et son
  application écrit dans l'équipe, pas dans le bloc. nitwhiz : « The first
  Patch List contains the indexes `0x00` to `0xFC`, the second Path List
  contains the indexes `0xFD` to `0x19E` », et sa fonction de construction
  parcourt le bloc entier.
- **Confiance** — **conflit non tranché.** L'enjeu est réel et non théorique :
  `0xFE` code le caractère « 8 » dans le jeu de caractères Gen 1 (voir
  [`gen1-charset.md`](gen1-charset.md)), donc un surnom contenant un 8 tombe
  dans la zone de désaccord. Position retenue pour l'implémentation : celle de
  kbembedded, la seule des deux vérifiée contre une cartouche physique, et la
  seule dont le découpage `0x00`–`0xFB` / `0xFC`–`0x107` se justifie
  arithmétiquement par la taille de la zone couverte. À trancher par
  observation dès qu'un montage matériel existe : c'est le premier test à
  faire passer, avec un surnom contenant un 8.

## Patch list : longueur transmise

- **Ce que c'est** — combien d'octets occupe la patch list sur le fil.
- **Valeur / disposition** — 3 × `0xFD` (les trois derniers des six octets de
  préambule qui suivent le bloc), puis 7 × `0x00`, puis 189 octets de liste.
- **Source** — kbembedded : « The patch list starts with 3x more
  `PREAMBLE(0xFD)` bytes for a total of 6x PREAMBLE, followed by 7x BLANK
  bytes. Then remaining 189 bytes of patch list data. » Et, plus loin : « the
  Pokemon code seems to allocate 203 bytes, 3x for the preamble, and then 200
  bytes of patch list. But in practice, the Game Boy seems to transmit 3x
  preamble bytes, 7x `0x00`, then 189 bytes for the patch list. A total of 199
  bytes transmitted. » nitwhiz déclare de son côté un tampon de 190 octets.
- **Confiance** — probable, et les sources ne se recoupent pas exactement (189
  contre 190 octets de données, sans compter les mêmes en-têtes). Conséquence
  de conception : **ne pas s'appuyer sur une longueur exacte**. La lecture
  s'arrête sur les deux terminateurs `0xFF`, et le passage à la phase suivante
  se déclenche sur ce qui arrive ensuite, pas sur un compteur.

## Sélection du Pokémon

- **Ce que c'est** — les deux blocs ayant été échangés, chaque côté connaît
  déjà toute l'équipe adverse. Il ne reste qu'à annoncer quel Pokémon on
  propose. Le leader interroge en boucle avec l'octet neutre tant que rien
  n'est choisi.
- **Valeur / disposition** — octet d'interrogation `0x00`. Sélection :
  `0x60 + index`, l'index allant de 0 à 5, soit `0x60` à `0x65`. Quitter la
  table et revenir dans la salle : `0x6F`.
- **Source** — GBPlay : « the master polls the slave by repeatedly sending `0`
  […] Either player can exit the trade menu by sending `0x6F` which will cause
  the players to go back to the Trade Center. To trade, each side first
  indicates the selected Pokemon by sending its index number in the party
  Pokemon list (ranging from `0x60` to `0x65`). » nitwhiz : « the Game Boy
  sends `0x60` + <pokemon party index> over the wire. Right after that, a
  `0x00` follows. » kbembedded : `PKMN_SEL_NUM_MASK_GEN_I 0x60`,
  `PKMN_TABLE_LEAVE_GEN_I 0x6f`, et un état d'attente qui ne bouge pas tant
  qu'il ne reçoit que des `0x00`.
- **Confiance** — confirmée (trois sources).

## Verdict

- **Ce que c'est** — une fois les deux Pokémon annoncés, le jeu affiche la
  confirmation. Chaque côté accepte ou refuse. Sur un refus, on retourne à la
  sélection ; sur un accord des deux, l'échange a lieu.
- **Valeur / disposition** — refus `0x61`, accord `0x62`.
- **Source** — GBPlay : « If a player cancels the trade at this point, `0x61`
  is sent and the selection process is repeated. […] If both players accept,
  `0x62` is exchanged as a confirmation and the trade takes place. » nitwhiz :
  « If it's rejected, the Game Boy sends `0x61` and the player can select a
  Pokémon again. If it's accepted, the Game Boy sends `0x62` and the trade
  starts. » kbembedded : `PKMN_TRADE_ACCEPT_GEN_I 0x62`,
  `PKMN_TRADE_REJECT_GEN_I 0x61`.
- **Confiance** — confirmée (trois sources).

## L'ambiguïté de `0x61`

- **Ce que c'est** — `0x61` vaut « je propose le Pokémon d'index 1 » en phase
  de sélection et « je refuse » en phase de verdict. Rien dans l'octet ne les
  distingue.
- **Valeur / disposition** — la phase courante tranche, et elle seule.
- **Source** — kbembedded sépare explicitement les deux états
  (`TRADE_PENDING` teste l'appartenance à la plage de sélection, puis
  `TRADE_CONFIRMATION` teste le refus et l'accord), ce qui rend la distinction
  observable dans une implémentation qui fonctionne sur matériel réel.
- **Confiance** — confirmée par construction. C'est une contrainte de
  conception à part entière : une machine à états qui déciderait sur la seule
  valeur de l'octet serait fausse.

## Après l'échange

- **Ce que c'est** — l'échange effectué, chaque cartouche a déjà en mémoire
  tout ce qu'il lui faut : elle recopie les données reçues et joue l'animation
  d'échange, purement décorative. La partie est sauvegardée. Puis les deux
  côtés reviennent à la table et **réémettent préambule, graine, bloc et patch
  list** avec les équipes mises à jour, pour un échange suivant.
- **Valeur / disposition** — pas de valeur propre : reprise du déroulé au
  préambule.
- **Source** — GBPlay : « they already have everything they need in order to
  trade and can copy the relevant memory right away. No further transfers
  occur and the long-winded trade animation is played on both devices purely
  for show. […] both players are taken back to the trade menu […] and can trade
  more Pokemon if they wish. » nitwhiz : « After that, the random seed, Trading
  Blocks and Patch Lists are exchanged again, and another trade can be done. »
  kbembedded : « The Flipper actually goes back to step 7 […] After the trade
  is complete on the Game Boy, it re-sends the trade_block data. This re-syncs
  the states between the Flipper and Game Boy and another trade can occur. »
- **Confiance** — confirmée (trois sources).

## Constantes Gen 2 : notées, non implémentées

- **Ce que c'est** — le déroulé Gen 2 (Or/Argent/Cristal) est le même ; seules
  quelques valeurs changent, et une phase de courrier s'ajoute après la patch
  list. Ces valeurs sont consignées ici parce qu'elles ont été rencontrées au
  cours de ce sourçage, pas parce qu'elles sont utilisées : le codec du bloc
  Gen 2 n'existe pas, donc rien ne les exerce.
- **Valeur / disposition** — connexion `0x61` ; accord `0x72` ; refus `0x71` ;
  quitter la table `0x7F` ; base de sélection `0x70`. Phase de courrier après
  la patch list : 6 octets `0x20` de préambule, 198 octets (33 × 6) de
  courrier, 84 octets (14 × 6) de nom et identifiant du dresseur d'origine, un
  `0xFF`, puis 100 octets à zéro — 389 au total.
- **Source** — kbembedded, seul : `PKMN_CONNECTED_II 0x61`,
  `PKMN_TRADE_ACCEPT_GEN_II 0x72`, `PKMN_TRADE_REJECT_GEN_II 0x71`,
  `PKMN_TABLE_LEAVE_GEN_II 0x7f`, `PKMN_SEL_NUM_MASK_GEN_II 0x70`, et pour le
  courrier : « Preambled with 6x `0x20` bytes; 33*6 == 198 bytes of Mail, for
  each pokemon, even if they have no mail set; 14*6 == 84 bytes, for each
  pokemon's mail, the OT Name and ID; a `0xff`; 100 zero bytes (unsure if they
  are always 0). This is 6 + 198 + 84 + 1 + 100 == 389. » L'auteur signale par
  ailleurs en tête de fichier que sa documentation Gen 2 est moins mûre que
  celle de la Gen 1.
- **Confiance** — probable (source unique, et incertitude déclarée par
  l'auteur sur les 100 octets finaux). À recouper avant tout usage.

## Rythme des octets

- **Ce que c'est** — le délai que respecte une implémentation réelle entre deux
  octets côté suiveur.
- **Valeur / disposition** — 15 microsecondes chez kbembedded.
- **Source** — kbembedded, `#define DELAY_MICROSECONDS 15`, valeur employée
  par une application qui échange avec du matériel réel.
- **Confiance** — probable (source unique). Sans effet sur ce lot : le crate
  `protocol` ne connaît pas le temps. C'est une contrainte pour le futur lot
  firmware, consignée ici pour ne pas avoir à la resourcer.

## Ce que ce document ne dit pas

- **Le Colosseum.** Les combats ne sont pas dans le périmètre du projet ; seule
  la valeur qui les sélectionne dans le menu est documentée, pour pouvoir les
  refuser proprement.
- **Le contenu des dix octets aléatoires.** Ils synchronisent le générateur
  pseudo-aléatoire des combats. Pour un échange, ils sont sans effet : on les
  consomme sans les lire.
- **Le format du bloc Gen 2.** Hors de ce lot, et toujours ouvert — voir
  [`README.md`](README.md).
