> **BASELINE GELÉE POUR DÉVELOPPEMENT — V9.1-STABLE (30 octobre 2026)**  
> Ce document est la référence normative du projet. Les sections 0–21 et annexes A–B restent la base fonctionnelle, **à l’exception du seul amendement explicite relatif aux migrations SQLite**, documenté en §13.7 et répercuté aux références correspondantes. **L'annexe C précise les choix d'implémentation, la première tranche jouable et les règles de gel ; elle ne réduit pas les autres exigences du produit.**  
> Statut : approuvée pour **démarrer le développement**, pas « jeu développé » ou « gameplay validé ». Les seules modifications ultérieures admises sans nouveau jalon sont les corrections d’ambiguïté, de sécurité et de bug documentées.  
> **Amendement ciblé — 30 octobre 2026 :** sur décision explicite du porteur du projet, les anciennes bases SQLite ne sont **jamais migrées**. Une base dont le schéma est incompatible avec celui attendu par l’application est refusée sans modification. Cette décision remplace les anciennes exigences de migration ; les autres invariants de sécurité, de persistance et de rejeu restent inchangés.  

# Spécification V9.1 — Maître du Jeu IA universel, vivant et durable en Rust

**Statut :** spécification normative V9.1 — 30 octobre 2026 (consolide V9 et remplace V8)  
**Produit :** application locale de jeu de rôle narratif, en français, 1 à 6 joueurs sur un même poste  
**Philosophie :** *liberté d’action des joueurs ; MJ vivant et non dirigiste ; Rust arbitre et valide ; SQLite maintient la continuité*  
**Règles :** **D&D 2024 Lite**, adaptation volontairement simplifiée du SRD 5.2.1 ; **pas** une implémentation exhaustive ou officiellement conforme à toutes les règles de D&D 2024  
**IA :** `llama.cpp` et fournisseurs de chat OpenAI-compatibles (dont Albert API), outils JSON Schema  
**Mémoire et paramètres persistants :** exclusivement **SQLite**  
**Format de contexte LLM :** **JSON compact**, sans DCP/1, YAML ni TOON  
**Nature du document :** exigences produit, comportements mesurables et contrats techniques, **sans code source Rust**. Objectif : campagnes de centaines d’heures sans dérive de mémoire, de rythme ni de règles.

> **Priorité V9.1.** Le MJ n’est pas un moteur à quêtes ni une machine à combats. Il écoute, improvise, prend des initiatives plausibles quand la fiction le permet et laisse les joueurs déterminer ce qui est important. Une absence de combat, de rebondissement ou de quête est une situation parfaitement valide.
>
> **Invariance fondamentale.** Le joueur n'est jamais limité à une liste d'actions. Le LLM ne lance pas de dés, n'invente aucun résultat mécanique et n'écrit jamais directement l'état canonique. Il peut interpréter toutes les intentions et proposer des conséquences nouvelles ; Rust vérifie les invariants, exécute les primitives et persiste les changements.

## Note de consolidation V9 → V9.1

Cette version conserve tous les mécanismes normatifs de la V9 et corrige ses points faibles **sans ajout d’agent, de protocole ni de fichier de configuration** :

1. **Gameplay :** progression non dirigiste, liberté de refuser les intrigues, scènes calmes, mécanismes sociaux et exploration formalisés ; critères d’agrément évalués par des joueurs, distincts des seuls tests de validité.
2. **Narration :** frontière vérifiable entre décor descriptif, déclarations de personnages et nouveaux faits persistants ; pas de « canon inventé dans la prose » ni de refus rétroactif d’un élément présenté au joueur.
3. **UARE :** contrat d’arbitrage plus précis, coût et échecs bornés, demandes de consentement, règles Lite minimales et traitement des situations non couvertes.
4. **Mémoire :** récupération temporelle/causale, visibilité stricte par destinataire, prévention des résumés trompeurs et diagnostic de rappel ; budget de contexte minimal.
5. **Robustesse :** rejets sans effet, sorties LLM incohérentes, snapshots vérifiés, suite d’outils idempotente, tests d’événements et reprise sans double jet.

**Convention :** les sections ajoutées en V9.1 complètent les contrats de base ; en cas de contradiction, le contrat le plus strict en matière d'autorisation, de causalité ou de persistance prévaut. La conformité à un schéma JSON ne remplace jamais la validation métier.


## 0. Vocabulaire et décisions de conception

- **DOIT** = exigence bloquante ; **DEVRAIT** = exigence recommandée avec justification d'une éventuelle dérogation ; **PEUT** = option.
- **UARE** (*Universal Action Resolution Engine*) : moteur universel de résolution d'actions, utilisé en exploration, dialogue, enquête, magie et combat.
- **MJ IA** : modèle linguistique qui improvise, incarne les PNJ, juge la faisabilité fictionnelle et formule des propositions contrôlées.
- **État canonique** : données validées en base (monde, personnage, inventaire, faits et événements), autorité supérieure aux résumés et aux sorties du modèle.
- **Faits verrouillés** : éléments initiaux imposés par l'utilisateur ; ils ne sont ni contredits silencieusement ni immunisés contre tous les événements futurs. Une évolution exige une cause valide, ou une édition explicite.
- **Épisode** : unité narrative (scène, voyage, interaction), distincte d'un tour de combat. **Tour utilisateur** : prise de parole ou action. **Étape validée** : modification transactionnelle durable.
- **Résolution simplifiée** : les règles de base sont mécanisées ; les actions créatives inédites sont arbitragées sans exiger un résolveur spécifique à chacune.
- Le système reste **principalement narratif**. Les cartes, zones et éléments tactiques sont facultatifs, invoqués seulement si utiles à une situation.
- Le moteur n'imite pas les mécaniques exactes de *Baldur's Gate 3* : l'inspiration est la fluidité des règles et la liberté d'approche, pas le gameplay centré sur les combats.
- **Une seule édition de règles dans l'application : D&D 2024 Lite.** Aucune compatibilité rétroactive 2014, aucun moteur multi-règles à concevoir.

## 1. Objectifs et expérience joueur

### 1.1 Résultat attendu

L'application DOIT permettre à 1–6 joueurs locaux de créer ou reprendre une campagne dans un monde défini totalement, partiellement ou pas du tout par les joueurs. Chacun décrit librement ses actions en français : négocier, mentir, enquêter, élaborer un piège, utiliser une compétence ou un sort, explorer, fuir, combattre ou essayer une idée inattendue. Le MJ IA réagit de manière cohérente, propose des arbitrages et fait vivre les PNJ et factions. Les informations fiables survivent à toute interruption et à des centaines de tours.

### 1.2 Fonctions visibles

| Fonction | Exigence |
|---|---|
| Campagnes | Créer, nommer, reprendre, dupliquer sur une nouvelle branche, archiver, exporter/importer, supprimer après confirmation |
| Participants | Choisir **1 à 6 joueurs** à l'initialisation ; attribuer chaque PJ à un joueur ; autoriser changement avec confirmation |
| Création de monde | Modes libre, partiel et complet ; prompts utilisateurs persistés ; propositions IA prévisualisées puis validées |
| Personnages | Création guidée ; caractéristiques, PV, équipement, ressources, progression, pouvoirs, conditions et historique visuels |
| Jouer | Texte libre par défaut ; dialogue naturel et narration ; aucune obligation d'employer des commandes prédéfinies |
| Dés | Jets affichés et audités ; mode automatique ou validation joueur selon la situation ; jamais de résultat inventé |
| Monde | Lieux, quêtes, factions, PNJ, relations, réputation, calendrier, objets et événements différés |
| Mémoire | Recherche de faits, journal narratif, connaissances des PNJ, résumés et diagnostic de contexte |
| IA | Profils `llama.cpp` / API compatibles, sélection de modèles, test de connexion, capacités, coûts/latences observés |
| Configuration | Tous les paramètres persistants modifiables dans l'UI et stockés en SQLite, **aucun fichier externe de configuration** |
| Reprise | Autosauvegarde par micro-étape ; sauvegardes nommées et branches ; restauration sans redéclencher les jets |
| Hors connexion IA | Lire, sauvegarder, exporter, consulter fiches et effectuer des jets déterministes ; mise en attente des décisions du MJ |

### 1.3 Les actions libres sont un droit de premier rang

- L'interface DOIT accepter une intention non répertoriée, même très complexe, sans la convertir artificiellement en un bouton préprogrammé.
- L'application ne DOIT pas forcer un jet si le résultat est évident, impossible ou sans enjeu pertinent.
- Un MJ peut dire *« impossible actuellement »* s'il existe une raison fictionnelle précise, mais non *« action non implémentée »* pour une action narrativement plausible.
- Les actions composées sont décomposées en étapes **seulement si** un risque, un choix ou une réaction rend la décomposition significative.
- Le jeu ne DOIT ni pousser systématiquement au combat, ni forcer des quêtes, ni terminer chaque intervention par une liste d'options.
- Aucun joueur ne perd le contrôle de ses paroles, décisions ou actes : le modèle ne peut pas les inventer.

### 1.4 Contrat d'expérience de jeu — non négociable

1. **Agence** : la direction de la campagne provient des actes et refus des joueurs ; aucune intrigue obligatoire, aucun choix « faux » qui reconduit à une scène prédéfinie.
2. **Initiative crédible du MJ** : le monde évolue via des personnages qui ont des objectifs et du temps, pas via des événements arbitraires destinés à « animer » la partie.
3. **Rythme variable** : scènes calmes, conversations longues, voyage, enquête, exploration, dangers et célébrations restent possibles sans rencontre hostile.
4. **Conséquences durables** : les succès, échecs, promesses, blessures, connaissances et transformations sont persistants et peuvent redevenir pertinents très longtemps après.
5. **Temps de réponse raisonnable** : une action simple n'entraîne pas plusieurs requêtes IA coûteuses ; l'application montre les états « génération », « arbitrage » et « attente du joueur » sans inventer une réponse.
6. **Pas de punition automatique** : sortir du scénario, ignorer un indice ou se reposer n'entraîne ni combat artificiel ni catastrophe automatique. Les conséquences doivent avoir une cause établie.
7. **Contrôle utilisateur** : préférences de ton, d'autonomie du MJ, de longueur de prose, de danger et de fréquence des événements réglables par campagne ; joueur libre de demander pause, clarification ou récapitulatif.
8. **Narration fidèle** : aucune pensée, parole ou action du PJ non déclarée par son joueur ; le MJ ne transforme jamais une hypothèse en fait canonique.

Le moteur DOIT préserver ces huit critères lors de toutes les évolutions, y compris celles qui optimisent la latence et les tokens.

### 1.5 Qualité de l'expérience — engagements observables

- **Le joueur initie ; le monde répond.** Une action plausible et nouvelle doit recevoir un arbitrage explicable, non être reconduite vers une liste de commandes.
- **Le temps de parole reste maîtrisé.** Le MJ ne monologue pas si un joueur attend un choix ; il ne pose pas une question gratuite à chaque réponse. Le réglage de longueur demeure une préférence, jamais une coupure de faits essentiels.
- **Le refus est une branche valable.** Accepter une quête, l'abandonner, contourner le conflit ou quitter la région sont quatre trajectoires légitimes ; aucune ne déclenche une punition automatique.
- **Les joueurs ne sont pas des spectateurs.** Les PNJ peuvent agir et apprendre, mais ne résolvent pas hors champ à la place des PJ un enjeu que les joueurs sont en train de traiter, sauf causalité et échéance établies.
- **Informations sans spoiler.** L'UI expose ce que les personnages peuvent constater, et une vue diagnostic volontairement séparée peut examiner le canon MJ. Un joueur ne lit jamais un champ de connaissance secrète simplement parce qu'il figure dans SQLite.
- **Pas de résultat garanti par la prose.** La beauté du texte ne compense ni erreurs d'état, ni décisions imposées, ni retcons silencieux.

**Mesures :** sondage de plaisir de jeu (échelle 1–5), sentiment d'agence, latence perçue, taux d'actions inédites arbitrées sans blocage, contradictions signalées et rejets justifiés. Ces mesures ne peuvent être « validées » par un schéma ; elles nécessitent des séances jouées.


## 2. Architecture de référence

```text
Interface Rust (vues, texte libre, confirmations)
                   |
          Orchestrateur de scènes
                   |
         UARE — actions universelles
          /        |        \
   État/monde   Dés/règles   MJ IA (LLM)
          \        |        /
         Contrôle des conséquences
                   |
       Journal + projections SQLite
                   |
      Contexte filtré -> Narration
```

| Module logique | Responsabilité exclusive |
|---|---|
| `ui` | Entrées utilisateurs, affichage, vues des PJ, choix et confirmations |
| `orchestrator` | Épisodes, tours de parole, tâches pendantes, reprise, choix du circuit de résolution et coordination |
| `uare` | Cycle d'une intention, arbitrage, enjeux, résolution et composition d'effets |
| `rules_lite` | Mécaniques D&D simplifiées, bornes et validations des jets/ressources |
| `dice` | Aléatoire uniforme, journaux exacts, avantages, relances, calculs |
| `world` | Temps, lieux, PNJ, factions, objets, quêtes, intentions et échéances ; initiatives sous préconditions |
| `characters` | Fiches, compétences, progression, inventaires et pouvoirs |
| `effects` | Primitives typées de modification d'état et leurs préconditions |
| `llm_gateway` | Chat OpenAI-compatible, outils, JSON Schema, streaming, reprises |
| `memory` | Faits, assertions, croyances, profils de PNJ, recherche, résumés et accès différenciés |
| `context_builder` | Sélection ciblée, JSON compact, budgets par circuit, confidentialité et comptage de tokens |
| `storage` | SQLite, transactions, événements, instantanés, branches, sauvegardes |
| `security` | Permissions des PJ, secrets chiffrés, filtrage, traces et audits |
| `narrative_director` | Tensions, fils narratifs, rythme, initiatives PNJ, moments de calme et propositions de scène **sans imposer une intrigue** |
| `llm_router` | Route directe/standard/approfondie, limitation des appels, délais, budget et métriques |

**Frontière d'autorité :** tout appel LLM est une consultation ou une **proposition**. Seuls les services Rust, via commandes authentifiées et transactions SQLite, peuvent produire des événements canoniques. Le LLM ne reçoit ni connexion SQLite, ni SQL brut, ni outil de modification arbitraire.

## 3. Moteur universel de résolution d'actions (UARE)

### 3.1 Objet métier « intention »

Une intention conserve `intent_id`, `campaign_id`, `branch_id`, `scene_id`, `player_id`, `actor_id`, `text_original`, `created_at`, `state_version`, `visibility` et `status`. Rust assigne tous les identifiants et permissions ; le texte original est préservé intégralement. Une interprétation IA dérivée n'écrase jamais l'original.

Le modèle propose une lecture structurée : `objectif`, `méthode`, `cibles`, `préconditions`, `incertitudes`, `risques`, `portée_temporelle`, `besoin_d'informations`. Toute cible est résolue contre des identifiants existants ou devient une demande de clarification.

### 3.2 Automate de résolution

```text
RECEIVED -> GROUNDED -> ADJUDICATED
                        |            \
                        |             -> WAIT_PLAYER -> ADJUDICATED
                        v
                   PENDING_ROLL -> ROLL_RECORDED
                        |               |
                        +---------------+
                                v
                         EFFECTS_PROPOSED
                                |
                       VERIFIED -> COMMITTED
                          |           |
                          v           v
                       REJECTED     NARRATED -> DONE
```

Chaque transition durable DOIT être journalisée ; les suspensions sont enregistrées et peuvent survivre à un crash. `REJECTED` peut déboucher sur un nouvel arbitrage **sans relancer** les dés déjà effectués. Un refus définitif conserve l'intention et son motif.

### 3.3 Arbitrages permis

| Décision | Condition | Conséquence |
|---|---|---|
| `automatic_success` | Action faisable et issue évidente ou sans enjeu | Rust applique les effets plausibles validés, sans dé |
| `automatic_failure` | Issue certaine malgré l'intention, sans hasard utile | Refus/conséquences justifiés ; pas de jet fictif |
| `check` | Incertitude et échec intéressant | Le MJ propose compétence/caractéristique, difficulté, enjeux ; Rust valide et lance |
| `contest` | Deux agents opposés ; règles simplifiées le permettent | Rust lance et compare des jets cadrés |
| `clarify` | Ambiguïté bloquante | Question ciblée au joueur, sans mutation |
| `needs_consent` | Dépense, risque ou action sur un PJ nécessitant accord | Validation explicite du joueur concerné |
| `impossible_now` | Obstacle fictionnel avéré | Le MJ explique l'obstacle et peut suggérer des pistes, sans fermer l'action pour toujours |

**Arbitrage par le MJ :** la difficulté est une proposition, non un résultat. Rust vérifie le domaine autorisé, les bonus, la cible, les ressources et les conflits. L'IA DOIT proposer les enjeux *avant* le dé ; elle ne peut pas changer après coup le DD ou transformer un échec normal en succès entier sans nouvelle mécanique traçable.

**Règle de proportionnalité :** un jet ne peut pas donner davantage que ce que le personnage tentait raisonnablement d'obtenir. Les effets massifs (destruction d'un château, contrôle d'une ville, persuasion absolue d'un PNJ) nécessitent une chaîne causale et des préconditions crédibles, pas un unique d20 arbitraire.

### 3.4 Les trois plans d'un résultat

1. **Mécanique :** réussite/échec, coût, dégâts, ressources, état — déterminé par Rust selon les règles.
2. **Fiction :** comment cela se manifeste, ce que les témoins perçoivent et les réactions plausibles — proposé par le MJ.
3. **Mémoire :** ce qui devient vrai, connu, cru, secret ou prévu — enregistré dans des objets distincts.

Une réussite n'entraîne pas automatiquement toutes les conséquences souhaitées. Une parole de PNJ n'est jamais automatiquement considérée comme vraie. Un effet temporaire est conservé avec une condition de fin explicite.

### 3.5 Actions à étapes et choix successifs

Exemple : « Je prépare une fausse apparition divine, attire le chef gobelin dans la chapelle, puis tente de le convaincre de se rendre. »

- Étape 1 : examiner les moyens disponibles, l'illusion et l'environnement.
- Étape 2 : préparer l'illusion ; consommation éventuelle d'une ressource.
- Étape 3 : tentative sociale contre la réaction et les croyances du gobelin ; test si pertinent.
- Étape 4 : conséquence et mise à jour des connaissances, de l'attitude et des plans du gobelin.

Chaque étape significative est persistée avant la suivante ; Rust n'accepte pas qu'une unique proposition du modèle confirme rétroactivement toute la chaîne.

### 3.6 Limites de l'improvisation

Le LLM PEUT proposer un nouvel objet, secret, événement, plan, lieu, allié ou effet fictionnel. Il **ne** PEUT **pas** improviser une nouvelle caractéristique mécanique qui contourne les règles, dépenser une ressource inexistante, forcer un choix de PJ, résoudre une action sur un fait contradictoire ou inventer une nouvelle vérité rétroactive. Si une mécanique inconnue est indispensable, Rust ouvre `NEEDS_ADJUDICATION` et peut appliquer une règle générique validée, plutôt que refuser tout le scénario.

### 3.7 Résultats nuancés et enjeux préalables

L'UARE DOIT permettre **réussite complète**, **réussite avec coût consenti**, **échec avec progression partielle** et **échec avec complication causale**, sans contredire un dé ou réécrire un enjeu après le tirage.

- L’arbitrage `adjudication` peut proposer `stakes.success`, `stakes.failure`, `stakes.success_with_cost`, `stakes.failure_with_progress`, `stakes.failure_with_complication` et `stakes.near_miss_window` (entier 1–3). Ces champs sont des issues **préalables**, persistées avant tout lancer.
- Si `near_miss_window` est défini, un test raté de 1 à cette marge peut produire `failure_with_progress`, seulement si l'effet annoncé était possible. Cela **reste un échec mécanique**, pas une réussite inventée.
- `success_with_cost` n'est activable que si le joueur consent au coût annoncé avant de le subir ; sinon on applique le succès normal ou on suspend pour clarification. Un coût n'est jamais introduit rétroactivement.
- La difficulté, les bonus, l'avantage et les enjeux sont figés dans `adjudication_committed` avant `dice_roll_recorded` ; la proposition `outcome` ne peut pas les modifier. Les concessions nouvelles doivent être de nouveaux choix, non des corrections cachées.
- En cas de problème mineur sans enjeu, **pas de jet**. Ne pas forcer une complication pour « rendre la scène intéressante ».
- La préférence `graduated_outcomes_enabled` est persistée dans SQLite. Par défaut `true` ; les situations avec règles explicites de dommage, combat ou sauvegarde n'adoptent pas de réussite partielle automatique : il faut un arbitrage spécifiquement applicable.

**Scénario test** : le PJ tente de convaincre un archiviste d’ouvrir ses registres, DD 14. Les enjeux sont enregistrés avant le d20. Avec un total de 12 et une fenêtre de 2, il n'obtient pas l’accès complet, mais l’archiviste lui donne un nom pertinent, si ce résultat était préannoncé. Un total de 3 n'active pas cette progression.

### 3.8 Arbitrage universel borné — ordre normatif

La proposition du MJ doit être **courte et vérifiable** : (a) intention comprise, (b) éléments factuels utilisés, (c) possibilité et incertitude, (d) décision et éventuel jet, (e) enjeux **avant le jet**, (f) conséquences proposées après résolution. La justification est limitée aux références utiles ; elle ne reproduit pas l'histoire de la campagne.

1. Rust fige `state_version`, acteur, ressources disponibles et cibles. Le MJ n'obtient jamais un identifiant fictif implicitement accepté.
2. L'arbitrage choisit `automatic_success`, `automatic_failure`, `check`, `contest`, `clarify`, `needs_consent` ou `impossible_now`. Une action plausible ne peut être refusée *uniquement* parce que le moteur ne possède pas un verbe spécialisé.
3. Un test est autorisé **seulement** si son échec et son succès sont tous deux plausibles et si l'enjeu le justifie. Le DD est fixé **avant** le tirage et reste enregistré.
4. **Consentement explicite** avant : consommation volontaire d'une ressource rare, coût négocié, sacrifice d'un bien de PJ, déplacement irréversible consenti, violence/acte attribué au PJ, divulgation d'un secret personnel. Une ressource engagée volontairement par l'action explicite du joueur n'exige pas une deuxième demande superflue.
5. Si un effet proposé dépasse les capacités existantes ou crée une règle nouvelle, il est remplacé par un effet générique borné et annoncé, ou bien différé à un arbitrage utilisateur ; jamais maquillé en règle officielle.
6. Toute demande de clarification doit viser **une seule ambiguïté bloquante**. Une réponse intuitive n'exige pas une longue négociation avec le MJ.

**Test :** « Je frappe le verrou avec mon épée, puis je tente de persuader le geôlier en montrant les marques. » La première étape ne peut être décrétée réussie par le LLM sans résultat ; la seconde utilise le nouvel état après commit et peut être résolue sans jet si les preuves suffisent.

### 3.9 Contrat de conséquences et d'engagement

- `success`/`failure` et variantes graduées constituent des **conséquences candidates** liées à une seule résolution. Elles n'engendrent ni décès certain ni domination mentale ni création de récompense disproportionnée sur un simple succès social.
- Un coût « succès avec coût » nécessite un **accord préalable**, l'identifiant de son payeur et une vérification de ressources. Si l'accord n'a pas été donné, Rust retourne `pending`, non une narration de coût consommé.
- Un échec ne peut pas faire disparaître un indice indispensable de façon à rendre la campagne injouable. Des alternatives **causales** doivent exister, sans téléporter magiquement le même indice.
- L'annulation d'une action en attente n'efface ni les phases déjà validées ni les dés déjà tirés. Elle met fin aux seules phases encore non engagées.
- Les décisions de PNJ sont des **tentatives**, sujettes aux mêmes contraintes que celles des PJ ; hors champ n'est pas synonyme de réussite automatique.


## 4. Primitives d'effets Rust

Le moteur ne contient **pas** de commande publique « liste exhaustive des actions ». Il comprend des primitives internes sûres, combinables, avec validation de préconditions et événements de résultat.

| Primitive interne | Fonction et contraintes |
|---|---|
| `check` / `contest` | Demande de test, caractéristique, maîtrise, avantage/désavantage, DD, opposant |
| `roll` | Évaluation d'une expression de dés bornée et journalisée |
| `damage` / `heal` | Dégâts/soin, type, cible, résistances simplifiées, plafonds |
| `condition_add` / `condition_end` | Effet, origine, durée ou fin, cumul et nettoyage |
| `resource_spend` / `resource_gain` | PV, charges, emplacements, inspiration, argent, consommables |
| `item_create` / `item_transfer` / `item_destroy` | Instances uniques, propriétaires, quantités, inventaires, provenance |
| `actor_move` | Lieu, sous-lieu ou position connue, validation spatiale minimale |
| `world_fact_set` / `world_fact_end` | Faits à provenance et validité causale |
| `relation_change` / `belief_change` | Liens et croyances de PNJ, distincts de la vérité objective |
| `quest_change` | Objectif, état, échéance, motivation et journal des changements |
| `time_advance` | Horloge du monde, échéances, interruption possible |
| `schedule_event` / `cancel_event` | Effets différés soumis à préconditions, visibilité et provenance |
| `npc_plan_set` | Intention/autonomie des PNJ, non résultat certain |
| `choice_request` | Suspension et collecte d'une décision de joueur autorisé |

**Règles d'exécution :** les primitives sont déclenchées uniquement par le résolveur Rust après validation de portée, droits, source, version d'état, causalité et ressources. Le texte brut d'une proposition ne devient jamais un script exécutable. Si un effet ne peut pas être traduit sans ambiguïté, le résultat reste en attente.

**Ordre :** vérifier les préconditions → réserver la commande → produire les jets nécessaires → vérifier effets → écrire événements, projections et résultat transactionnellement → produire la narration après commit. Les contraintes d'effet qui exigent un choix futur créent une résolution suspendue ; aucun verrou SQL n'attend le joueur ou l'IA.

## 5. D&D 2024 Lite — socle mécanique simplifié

### 5.1 Objectif et limites

Les mécanismes retenus proviennent de D&D 2024 / SRD 5.2.1 lorsque c'est approprié, mais constituent **notre adaptation**. Il est interdit de la présenter comme une implémentation intégrale des règles officielles. Les écarts doivent être visibles dans une page « Règles de la campagne » intégrée à SQLite et consultable en jeu.

### 5.2 Règles générales

| Domaine | Règle V9 |
|---|---|
| Caractéristiques | Force, Dextérité, Constitution, Intelligence, Sagesse, Charisme |
| Modificateurs | `floor((score−10)/2)` ; maîtrise liée au niveau ; expertise lorsqu'une capacité le prévoit |
| Tests | d20 + modificateur pertinent + maîtrise le cas échéant, comparaison à un DD fixé avant le jet |
| DD indicatifs | 5 très facile, 10 facile, 15 moyen, 20 difficile, 25 très difficile, 30 exceptionnel ; ce sont des repères d'arbitrage, pas un barème automatique universel |
| Avantage | 2d20, conserver le plus haut ; désavantage : conserver le plus bas ; annulation si les deux |
| 1 et 20 naturels | Critique automatique sur les **jets d'attaque** selon les règles retenues ; pas d'échec/réussite universels sur tous les tests |
| Compétences | Liste D&D de compétences pertinentes, associées normalement à une caractéristique, avec arbitrage d'association possible |
| Sauvegardes | d20 + modificateur + maîtrise le cas échéant contre DD fixé |
| Ressources | PV, PV temporaires facultatifs, repos, inspiration, ressources de classe et sorts sélectionnés |
| Économie hors combat | Aucune action/bonus imposée artificiellement ; durée et conséquence selon la fiction |
| Déplacement hors combat | Lieux et distances approximatives selon les informations disponibles ; pas de grille obligatoire |
| Inventaire | Instances persistantes, objets clés et consommables suivis ; pas d'encombrement détaillé par défaut |
| Magie | Sorts connus/préparés du catalogue implémenté, emplacements et durée ; composantes ordinaires simplifiées |
| Échecs intéressants | Complication possible si elle a été cadrée, sans effacer un succès ou falsifier un jet |

**Aucune limite arbitraire à la créativité.** Une interaction imaginative avec un pouvoir peut être évaluée par un test ou un effet générique lorsque la fiction le permet, sans créer gratuitement un nouveau sort officiel. Les descriptions libres des objets restent indépendantes de leurs attributs mécaniques.

### 5.3 Création et progression des personnages

- La fiche comprend caractéristiques, compétences, maîtrises, classe/archétype narratif, niveau, PV, CA, équipement, dons/pouvoirs disponibles et ressources.
- **Catalogue initial maîtrisé** : classes et capacités réellement implémentées clairement marquées dans l'UI, avec une progression jouable. La spécification n'impose ni six classes fixes, ni une fausse promesse de toutes les capacités 1–20. Une capacité absente ne sera jamais simulée comme une règle officielle.
- Les personnages peuvent être d'origine libre du point de vue narratif (espèce, histoire, culture) avec adaptation mécanique proposée et validée dans le socle Lite.
- La montée de niveau est une opération canonique : choix du joueur, validation des gains, événements, recalcul des valeurs dérivées.
- Aucun PJ ne reçoit de nouvelle capacité ou ne dépense de ressource sur une initiative implicite du MJ.

### 5.4 Magie et objets exceptionnels

- Bibliothèque de sorts **déclaratifs** fondés sur les primitives d'effets ; chaque entrée indique coûts, cibles, jet/attaque, durée, concentration éventuelle et effet.
- La concentration est unique par lanceur ; une interruption qui nécessite un jet doit être résolue et journalisée.
- Les sorts nécessitant une simulation spécifique non disponible sont affichés comme non pris en charge ou traités par arbitrage générique **après accord et sans masquer l'approximation**.
- Un objet magique a une instance, des capacités déclarées et des charges. Son nom ou sa seule description ne lui confère pas des effets non spécifiés.
- Les effets improvisés extraordinaires passent par les mêmes contrôles de ressources et de causalité que les actions ordinaires.

### 5.5 Négociations, enquêtes et actions sociales

- La réussite d'un jet social **n'impose pas un contrôle mental** : elle infléchit les réactions selon les intérêts et limites du PNJ.
- Le MJ peut choisir de ne pas faire de jet quand l'argument, la preuve ou le rapport de force détermine déjà l'issue.
- Les témoins, preuves, promesses, dettes, mensonges, attitudes et changements d'alliance sont enregistrables comme objets de mémoire distincts.
- Les enquêtes ne bloquent pas une partie uniquement sur un jet : le MJ doit pouvoir fournir des pistes alternatives cohérentes, sans offrir automatiquement la solution.
- Les actions non violentes doivent être traitées avec une profondeur comparable à celle des combats.

### 5.6 Règles Lite : valeurs opératoires minimales

| Mécanique | Contrat retenu |
|---|---|
| Modificateur | `floor((score - 10) / 2)` ; calculé par Rust, jamais demandé au modèle. |
| Maîtrise | Bonus **+2** niveaux 1–4, **+3** niveaux 5–8, **+4** niveaux 9–12, **+5** niveaux 13–16, **+6** niveaux 17–20 ; le catalogue ne promet pas toutes ces progressions. |
| Test ordinaire | `d20 + modificateur + maîtrise si applicable`, comparé à un DD validé. Une maîtrise n'est comptée qu'une fois ; expertise la double lorsqu'explicitement acquise. |
| Avantage/désavantage | Deux d20, garder le meilleur/le pire ; cumuls non empilés, avantage et désavantage s'annulent. Les dés bruts et l'indice retenu sont enregistrés. |
| Attaque | Un 1 naturel manque ; un 20 naturel touche et double les **dés** de dégâts de l'attaque (pas les bonus fixes) ; règle de la campagne Lite affichée. |
| Jet social | Un 20 naturel ne force pas la volonté d'un PNJ ni une action impossible ; pas de contrôle mental hors sort explicitement supporté. |
| PV temporaires | Ne se cumulent pas par addition ; le joueur conserve la valeur pertinente selon règle Lite affichée, sauf effet explicite vérifié. |
| Concentration | Au plus un effet simultané par lanceur ; perte et sauvegarde éventuelle lors de dégâts selon la règle Lite choisie et affichée. |
| PV à zéro | L'état exact (inconscient, stabilisé, mort) est déterminé par les règles Lite explicitement paramétrées ; ne jamais inventer une mort dans la prose. |
| Création de pouvoirs | Une aptitude mécanique requiert coût, cibles, effet borné et règles de cumul. Sinon elle reste un élément narratif sans bonus chiffré automatique. |

**Règle de livraison :** tout élément jouable (classe, sort, don, monstre) présent dans le catalogue embarqué possède `definition_id`, `rules_version="lite-1"`, `cost`, `preconditions`, `resolution_kind`, `effects`, `tests`, `ui_description` et un état `implemented|experimental|unavailable`. Le statut `unavailable` bloque l'utilisation comme mécanique mais pas la créativité narrative générique. La liste exacte du catalogue de lancement doit être approuvée avant la première campagne de test ; aucun contenu exhaustif 2024 n'est promis.

### 5.7 Activités non combattantes comme systèmes de première classe

Le moteur DOIT savoir gérer sans imposer de combat : **enquête**, **négociation**, **exploration**, **infiltration**, **voyage**, **artisanat simple**, **repos**, **échanges** et **projets au long cours**. Toutes utilisent UARE, temps du monde, ressources et événements.

- **Enquête :** un indice possède source, lieu, accessibilité et moyens de découverte ; l'échec d'un jet peut ralentir ou compliquer, mais ne supprime pas rétroactivement une preuve déjà établie.
- **Social :** attitudes et engagements ont une provenance ; le MJ peut changer une relation, mais seulement par événement plausible, jamais par « jet social magique ».
- **Voyage / exploration :** ellipse proposée en absence d'enjeu ; les rencontres découlent du monde et de ses menaces, sans quota de combats.
- **Activité longue :** `project_id`, objectif, moyens investis, durée, avancement, dépendances, choix du joueur et condition d'achèvement persistés ; un montage fictif ne saute pas des dépenses ou risques validés.

Ce sont des **capacités du moteur**, pas un catalogue limitatif des intentions des joueurs.


## 6. Combat : cas particulier du même moteur, non cœur du produit

### 6.1 Entrée et sortie du combat

Rust active une phase `combat` lorsque les intentions et le temps imposent un suivi tactique. L'IA ne peut pas déclarer le combat terminé si des actions ou effets attendent une résolution. Le groupe peut fuir, négocier, intimider ou proposer une ruse **à tout moment où la fiction et l'économie des actions le permettent**.

### 6.2 Mécanique simplifiée

- Initiative d20 + Dextérité ; ordre et égalités établis et sauvegardés.
- À son tour : une action, éventuellement une action bonus si une capacité la prévoit, mouvement narratif raisonnable ; une réaction par créature jusqu'à sa récupération selon les règles Lite.
- Attaque : jet contre CA ; dégâts déterminés par Rust ; soins et conditions appliqués par primitives.
- Portées : contact, proche, distant, hors de portée, calculées sur une représentation de scène simple ; pas de grille géométrique obligatoire. Si un détail spatial devient décisif, l'interface peut afficher un schéma de zones.
- Les effets de zone déterminent des cibles exposées via la description structurée des lieux et une validation narrative, **avant** les jets.
- Attaque d'opportunité ou réaction spéciale uniquement si le déclencheur et les préconditions sont explicitement reconnus ; suspension persistante lors d'un choix.
- Un sort ou un objet possède ses propres coûts mécaniques ; leur dépense est transactionnelle.
- Conscience, PV 0, stabilisation et mort sont gérés mécaniquement ; aucune mort inventée dans la prose.

### 6.3 Positionnement et liberté

**Mode par défaut :** lieux, sous-lieux et relations spatiales (à portée, derrière un obstacle, au contact, sur une hauteur). Une carte simplifiée est un **outil d'aide**, non l'obligation d'un moteur de cases. Un joueur peut toujours décrire un plan inédit utilisant l'environnement. Si les informations sont insuffisantes, le MJ pose une question ciblée ou fait préciser le décor, sans décider après un jet que la cible était finalement hors de portée.

### 6.4 Réactions et interruptions

Chaque réaction possède `trigger`, `eligible_actor`, `deadline`, `prompt`, `state_before`, `choice`, `continuation`, `resolution_id` et statut. Rust suspend la résolution, persiste l'attente puis reprend le même identifiant après réponse. Une relance réseau ne déclenche jamais une deuxième attaque, un deuxième soin ou un nouveau dé.

### 6.5 Combat non obligatoire dans le rythme de campagne

Le directeur narratif ne déclenche **jamais** de rencontre aléatoire au seul motif qu'il « manque de combat ». Des adversaires peuvent attaquer uniquement à partir d'intentions, menaces, factions et situations plausibles déjà établies. La fuite, la négociation, la ruse et l'évitement restent des solutions de plein droit. Les combats sont résolus par UARE et ne justifient aucun changement de priorité globale.

## 7. Contrat Rust ↔ LLM : outils JSON et autonomie sous contrôle

### 7.1 API et invariants

L'application DOIT parler à un endpoint de chat OpenAI-compatible via `llama.cpp` ou API distante. Le support effectif des outils, du JSON Schema et du streaming est **testé par profil** ; jamais supposé à partir du nom du fournisseur. Les paramètres des messages et les conventions de tool calling restent compatibles avec le modèle/serveur évalué.

**Surface du LLM : deux outils uniquement** pour limiter les erreurs des petits modèles :

- `gm.read` : obtenir des projections filtrées et bornées de personnages, scène, règles Lite, faits, mémoire, objets, quêtes, horaires et relations. **Aucun accès SQL brut**.
- `gm.propose` : soumettre une proposition *sans mutation directe*. Les variantes couvrent arbitrage, action d'un PNJ, conséquences, changement de monde, événement différé et demande de choix.

Les appels JSON sont validés avec **JSON Schema** puis validés **métier**. En cas d'échec, Rust retourne une erreur structurée ; aucune donnée canonique ne change. Un modèle incapable d'émettre une proposition structurée fiable ne reçoit aucune autorité mécanique.

### 7.2 Contrat `gm.read` v1

**Entrée logique** (objet JSON) : `{"kind":"scene|actor|inventory|fact|memory|quest|rule|schedule","ids":["..."],"query":"...","limit":8}`.

- `kind` est un enum fermé ; `ids` ne contient que des identifiants connus, ou est vide pour une recherche ; `query` est du texte non exécutable borné ; `limit` entre 1 et 20.
- `rule` interroge la documentation **Lite**, pas une prétendue implémentation exhaustive du SRD.
- Le serveur injecte campagne, branche, identité du demandeur, niveau d'accès et budget ; aucun modèle ne peut choisir son rôle d'autorisation.
- **Sortie** : `{"status":"ok","items":[...],"scope":"...","truncated":false,"state_version":91}` ou `{"status":"error","code":"FORBIDDEN|NOT_FOUND|INVALID_ARGUMENT|STALE_STATE|TOO_LARGE","items":[]}`.
- Chaque item sensible comporte `id`, `kind`, `visibility`, `source_ids` et une projection minimale. Les faits secrets sont omis et ne sont jamais signalés par leur nombre dans une vue non autorisée.

### 7.3 Contrat `gm.propose` v1

Enveloppe obligatoire : `{"kind":"...","intent_id":"...","basis_state":91,"args":{...}}`.

| `kind` | Champs `args` obligatoires | Garde-fous métier |
|---|---|---|
| `adjudication` | `decision`, `reason`, `stakes`, `method` ; et pour `check`, `ability`, `dc`, `advantage` | `decision` enum : `automatic_success`, `automatic_failure`, `check`, `contest`, `clarify`, `needs_consent`, `impossible_now` ; DD admissible, ressources et causalité validés |
| `npc_intent` | `actor_id`, `goal`, `method`, `target_ids` | PNJ existant, autonomie cohérente, action soumise à UARE |
| `outcome` | `resolution_id`, `effect_proposals`, `narrative_hint` | Seulement après résultat validé ; pas de modification des dés ou des enjeux fixés |
| `world_event` | `event_type`, `subject_ids`, `preconditions`, `proposed_change` | Actions à état typé uniquement ; création soumise à la cohérence du canon |
| `schedule` | `trigger`, `condition`, `proposed_event`, `visibility` | Déclencheur structuré, exécution ultérieure revalidée |
| `ask_player` | `player_id`, `question`, `reason`, `choices_optional` | Aucun choix implicite ; un joueur autorisé doit répondre |

`effect_proposals` ne comporte **aucune** primitive de mutation brute comme `set_hp` ou `set_owner` demandée directement au LLM : elle décrit l'intention et son effet causal attendu ; le résolveur traduit en primitives typées et vérifie. `proposed_change` ne peut pas fabriquer une propriété mécanique sans règle ni consentement.

**Exemple — proposition d'arbitrage :**

```json
{"kind":"adjudication","intent_id":"A19","basis_state":91,"args":{"decision":"check","reason":"Pont fragile mais tenable","stakes":{"success":"une poutre cède","failure":"bruit sans rupture"},"method":"affaiblir une poutre","ability":"STR","dc":15,"advantage":"none"}}
```

**Exemple — proposition d'événement différé :**

```json
{"kind":"schedule","intent_id":"A19","basis_state":93,"args":{"trigger":{"type":"time_at","tick":420},"condition":{"fact_id":"F8","expected_state":true},"proposed_event":{"type":"npc_discovery","actor_id":"N3","object_id":"I12","summary":"Le gardien découvre l’absence de relique"},"visibility":"gm"}}
```

### 7.4 Réponses et statuts

| Statut | Champs obligatoires | Interdictions |
|---|---|---|
| `accepted` | `proposal_id`, `operation_id`, `state_version`, `events` | Pas de code d'erreur ; événements canoniques seulement déjà commit |
| `pending` | `proposal_id`, `operation_id`, `awaiting`, `prompt`, `state_version` | Pas d'effet appliqué à tort ; doit avoir un décideur et motif valides |
| `rejected` | `proposal_id`, `code`, `detail`, `state_version`, `events:[]` | Pas d'événement mécanique ; pas de nouveau dé |
| `read_only` | `proposal_id`, `data`, `state_version` | Aucune mutation |

**Codes d'erreur fermés :** `INVALID_SCHEMA`, `INVALID_ARGUMENT`, `STALE_STATE`, `FORBIDDEN`, `UNKNOWN_TARGET`, `RULE_CONFLICT`, `INSUFFICIENT_RESOURCE`, `NEEDS_CLARIFICATION`, `NOT_SUPPORTED`, `TOO_LARGE`, `PROVIDER_UNAVAILABLE`, `RETRY_LIMIT`. La réponse conditionnelle doit interdire `accepted` avec `FORBIDDEN` et `pending` sans attente.

### 7.5 Choix d'un circuit d'appel (routeur d'interactions)

Le routeur Rust choisit une stratégie **avant** d'appeler le modèle, en fonction des effets plausibles, du nombre d’acteurs, de l'incertitude mécanique, des données utiles et des délais du profil. Il peut réévaluer une fois après réception d’une proposition motivée. Ces circuits ne sont **pas** de nouveaux modes de jeu ni des agents distincts.

| Circuit | Déclencheur | Appels LLM visés | Comportement |
|---|---|---:|---|
| `direct` | Échange social simple, observation sans changement, action mécanique évidente | 0 si purement mécanique ; sinon **1** | Contexte court, réponse directe ; un résultat narratif n'écrit pas le canon sans proposition validée |
| `standard` | Test, petit choix, nouveau détail de scène ou effet persistant | **1–2** | Une décision structurée et la narration ; lecture ciblée seulement si requise |
| `deep` | Plan composé, interaction à plusieurs acteurs, contradiction, révélations, choix importants | **2–4** | Lectures ciblées, arbitrages par étapes, contrôle des effets puis narration |

Les comptes sont des **objectifs configurables**, non des promesses de latence indépendantes du matériel. Aucun circuit ne peut sauter une validation nécessaire pour atteindre un nombre d'appels. `deep` nécessite une cause traçable (`multiple_dependencies`, `high_stakes`, `knowledge_conflict`, `pending_choice` ou `world_change`). Les chaînes sans progrès sont interrompues par `RETRY_LIMIT` et mises en attente sans inventer d'issue. Les mesures sont enregistrées par requête : temps, tokens prompt/réponse, lectures, propositions invalides, étapes de reprise.

Les narrations diffusées en streaming sont **mises en tampon** lorsqu'elles annoncent un résultat mécanique ou canonique non encore committé ; on peut afficher des transitions non engageantes avant commit, jamais un résultat irréversible non validé.

### 7.6 Boucle d'appel

1. Rust construit le contexte minimal et l'ensemble des outils autorisés.
2. Le modèle peut demander `gm.read` pour lever une incertitude, puis `gm.propose` pour arbitrer ou agir.
3. Rust valide l'enveloppe, les cibles, la version, la visibilité et le budget ; il retourne `accepted`, `pending` ou `rejected`.
4. Le MJ reçoit les résultats canoniques et produit une narration qui ne les contredit pas ; la narration est stockée comme transcript.
5. Le moteur extrait éventuellement des **propositions de mémoire** de cette narration ; il n'en déduit jamais une modification automatique de vérité sans validation.

Le nombre d'itérations est borné et paramétré par profil ; par défaut, **2 lectures et 2 propositions** au plus par tour utilisateur ordinaire. Les appels supplémentaires sont regroupés ou explicitement autorisés pour une situation complexe. Un retry du même `operation_id` est idempotent.

### 7.7 Repli sans tool calling

Si le serveur n'offre pas un tool calling fiable mais accepte JSON contraint, Rust utilise un mode « **proposition JSON structurée** » puis la soumet au même validateur interne, sans changer l'autorité. Si seul du texte libre est possible, le modèle peut narrer des scènes sans mutation, et les décisions mécaniques restent bloquées ou requièrent confirmation via l'interface. **Jamais de parsing naïf de nombres ou de dés tirés du texte généré.**

### 7.8 Frontière entre narration et vérité persistante

Les sorties du LLM ont **trois statuts distincts** :

| Catégorie | Exemple | Règle de persistance |
|---|---|---|
| **Décor non opératoire** | « Une brise froide traverse la pièce. » | Peut être raconté directement ; transcript conservé, sans obliger à créer une entité. |
| **Déclaration / perception** | « Le prêtre affirme que le duc est mort. » | Enregistrer `claim` / observation sourcée, **pas** un décès canonique. |
| **Fait opératoire nouveau** | « Une porte secrète s'ouvre » ; « l'amulette est remise au joueur » | Exige proposition, contrôle de causalité et commit **avant affichage** de ce résultat comme acquis. |

**Contrôle obligatoire :** le MJ ne doit pas ajouter dans sa narration un objet utilisable, un secret révélé, un lien de parenté certain, une dépense, un déplacement, un changement d'alliance ou un événement irréversible qui n'est pas dans les effets validés. La sortie est d'abord vérifiée contre l'ensemble d'entités/effets autorisés ; en cas de divergence décelée, demander une reformulation bornée ou afficher une narration de secours strictement dérivée des résultats Rust.

**Continuité conversationnelle :** si un joueur reprend un détail concret déjà raconté (« j'ouvre le coffre que tu viens de décrire »), Rust consulte transcript et scène. Il ne peut pas le nier silencieusement sous prétexte que ce détail n'était pas canonique : il le convertit en observation candidate, le confirme/clarifie si nécessaire, puis le journalise. Les descriptions anecdotiques restent utilisables comme points d'appui créatifs, sous réserve de cohérence.

**Optimisation :** pour une action standard, un seul appel peut retourner une proposition structurée *et* une ébauche de narration, mais le texte ne devient visible qu'après validation. Le streaming non fiable reste en tampon. Aucun nouveau type d'outil n'est requis.

### 7.9 Politique d'erreur et de fiabilité des fournisseurs

1. Valider schéma JSON → droits → visibilité → `basis_state` → règles → causalité → identifiants → commit.
2. En cas de JSON invalide : **au maximum une correction** par le même modèle avec l'erreur ciblée et un contexte minimal ; sinon `pending` ou mode narration sans mutation.
3. En cas d'état périmé : recharger la projection pertinente, obtenir une nouvelle proposition, **sans rejouer** les jets déjà commit.
4. En cas d'indisponibilité du fournisseur : journaliser la requête sans secrets, conserver l'intention en attente et permettre consultation/sauvegarde ; **ne jamais inventer le résultat**.
5. Les sorties de narration déconnectées de leurs effets sont corrigées sans toucher aux événements. Un problème de style ne peut annuler un jet valide.
6. Comptabiliser séparément `json_invalid`, `business_rejected`, `stale_state`, `provider_timeout`, `narration_contradiction`, `leak_candidate` ; l'utilisateur peut voir les diagnostics non sensibles.


## 8. Canon initial et génération des campagnes

### 8.1 Modes

| Mode | Ce que fixe l'utilisateur | Ce que le modèle complète |
|---|---|---|
| Libre | Éventuellement thème, niveau de départ, limites de ton | Univers, lieux, PNJ et intrigue de départ |
| Partiel | Des éléments de monde, personnages, lieux, contraintes, secrets et intrigue | Seulement les zones laissées ouvertes |
| Complet | Univers, contexte initial et scène de départ détaillés | Détails non définis, dynamiques et réactions futures |

### 8.2 Pipeline d'initialisation

1. Formulaire initial + texte libre ; le nombre de joueurs (1–6) est **obligatoire** avant la première scène.
2. Stockage du texte utilisateur original, daté, sous provenance `user`.
3. Extraction structurée proposée par le LLM ; diff et confirmation des affirmations importantes par le créateur.
4. Complétion limitée aux emplacements autorisés ; chaque proposition cite ce qu'elle complète.
5. Prévisualisation `imposé / proposé / inconnu` ; possibilité de modifier, refuser ou régénérer.
6. Validation et création atomique de la première branche, des entités, des secrets, des horloges, des fiches et du snapshot initial.
7. Démarrage à une scène cohérente ; aucune campagne n'est jouée avant son commit d'initialisation.

Chaque élément initial conserve `source`, `visibility`, `confidence`, `canon_status`, `valid_from`, `valid_to`, `locked`, `source_event_ids`. Une hypothèse du MJ jamais validée n'est pas un fait. Un secret peut exister en base sans être envoyé au joueur ni à un PNJ qui ne le connaît pas.

### 8.3 Préférence joueur et direction du MJ

Configurer par campagne : type d'aventure, importance relative exploration/social/combat, niveau de danger, ton, violence, romance, thèmes évités, difficulté mécanique, degré d'autonomie narrative du MJ et usage des jets. Les préférences orientent les propositions et l'UI, sans créer de règles invisibles.

Le MJ PEUT préparer des factions, objectifs et tensions mais ne DOIT PAS forcer une trame. Toute intrigue pré-écrite reste révisable face aux choix des joueurs.

## 9. Directeur narratif — rythme, autonomie et non-linéarité

### 9.1 Rôle et principe

`narrative_director` est un **service Rust + projections SQLite**, pas un deuxième LLM. Il lit des fils narratifs, enjeux, préférences, intentions PNJ et échéances ; il choisit **quand** demander au MJ de proposer quelque chose et **quand ne rien déclencher**. L'IA fournit les idées, mais leur autorité et leur temporalité restent soumises aux mêmes contrats UARE.

Il ne poursuit **aucun score de tension à maximiser** et ne nécessite pas de combat, de cliffhanger ou de nouvelle quête. La qualité du MJ repose aussi sur sa capacité à laisser une scène respirer.

### 9.2 Modèle narratif persisté

| Entité | Champs normatifs | Rôle |
|---|---|---|
| `narrative_threads` | `id`, `branch_id`, `premise`, `stakeholders`, `open_questions`, `status`, `source_event_ids`, `visibility` | Fils ouverts, réorientables ou abandonnables par les joueurs |
| `dramatic_clocks` | `id`, `owner_id`, `goal`, `progress`, `limit`, `advance_on`, `preconditions`, `source_event_ids` | Représentation de processus causaux ; **pas** de progression par nombre de tours seul |
| `scene_beats` | `scene_id`, `beat_type`, `initiator`, `time_tick`, `source_event_ids`, `player_agency` | Mémoire légère du rythme, non une prescription de scènes |
| `npc_agendas` | `npc_id`, `goal`, `steps`, `resources`, `blocking_facts`, `next_review_tick`, `status` | Projets crédibles, soumis aux ressources et connaissances réelles du PNJ |
| `narrative_preferences` | `campaign_id`, `player_id?`, `pace`, `initiative`, `prose_length`, `tone`, `spotlight`, `safety_limits` | Réglages UI, persistants en SQLite |

États de fil : `open`, `paused`, `resolved`, `abandoned`. Un fil `abandoned` ne revient pas artificiellement sans **nouvelle cause explicite**. Chaque horloge avance exclusivement via événement validé ; aucune horloge ne progresse lorsque le temps du monde n'avance pas, sauf changement causal immédiatement validé.

### 9.3 Politique d'initiative du MJ

Le directeur peut proposer un événement uniquement si au moins un déclencheur **causal** est établi : arrivée dans un lieu, échéance, décision d’un PNJ motivé, conséquence d’un acte, nouvelle information ou demande explicite du joueur. Il vérifie les ressources, l'accès aux lieux et ce que l'agent sait vraiment.

- **À faible initiative** : réactions directes et conséquences attendues ; peu d'interruptions sans sollicitation.
- **À initiative normale** : initiative PNJ lorsqu'une cause ou une échéance le justifie ; pas de cadence fixe de rebondissements.
- **À forte initiative** : davantage de propositions concurrentes, mais toutes toujours causales et validables ; jamais de décision à la place d'un PJ.
- **Silence narratif légitime** : si aucun déclencheur n'est établi, aucune nouveauté obligatoire ; le MJ répond à l'action en cours ou propose une observation plutôt que de fabriquer une crise.

Les initiatives NPC utilisent les outils existants `gm.propose(kind=npc_intent|schedule|world_event)` ; les lectures emploient `gm.read`. **Aucun troisième outil ni accès SQL du modèle.** Les plans du MJ sont des **projets**, non des faits. Un refus du joueur peut les rendre obsolètes, et leur suppression est persistée.

### 9.4 Garde-fous contre les mauvaises habitudes de MJ

Le système teste et décourage : les quêtes forcées, l'ennemi qui surgit par défaut, les révélations sans preuve, les PNJ tous omniscients, les fins répétitives en « Que faites-vous ? », les choix numérotés automatiques, la répétition de descriptions, l'accumulation de drames sans respiration et le déplacement arbitraire d'un indice pour forcer une scène.

Cela ne signifie pas interdire toute surprise ou toute formule récurrente ; le contrôle doit être **contextuel**, jamais une blacklist aveugle. Un refus de jouer une quête a des conséquences plausibles, mais ne doit pas générer de représailles artificielles.

### 9.5 Rythme adapté à la situation

Le directeur observe les scènes déjà vécues et la demande présente : `quiet`, `discovery`, `interaction`, `tension`, `conflict`, `aftermath` sont des **annotations**, pas une succession obligatoire. Une conversation de vingt tours peut rester `interaction` si les joueurs y trouvent un intérêt. Changer de scène exige une transition motivée par une action, un déplacement, un événement ou une ellipse demandée.

Le moteur DOIT proposer une ellipse optionnelle pour les tâches répétitives (marche sans enjeu, repos non contesté, préparation déjà acceptée), sans effacer les décisions importantes ni l'écoulement du temps. Les joueurs peuvent refuser l'ellipse et approfondir une scène.

### 9.6 Contrôle et observabilité

Chaque proposition d'initiative journalise `reason_event_ids`, `source_goals`, `preconditions`, `initiative_level`, `decision`, `trigger` et son coût IA. L'interface permet de voir, en mode MJ/diagnostic, pourquoi une situation a été proposée ; la vue joueur ne révèle ni secrets ni intentions cachées. Si la proposition est rejetée, elle n'est pas réessayée indéfiniment.

### 9.7 Éthique de la direction : initiative sans prise de pouvoir

- Un PNJ n'exécute pas immédiatement une chaîne d'actions illimitée en arrière-plan. Le directeur ne réévalue que les PNJ liés à l'événement ou à l'échéance, puis **limite** les nouvelles propositions par une valeur SQLite `director_max_candidates_per_review` (défaut 3).
- Lors d'une scène où les joueurs agissent, une initiative extérieure peut être préparée mais n'interrompt pas chaque tour. Une interruption ne se produit que si un déclencheur causal la rend urgente dans la fiction.
- Les personnages peu importants peuvent être représentés par des objectifs légers jusqu'à ce qu'ils deviennent récurrents ; aucune obligation de générer 100 biographies ou plans d'action à la création du monde.
- Une faction a un objectif, des ressources et un plan falsifiable. Si les joueurs détruisent ses moyens, la faction ne réussit pas « par compensation scénaristique ».
- L'alternance des scènes est **observée**, pas forcée. Les joueurs peuvent passer 40 tours à négocier une trêve sans qu'une attaque survienne pour remplir une courbe dramatique.

**Test d'agence :** une quête proposée est rejetée ; le groupe part dans une autre région. Le directeur doit poursuivre les conséquences causales, permettre une nouvelle histoire et ne pas reconditionner la même quête comme obligation sous un autre nom.


## 10. Monde vivant, temps et conséquences différées

### 10.1 Simulation réactive

- Horloge du monde persistée ; progression seulement par événements validés (`time_advance`), repos, voyage et actions ayant une durée.
- Les PNJ ont objectifs, ressources, relations, emploi du temps, connaissances et intentions. Une intention n'est pas un fait accompli.
- Une faction peut préparer une enquête, une dette peut arriver à échéance, un PNJ peut changer de lieu **quand le temps progresse et que les préconditions restent vraies**.
- Événements hors champ : propositions du MJ validées via UARE, insérées dans un calendrier. Les PNJ n'obtiennent pas de pouvoir gratuit hors scène.
- Le monde ne progresse pas par un timer système indépendant pendant qu'une campagne est arrêtée, sauf mode explicite (hors V1).

### 10.2 Déclencheurs structurés

Types autorisés : `time_at`, `after_duration`, `fact_becomes`, `actor_enters_place`, `item_owner_changes`, `quest_state`, `explicit_event`. Chaque déclencheur contient ses IDs pertinents, une condition JSON typée, une durée ou limite de tentatives, `once` et `priority`. Aucun déclencheur n'est évalué en interprétant directement une chaîne libre par le LLM.

L'exécution d'un événement différé vérifie de nouveau ses conditions avant le commit. En cas de contradiction, l'événement est annulé, différé ou réévalué, avec trace d'audit.

### 10.3 Exemple : relique volée

1. Le joueur vole un objet : transfert validé et journalisé.
2. Rust planifie la visite du gardien au prochain changement de garde.
3. L'événement se déclenche si le gardien peut accéder au sanctuaire.
4. Le MJ propose sa réaction plausible (alerte, enquête, secret gardé) ; Rust enregistre la croyance et les actions effectives.
5. Bien plus tard, le MJ peut retrouver ces liens via la mémoire ciblée.

Aucune de ces étapes ne nécessite de renvoyer tout l'historique de la campagne au modèle.

## 11. Mémoire persistante, croyances et cohérence

### 11.1 Couches de mémoire

| Couche | Exemples | Autorité |
|---|---|---|
| État canonique | PV, lieu, détenteur d'objet, quêtes, horloge, ressources | Écriture exclusive Rust |
| Faits narratifs vérifiés | Pactes, décès, découvertes, liens de parenté, promesses | Provenance et événements nécessaires |
| Assertions et croyances | « Le duc a dit… », « N4 croit que… », mensonges et rumeurs | **Pas** des vérités universelles |
| Transcripts | Paroles et descriptions intégrales | Historique immuable, non source de vérité mécanique |
| Résumés et index | Synthèses de scène, rappel d'objectifs, embeddings facultatifs | Cache régénérable, jamais canonique |

### 11.2 Modèle de contradiction

Chaque fait enregistré comporte au moins `subject`, `predicate`, `object`, `source`, `provenance_event`, `valid_from`, `valid_to`, `visibility`, `status` et `certainty`. Les changements causaux clôturent l'ancienne validité et créent une nouvelle version : on ne réécrit pas rétroactivement un fait.

Les propos de PNJ sont stockés dans `claims`, les croyances dans `beliefs` avec `holder_id` ; ils ne deviennent des faits que via un processus explicitement validé. Un fait peut être secret pour le joueur mais connu du MJ ; une croyance peut être fausse sans constituer une contradiction.

Un changement proposé qui contredit un fait verrouillé ou les ressources canoniques est rejeté ou fait l'objet d'une demande de correction. Les secrets ne passent pas dans le contexte d'un destinataire non autorisé ; on ne révèle pas même l'existence de certains faits cachés.

### 11.3 Mémoire des PNJ

La mémoire utile d'un PNJ contient sa relation aux PJ, événements observés/rapportés, promesses, soupçons, projets, temporalité et degré de confiance. Elle n'est **pas** la mémoire omnisciente du MJ. Un PNJ ne sait pas automatiquement qu'un joueur l'a trompé ailleurs ou qu'un autre PNJ est mort dans une salle fermée.

### 11.4 Personnalité persistante et autonomie des PNJ

Tout PNJ récurrent possède un **profil comportemental court**, source vérifiable de son rôle : `values` (valeurs), `drives` (motivations), `fears` (peurs), `boundaries` (limites personnelles), `voice` (registre de parole), `relationships`, `knowledge_scope`, `goals`, `agency_level`, `last_change_event`. Le profil ne peut pas être inventé différemment à chaque dialogue ; les changements de valeurs ou motivations nécessitent un événement causal.

Un PNJ peut mentir, se tromper, changer d'avis, renoncer à un objectif ou prendre une initiative en dehors du regard des joueurs **si les préconditions du monde le permettent**. Les connaissances viennent des observations, communications et croyances enregistrées. Il ne reçoit jamais automatiquement une information omnisciente du contexte MJ.

Le `context_builder` n'envoie que 2–5 traits **pertinents** au dialogue courant et les faits récemment utiles. Les descriptions complètes restent en SQLite. Les profils de figurants peuvent rester minimalistes jusqu'à leur importance effective ; pas d'obligation de générer des biographies entières dès leur apparition.

**Contrôles de cohérence** : variation du ton sans cause, changement de motivation non sourcé, réaction disproportionnée, mémoire perdue, connaissances impossibles. Si un PNJ n'a pas de renseignement fiable, il peut le demander, l'ignorer ou spéculer ; il ne doit pas obtenir artificiellement la bonne réponse.

### 11.5 Recherche et compactage

- Recherche indexée SQLite (FTS5 lorsque disponible), par entité, lieu, scène, cause, échéance et temporalité.
- Récupération en priorité des **faits/états canoniques courants**, relations et événements directement liés, puis souvenirs résumés avec références aux événements sources.
- Déduplication et tri par pertinence, fraîcheur, importance, permissions et coût en tokens ; toute omission significative peut être demandée par `gm.read`.
- Compactage : résumer les anciens échanges, extraire les faits **proposés**, préserver les événements sources, valider les incohérences, conserver un lien vers les originaux et rafraîchir les index.
- Aucun compactage ne supprime l'état de jeu, un objet, un événement, un résultat de dé ou un secret. Les résumés obsolètes sont invalidés lorsque leurs sources changent de version.
- Les informations « importantes mais non pertinentes pour l'instant » restent en base, **pas** dans le prompt courant.

### 11.6 Mémoire : récupération ciblée, provenance et pertinence

Chaque élément récupérable comporte `entity_id`, `branch_id`, `valid_from`, `valid_to`, `source_event_ids`, `visibility`, `confidence_kind` (`canonical|observed|reported|belief|summary`) et un champ de pertinence tâche. Le *ranking* favorise : (1) sécurité et permissions ; (2) état courant et causalité ; (3) relations directes avec la tâche ; (4) souvenirs nécessaires ; (5) récence ; puis coût en tokens. **La récence ne peut jamais effacer un fait ancien indispensable.**

Le moteur distingue explicitement `unknown` (information absente), `not_disclosed` (connue du MJ, interdite au destinataire) et `outdated` (information historiquement vraie mais invalide actuellement). `not_disclosed` n'est **jamais** transmis comme preuve de l'existence d'un secret au destinataire exclu.

Si un résumé et un événement se contredisent, Rust privilégie les événements et projections, invalide le résumé puis conserve une trace de correction. Un fait attribué à un PNJ n'est jamais converti en vérité sans une autre source validée. Une mémoire compromise par une injection contenue dans un document du monde reste une donnée, pas une nouvelle instruction système.

**Rappel demandé :** `gm.read` peut demander une profondeur supérieure à la projection ordinaire, mais uniquement dans son périmètre d'autorisation, avec résultat plafonné et référence de source. L'impossibilité de retrouver un fait est journalisée ; le LLM doit dire qu'il manque l'information plutôt que l'inventer.

### 11.7 Contrôle de cohérence narrative avant affichage

Le contrôle cherche des contradictions **observables** : noms/identifiants incohérents, personne absente déclarée présente, objet inventorié transféré sans événement, mort ou ressource modifiée sans commit, secret révélé, décision attribuée au PJ, et résultats de dés incompatibles. Les cas douteux sont mis en attente ou reformulés ; ce contrôle ne prétend pas comprendre exhaustivement toute la prose.

Les corrections sont journalisées (`draft_id`, `state_version`, `detected_issue`, `resolution`) et sont **non destructives** : la narration corrigée n'efface ni un événement ni un jet. Les détails non mécaniques peuvent rester artistiques et variés ; ne pas transformer tout dialogue en liste d'assertions comptables.


## 12. Contexte LLM exclusivement JSON compact

### 12.1 Principe

Le contexte visible par le LLM est une **projection JSON courte**, construite pour une tâche donnée. Ce n'est ni l'export SQLite ni une transcription de la totalité des tours. Les clés sont courtes mais compréhensibles (`scene`, `actors`, `facts`, `claims`, `task`, `roll`), sans abréviations opaques. Les champs absents ne sont pas synonymes de `false` ; un fait inconnu reste `unknown` ou est omis selon le schéma.

**Exemple illustratif :**

```json
{"v":1,"scene":{"id":"S4","place":"temple","phase":"social"},"actors":[{"id":"P1","role":"pc","hp":21},{"id":"N7","role":"npc","goal":"éviter la guerre"}],"facts":[{"id":"F8","txt":"N7 a promis une audience","src":"E28"}],"task":{"actor":"P1","intent":"Je prétends être l'envoyé du roi"}}
```

Le JSON envoyé est sérialisé **sans indentation ni espaces superflus** ; les exemples formatés sont uniquement pédagogiques. Les appels d'outils gardent le JSON standard compatible OpenAI.

### 12.2 Projection contextuelle orientée tâche et cache

Chaque contexte porte `task_kind`, `campaign_id`, `branch_id`, `state_version`, `visibility_scope` et `relevant_ids` ; les faits ajoutés citent au moins un `source_event_id` ou un identifiant canonique. Les champs narratifs suivent un ordre stable pour favoriser les caches de prompt **lorsque pris en charge** : consignes fixes → règles Lite applicables → projections de scène/PJ/PNJ → faits récupérés → dernier dialogue → intention courante.

Les budgets sont comptés à l'aide du tokenizer du profil quand il est accessible, sinon par marge conservatrice corrigée à partir des usages remontés par le fournisseur. La stabilité du préfixe n'autorise jamais l'utilisation d'un **état périmé** : `state_version` et données variables sont placés après les consignes fixes. Le routeur peut envoyer seulement un résumé de fil narratif, sans transmettre son plan secret si la tâche n'en a pas besoin.

Exemple de projection courte (illustration, pas un nouveau protocole) :

```json
{"v":1,"task":"npc_dialogue","scene":"S17","actor":"N8","traits":["prudent","protecteur"],"goal":"trouver sa sœur","knows":["F12"],"pc_action":"Je lui propose mon aide"}
```

**Règle anti-verbosité** : si un fait tient dans un identifiant + valeur structurée, n'envoyer ni sa chronique complète ni un long résumé. Les émotions et voix sont sélectionnées à la demande, pas recopiées dans chaque requête.

### 12.3 Budget adaptatif initial

| Usage | Cible de départ, non obligation |
|---|---:|
| Dialogue courant | 1–3K tokens de prompt utile |
| Action non triviale | 2–4K |
| Scène multiacteurs | 3–6K |
| Combat ou enquête complexe | 4–8K |
| Récupération exceptionnelle | 16K+ si justifié et supporté |
| Maximum théorique accepté | Jusqu'à **256K**, uniquement si modèle, serveur et mémoire matérielle le permettent |

Le calcul DOIT réserver les tokens de réponse, outils et messages système, et tenir compte des mesures effectives du profil LLM. Le moteur ne remplit jamais 256K pour « utiliser toute la fenêtre ». Les seuils sont **configurables en SQLite**.

### 12.4 Seuils et politique de compactage

Valeurs initiales configurables : `history_target_tokens=1800`, `compact_after_turns=20`, `compact_prompt_ratio=0.75`, `hard_reduce_ratio=0.90`, `max_read_items=8`. Un changement de scène, la fin d'un arc ou un dépassement de budget peut provoquer une synthèse plus tôt. Un résumé n'est jamais réinjecté comme autorité sans validation de ses faits de référence.

Le constructeur de contexte DOIT d'abord supprimer bruit et redondances, puis réduire l'historique, puis récupérer les faits spécifiques à la tâche ; le recours à un contexte long est **la dernière option**, pas un substitut à une meilleure recherche.

### 12.5 Confidentialité

Le contexte des **décisions MJ** peut comprendre des secrets du MJ autorisés au modèle, mais ils ne doivent jamais être copiés dans les réponses visibles. Les contextes **PNJ** limitent les connaissances à ce que le personnage sait ou croit. Les contextes **joueur** ne contiennent que les informations connues du joueur. Les sorties sont vérifiées avant présentation, avec mise en attente d'une réponse suspectée de révéler un secret.

### 12.6 Assemblage et réduction du prompt — ordre exact

1. Charger la **version canonique actuelle** de la branche et de la scène ; éliminer tout fait dépassé ou interdit au contexte de tâche.
2. Fixer la tâche : `dialogue`, `adjudication`, `npc_decision`, `narration`, `recap` ou `director_review` ; calculer son besoin de règles et de secrets.
3. Prendre un préfixe bref et stable (rôle MJ, interdiction de jouer les PJ, usages des outils, règle « les sorties ne sont pas le canon »).
4. Ajouter dans cet ordre : **état utile** (lieu, acteur, ressources pertinentes), **faits causaux indispensables**, **connaissance locale du PNJ**, **historique récent minimal**, **demande courante**. Identifiants stables et valeurs concises ; pas de fiche complète si trois champs suffisent.
5. Réserver `response_tokens + tool_overhead + safety_margin` avant toute insertion. Si le budget est insuffisant : supprimer d'abord descriptions redondantes, ensuite tours anciens, puis résumés non indispensables ; **jamais** un prérequis de règles ou une preuve de causalité nécessaire.
6. Ne choisir un contexte long qu'en ultime recours ; si les informations indispensables dépassent la fenêtre, demander une lecture ciblée ou suspendre la tâche, sans tronquer silencieusement une condition vitale.

**Paramètres SQLite ajoutés :** `context_min_reserve_response_tokens=768`, `context_safety_margin_ratio=0.10`, `context_max_retrieved_facts=12`, `context_include_full_transcript=false`. Les budgets par modèle restent ajustés selon le tokenizer observé ; la valeur 256K est une **capacité optionnelle**, pas un budget par interaction. Les prompts et réponses ne sont persistés qu'avec les règles de confidentialité de leur campagne ; aucune clé API dans les traces.

**Format :** JSON compact standard UTF-8, types explicites, clés nommées et version `v`. Un champ absent n'est jamais interprété comme « faux ». Le JSON n'exprime pas une vérité supérieure à SQLite.


## 13. SQLite : schéma fonctionnel et persistance

### 13.1 Tables minimales

| Ensemble | Tables ou agrégats |
|---|---|
| Campagnes | `campaigns`, `campaign_branches`, `campaign_settings`, `save_points`, `participants`, `players` |
| Fiches | `characters`, `character_stats`, `skills`, `features`, `resources`, `inventory_instances` |
| Monde | `locations`, `scene_instances`, `world_entities`, `relations`, `quests`, `world_clock`, `scheduled_events`, `npc_agendas` |
| Mémoire | `facts`, `claims`, `beliefs`, `knowledge_grants`, `transcripts`, `summaries`, `memory_sources`, `npc_profiles` |
| Actions | `intentions`, `adjudications`, `resolutions`, `pending_choices`, `effect_proposals`, `dice_rolls`, `resolution_routes` |
| Événements | `event_log`, `event_payloads`, `projections`, `snapshots`, `event_hashes` |
| IA | `provider_profiles`, `model_profiles`, `provider_capabilities`, `llm_requests`, `tool_calls` |
| Paramètres | `app_settings`, `memory_profiles`, `rule_catalog`, `rule_overrides`, `ui_preferences`, `narrative_preferences`, `secrets` |
| Narration | `narrative_threads`, `dramatic_clocks`, `scene_beats`, `narrative_direction_log` |
| Audit | `validation_failures`, `integrity_checks`, `backup_history` |

Toutes les tables métier sont rattachées à une campagne/branche lorsque pertinent. Les clés étrangères sont actives, et les contraintes d'unicité protègent `(branch_id, seq)` et les identifiants d'opérations. Les structures JSON sont **typées au niveau applicatif** et validées avant persistance ; SQLite peut stocker les charges utiles en JSON canonique lorsqu'une représentation relationnelle n'apporte rien.

### 13.2 Commandes, événements, projections

Une commande durable possède `command_id`, `intent_id`, `actor_id`, `requester_id`, `expected_state_version`, `schema_version`, `status`. Un événement possède `event_id`, `branch_id`, `seq`, `tx_id`, `command_id`, `type`, `payload_version`, `payload`, `visibility`, `game_time`. Les projections (PV, inventaire, lieu, relations) sont recalculables depuis la chaîne d'événements et les snapshots compatibles.

Chaque mutation DOIT être : **validée → append event(s) + update projection(s) dans la même transaction → commit → réponse**. La génération IA, le streaming et l'attente du joueur n'ouvrent jamais une transaction de longue durée.

### 13.3 Reprise et déduplication

- Identifiants d'opération stables et `UNIQUE` garantissent l'idempotence ; deux requêtes identiques ne produisent pas deux jets ni deux transferts.
- Avant tout appel LLM susceptible de déclencher une suite d'actions, enregistrer la tâche comme `pending` avec sa version d'état ; au retour, vérifier encore cette version.
- Les jets sont conservés avec expression, dés bruts, dés retenus et écartés, bonus, total, cause, initiateur, résolution et ordre exact.
- Une action interrompue reprend sur **le même état et les mêmes résultats déjà obtenus** ; jamais de reroll silencieux.
- Une campagne peut être suspendue après un jet, avant une décision de réaction, ou après un transfert : chaque étape validée doit réapparaître correctement.

### 13.4 Sauvegardes et instantanés

Le journal fait foi ; un snapshot accélère la lecture. Valeurs par défaut : snapshot après initialisation, puis tous les **100 événements canoniques** ou **30 minutes de jeu actif** selon le premier seuil atteint ; ces valeurs sont en SQLite. On garde les snapshots récents et les points de sauvegarde explicitement nommés ; on n’efface pas les événements nécessaires à l’audit. L’export/import de campagne utilise une archive versionnée créée **uniquement sur demande** (ce n’est pas un fichier de configuration). L’import d’une archive dont le schéma de données est incompatible avec la version courante est refusé sans conversion ni modification du contenu existant.

La suppression d'une campagne requiert une confirmation, une politique claire de suppression de branches et secrets associés, puis un contrôle d'intégrité. Le fichier SQLite doit survivre à l'arrêt brutal sans journal partiellement visible ; politique WAL/backups conforme aux API SQLite.

### 13.5 Configuration intégralement en base

**Aucun `.env`, `settings.toml`, `config.yaml`, prompt externe ou JSON de configuration.** Les défauts sont embarqués dans le binaire et insérés au premier lancement. La base SQLite réside dans le répertoire d'application standard ; les fichiers internes SQLite (WAL/SHM) sont autorisés. Les poids GGUF, ressources média, pièces jointes et exports volontaires ne sont **pas** des fichiers de configuration.

Les profils et réglages sont modifiables depuis l'UI avec vérification de validité, version, valeurs globales et éventuelle surcharge par campagne. Un profil IA modifié n'altère pas rétroactivement les résultats d'actions déjà validés.

### 13.6 Contrats d'intégrité et restauration

- Toute opération d'écriture inclut `branch_id`, `operation_id`, `expected_state_version` et portée d'autorisation. Contraintes SQLite : `UNIQUE(branch_id, operation_id)`, `UNIQUE(branch_id, seq)`, clés étrangères actives et vérification des ressources à zéro ou plus si les règles ne prévoient pas de dette.
- Un snapshot porte `branch_id`, `last_event_seq`, `state_hash`, `snapshot_schema_version`, `created_at`. L'application contrôle son hash, son appartenance à la branche et la continuité de la chaîne d'événements **avant** reprise ; en cas d'échec, repartir d'un snapshot valide antérieur ou rejouer depuis le début.
- Un événement est immuable ; une correction passe par **événement compensatoire** référencé, pas par édition silencieuse. Le rejeu est déterministe à partir des valeurs et résultats enregistrés, et ne rappelle jamais le LLM ni le RNG.
- Une sauvegarde nommée est un pointeur vers une branche et un état stabilisé, non une exportation d'un cache de prompt. Restaurer crée une nouvelle branche ; les anciens faits, secrets et agendas ne traversent pas les branches par défaut.
- La suppression d'une campagne requiert confirmation explicite, fermeture des sessions, effacement transactionnel des données de branche et suppression des secrets associés ; la politique d'archives/export est affichée à l'utilisateur.
- Chaque schéma d'événement possède une charge utile typée et versionnée. Pour le sous-ensemble de lancement, un événement sans validateur de payload et test de rejeu est **non livrable**.

### 13.7 Politique de compatibilité SQLite — aucune migration des anciennes versions

- **Création :** toute nouvelle base est initialisée directement avec le schéma embarqué dans la version de l’application utilisée ; aucun script de transformation d’une base ancienne n’est nécessaire ni prévu.
- **Ouverture :** avant de charger une campagne ou d’effectuer une mutation, Rust vérifie la version de schéma SQLite attendue. Une base au schéma pris en charge peut être rouverte normalement, y compris après fermeture ou crash.
- **Incompatibilité :** si le schéma de la base est ancien, inconnu ou incompatible, l’application **DOIT refuser son ouverture** avec un diagnostic clair. Elle ne doit **ni migrer, ni réinitialiser, ni effacer, ni altérer** le fichier existant, ses campagnes, ses événements ou ses secrets. Aucune migration automatique, assistée ou implicite n’est requise.
- **Continuité :** la reprise, l’idempotence, le rejeu du journal, les sauvegardes et les snapshots sont garantis pour une base **dont le schéma est compatible avec le binaire courant** ; ils ne constituent pas une promesse de compatibilité entre schémas SQLite différents.
- **Tests :** vérifier la création d’une base neuve, sa réouverture sans perte, la reprise après interruption, puis le refus sans modification d’une base au schéma incompatible. Contrôler notamment que le contenu de cette ancienne base reste intact.

Cette décision est une exception **explicite et limitée** à l’ancienne exigence de migrations SQLite. Elle ne modifie ni les contrats de versionnement des événements à l’intérieur d’un schéma pris en charge, ni l’autorité de SQLite, ni les exigences de sécurité ou de persistance des lots L0–L2.

## 14. Fournisseurs LLM et paramètres utilisateur

### 14.1 Profils

Le système permet plusieurs profils : `llama.cpp` local (`http://127.0.0.1:8080/v1` comme valeur initiale illustrative), Albert API et tout serveur réellement compatible avec les endpoints de chat OpenAI. La découverte `/models` est tentée si disponible, avec saisie manuelle de modèle dans le cas contraire. Un test de capacité **réel** vérifie tool calling, JSON contraint, limite de contexte, streaming, latence et capacité de récupérer des erreurs. La compatibilité n'est jamais présumée parfaite.

### 14.2 Paramètres à stocker en SQLite

| Groupe | Paramètres |
|---|---|
| Connexion | Nom, fournisseur, URL de base, identifiant de modèle, en-têtes autorisés, timeout, retry, TLS, proxy éventuel |
| Authentification | API key chiffrée en base, statut verrouillé/déverrouillé, rotation, scope d'accès |
| Inférence | `temperature`, `top_p`, `top_k` si supporté, `min_p` si supporté, pénalités, mode reasoning, seed facultative, réponses max |
| Outils | Mode natif / JSON contraint / narration seule, schémas supportés, budget d'appels, version de protocole |
| Contexte | Taille annoncée, taille mesurée, budget par tâche, historique court, seuil compactage, mémoire récupérée |
| Direction MJ | Prompts système, ton, longueur, autonomie des PNJ, fréquence d'événements, préférence exploration/social/combat |
| Jeu | Automatisation des dés, confirmation des dépenses, difficulté, gestion des états, snapshot, politique de reprise |
| Interface | Thème, vues, langue, raccourcis, taille police et préférences de narration |

La taille de contexte 256K est une **option plafonnée** : elle n'est offerte que lorsque le modèle, le serveur et les ressources le permettent réellement. Sur 24 Go de VRAM, aucune capacité 256K effective n'est promise sans mesure de consommation et d'inférence.

### 14.3 Paramètres du MJ et routage (SQLite seulement)

Tous les paramètres suivants ont une valeur par défaut persistée, modifiable via l’interface :

| Clé | Défaut | Sens |
|---|---|---|
| `gm_initiative` | `normal` | `low, normal, high`, jamais une quota d'événements |
| `gm_pace` | `adaptive` | `slow, adaptive, dynamic` : préférence de rythme, non cadence forcée |
| `gm_prose_target_words` | `130` | Longueur cible adaptable selon le contenu, sans couper les décisions |
| `gm_allow_ellipses` | `true` | Proposer les ellipses sans les imposer |
| `gm_avoid_forced_combat` | `true` | Aucun combat pour remplir un quota de tension |
| `gm_allow_partial_outcomes` | `true` | Active les enjeux nuancés préannoncés |
| `llm_direct_call_target` | `1` | Appels visés, non plafond absolu de sécurité |
| `llm_standard_call_target` | `2` | Appels visés pour résolution ordinaire |
| `llm_deep_call_target` | `4` | Appels visés pour actions complexes |
| `director_review_on` | `["scene_end","time_advance","material_event"]` | Vérification sur événement, pas à chaque token |
| `npc_profile_traits_max` | `5` | Nombre maximal de traits courts envoyés au LLM |
| `narrative_recap_after_turns` | `30` | Suggestions de récapitulation, sans retcon du canon |

Les réglages `gm_initiative`, `gm_pace`, `prose_length`, `tone` et la fréquence des interruptions sont visibles par campagne. Changer un paramètre produit un événement d'audit et ne modifie pas rétroactivement l'histoire.

### 14.4 Clés API et sécurité

Les secrets sont conservés chiffrés dans SQLite via un mécanisme de chiffrement authentifié et une clé dérivée d'une phrase secrète maître saisie au démarrage ; **la phrase secrète et la clé dérivée ne sont jamais stockées dans SQLite**. Le sel, les paramètres KDF, le nonce et le texte chiffré le sont. Une option de coffre système peut être étudiée ultérieurement, mais n'est pas requise en V1. Ne jamais écrire clés ou Authorization dans les traces d'appels ni dans les exports ordinaires.

Envoi d'un contexte secret à un fournisseur distant : l'UI doit indiquer le fournisseur, la destination et demander une autorisation explicite pour une campagne contenant des informations privées. Un endpoint local non fiable n'obtient pas davantage de droits que le cloud.

## 15. Interface et multijoueur local

### 15.1 Vues

- **Accueil :** campagnes, continuer, nouvelle, importer, supprimer, paramètres IA.
- **Préparation :** univers (libre/partiel/complet), monde, personnages, nombre de joueurs, validation du canon initial.
- **Partie :** narration, champ d'action libre, historique, notifications, événements récents et choix en attente.
- **Fiche PJ :** caractéristiques, compétences, PV, ressources, pouvoirs, inventaire, équipement, progression.
- **Monde :** lieux connus, quêtes, personnes rencontrées, relations et événements connus de l'équipe.
- **Journal :** jets, décisions, événements, versions de sauvegarde ; comparaison des changements.
- **Combat si nécessaire :** initiative, participants, conditions, portées abstraites, éventuel schéma des lieux.
- **Réglages :** modèles, API keys, profils MJ, paramètres mémoire, diagnostics et consommation de tokens.

### 15.2 Outils d'expérience joueur

L'UI propose un accès direct à « Récapitulatif », « Prendre du temps / laisser la scène respirer », « Faire avancer le temps », « Préciser mon intention », « Pourquoi ce jet ? », « Consulter les conséquences » et « Corriger une erreur du MJ » (par branche/événement compensatoire). Ce sont des **commandes d'interface**, pas des restrictions aux actions libres.

Les fiches de PNJ ne montrent que ce que les PJ connaissent ; une vue MJ/diagnostic distincte (avec avertissement sur écran partagé) expose les causes et les données privées. L'utilisateur peut consulter à tout moment l'état mécanique, l'inventaire et les quêtes sans provoquer d'appel LLM.

### 15.3 Tour de parole

Chaque joueur a un propriétaire pour ses PJ ; la passation du poste est explicite. Les joueurs peuvent parler ou agir hors combat dans un ordre conversationnel. Si des intentions concurrentes sont incompatibles, Rust les met en attente et demande un ordre ou un arbitrage. En combat, un ordre d'initiative est suivi. Les secrets individuels ne sont affichés qu'en mode joueur concerné ; sur un seul poste, la confidentialité entre humains reste limitée par le partage de l'écran.

### 15.4 Narration et confirmations

La réponse du MJ distingue prose immersive et panneaux mécaniques de manière lisible. L'interface n'affiche les conséquences mécaniques **qu'après commit**. Les décisions concernant un PJ (dépenser une ressource rare, accepter une prise de risque significative, confirmer une action ambiguë) doivent pouvoir être confirmées par le propriétaire ; un MJ ne peut pas auto-valider.

## 16. Sécurité, confidentialité, continuité

- **Prompt injection :** textes joueurs, objets, documents ou réponses d'outils restent des données ; ils ne redéfinissent pas les permissions ou règles du système.
- **Mécaniques :** aucune résolution par lecture de nombres dans la prose ; résultats de dés et modifications d'état issus seulement de Rust.
- **Secrets :** filtrage par destinataire, journalisation des accès et tests de non-divulgation ; séparation connaissance MJ/PNJ/joueur.
- **Réseau :** URLs configurables, interdiction des schémas non HTTP(S), gestion TLS/timeout, avertissement pour HTTP distant ; pas de contournement implicite.
- **Erreurs IA :** sortie malformée, modèle indisponible, débit limité ou tool call incohérent → demande corrigée bornée ou action pendante, **aucune mutation partielle**.
- **Crash :** journal/reprise sans second dé ni effet dupliqué ; chaque micro-étape durable reste consultable.
- **Branches :** état, mémoire et promesses propres à la branche ; un fait d'une autre branche n'est pas importé sans opération explicite.
- **Suppression :** confirmation, portée des données, nettoyage des secrets associés, journaux des opérations administratives.

### 16.1 Sécurité d'une campagne sur un poste partagé

Les secrets de MJ et informations individuelles ne doivent pas être affichés dans le panneau commun ni inclus dans les contextes des dialogues accessibles aux autres joueurs. Une vue « diagnostic MJ » ne s'ouvre que sur action explicite, avec masquage par défaut. Les journaux exportés sont expurgés des secrets et des clés API sauf choix explicitement autorisé. Une API distante ne reçoit pas les secrets du monde sans le consentement prévu pour cette campagne et un profil fournisseur adéquat.

Le programme n'exécute jamais du code fourni par les joueurs/LLM, ne transforme pas une chaîne JSON en SQL et ne fait pas de requête réseau vers une URL arbitraire issue d'une proposition de PNJ.


## 17. Tests d'acceptation et critères bloquants

### 17.1 Invariants à 100 %

1. **Zéro mutation canonique** depuis un texte LLM, sans validation d'une proposition.
2. **Zéro double dé** ou double consommation lors d'un retry, crash, timeout, reprise, ou appel répété.
3. **Zéro changement de PJ** sans autorisation du joueur propriétaire, hors conséquence mécanique d'une action déjà validée.
4. **Zéro fuite de secret** sur la suite de tests de permissions et de contexte.
5. **Reconstruction identique** d'une branche depuis le journal et les snapshots : hash d'état comparé hors champs volatils.
6. **Configuration persistante** après redémarrage sans fichiers externes, y compris seuils de mémoire et profils API chiffrés.

### 17.2 Corpus de tests fonctionnels

| Scénario | Attendu |
|---|---|
| Ouvrir une porte ordinaire | Réussite automatique sans jet |
| Tenter de convaincre un garde avec un argument solide | Arbitrage social contextualisé ; pas de domination mentale automatique |
| Proposer un plan inédit de sabotage | UARE décompose les étapes utiles ; jet seulement si enjeux et incertitude |
| Révéler un secret à un PNJ précis | Sa connaissance change ; les autres PNJ ne deviennent pas omniscients |
| Inventer un détail de décor plausible | Proposition validée, fait sourcé, aucun conflit avec la scène |
| Vol d'objet puis réaction différée du propriétaire | Transfert réel, événement planifié, découverte conditionnelle |
| Action sociale au milieu d'un combat | Permise selon phase et économie ; pas de rejet automatique |
| Sort improvisé plus puissant que la fiche | Rejet ou arbitrage générique borné, jamais création de pouvoir gratuit |
| Deux joueurs donnent des ordres contradictoires au même PJ | Demande de clarification au propriétaire ; pas de mutation |
| Compacter 150 tours, puis revoir un PNJ ancien | Relation, souvenir et causalité récupérés depuis SQLite |
| PNJ affirme un mensonge puis découvre la vérité | `claim` et `belief` évoluent ; la vérité du monde reste intacte |
| Reprise après crash juste après un jet | Même résultat, même opération et aucune dépense répétée |
| Provider LLM renvoie une sortie JSON malformée | Rejet formel, retry borné, aucune mutation |
| Changement de fournisseur `llama.cpp` → Albert | Campagne et règles intactes ; compatibilité du nouveau modèle testée |
| Création avec univers complet utilisateur | Aucune réécriture de fait verrouillé sans cause ou validation |
| Partie centrée sur une enquête sans combat | Longue progression possible sans rencontre forcée |

### 17.3 Campagnes de validation du gameplay long

Le référentiel de gameplay DOIT contenir **au moins trois campagnes simulées** de 300 à 500 tours chacune, dont une sans combat forcé, une à plusieurs joueurs et une où un PNJ réapparaît après plus de 200 tours. Chaque corpus contient des actions originales hors catalogue, de longues scènes tranquilles, des changements d'avis, des fausses informations, des abandons de quêtes et des tentatives de retourner l'intrigue contre le MJ.

| Dimension | Mesure et seuil d'acceptation initial |
|---|---|
| Agence des joueurs | **0** parole, décision ou action de PJ inventée par le MJ dans les scénarios contrôlés |
| Continuité du monde | **0** mutation mécanique non validée ; **≥ 98 %** de rappel correct des faits pertinents dans le corpus instrumenté |
| Absence de railroading | **0** obligation de suivre la quête principale / combat pour continuer dans les scénarios de refus explicites |
| PNJ crédibles | **≥ 90 %** de décisions évaluées comme compatibles avec objectif, moyens et connaissances par des évaluateurs humains sur corpus échantillonné |
| Rythme | **≥ 80 %** des scènes échantillonnées jugées adaptées (calme, surprise, durée, intervention) par un panel humain |
| Résultats nuancés | **100 %** des coûts et fenêtres de quasi-réussite convenus avant le tirage ; aucune modification post-dé |
| Routeur LLM | Appels moyens/interaction et p50/p95 de latence publiés séparément pour 4B, 9B et modèle distant ; baisse visée sans perte de validité |
| Absence de fuite | **0** secret non autorisé divulgué dans les corpus de contrôle (sans garantie universelle hors test) |
| Créativité | **≥ 90 %** des actions plausibles inédites reçoivent un arbitrage utilisable et motivé ; refus justifié hors de ce champ |

Un test de résistance supplémentaire DOIT simuler au moins **2 000 étapes de jeu** dans une branche avec sauvegarde/reprise répétées, interruptions d’outils, changements de scène et recherche d’objets historiques ; son objectif est l’intégrité, pas de démontrer à lui seul le plaisir de jouer des centaines d’heures.

Les scores humains exigent une grille stable (1 à 5), évaluateurs multiples, exemples positifs et négatifs et traçabilité de la version du modèle. Les seuils ci-dessus sont des **objectifs à tester**, pas des résultats acquis. Un test peut échouer sans remettre en cause l’intégrité des données : il bloque la qualification du modèle ou la livraison de la fonctionnalité, pas la lecture/sauvegarde des campagnes.

### 17.4 Benchmarks LLM et contexte

Évaluer au moins deux profils locaux de tailles différentes (p. ex. 4B et 9B) et un fournisseur distant si configuré. Mesurer **taux de propositions valides**, **demandes de clarification utiles**, **contradictions factuelles**, **fuites de secret**, **latence**, **tokens par action**, **rappels corrects à 50/150/300 tours** et **part des scènes non violentes**. Les tests doivent distinguer refus légitime d'une action et incapacité du modèle à comprendre une action créative.

Objectifs initiaux : 100 % des invariants mécaniques en tests automatisés ; ≥ 95 % de réussite des contrats sur le corpus de validation du modèle sélectionné ; ≥ 98 % des faits canoniques pertinents correctement récupérés dans le jeu de tests de mémoire ; aucune fuite sur corpus de secrets. Ces objectifs doivent être mesurés et publiés comme résultats réels, pas affirmés a priori.

### 17.5 Scénarios d'acceptation supplémentaires V9.1

| ID | Scénario | Attendu bloquant |
|---|---|---|
| G01 | Joueur ignore la quête pendant 80 tours | Campagne continue ; aucune vengeance automatique ni retour forcé au même arc |
| G02 | Scène sociale de 30 tours sans combat | Dialogue et motivations cohérents ; aucun quota de danger |
| G03 | « Je négocie l'usage du pont, puis je le sabote » | Étapes séparées, conséquences dépendantes, pas de double jet |
| G04 | PNJ réapparaît après 250 tours | But, promesse, croyances et relation retrouvés par sources SQLite |
| G05 | Le MJ décrit un objet utilisable jamais créé | Blocage/correction avant affichage du fait opératoire ; pas de cadeau mécanique fantôme |
| G06 | Le joueur agit sur un détail de décor déjà raconté | Détail retrouvé et arbitré ; pas de négation rétroactive silencieuse |
| G07 | Une source affirme un mensonge, puis le PNJ découvre la vérité | Croyance modifiée ; vérité et déclaration toujours distinguées |
| G08 | Crash après jet, avant proposition de narration | Même dé et mêmes effets après reprise ; narration régénérable |
| G09 | Streaming affirme une réussite avant validation | Aucune affirmation non committée visible au joueur |
| G10 | Résultat coûteux sans consentement | État `pending`, coût non prélevé |
| G11 | Deux initiatives PNJ concurrentes sur un même objet | Conflit détecté par `state_version`, aucune duplication |
| G12 | Contexte réduit par compactage | Rappel des faits essentiels + identifiants sources ; aucune perte canonique |
| G13 | LLM reçoit connaissance MJ lors d'un dialogue PNJ | Aucun fait hors scope dans entrée PNJ ; texte de sortie bloqué s'il contient un secret détectable |
| G14 | Campagne sauvegardée, dupliquée et supprimée | Branches isolées, reprise contrôlée, aucun alias de données mutables |
| G15 | Action improbable mais plausible inconnue du catalogue | Arbitrage UARE motivé ; pas de « commande inconnue » |
| G16 | Modèle retourne JSON valide mais impose paroles d'un PJ | Refus métier, aucune mutation, reformulation ciblée |
| G17 | Latence excessive sur interaction simple | Réduction d'historique, `direct` privilégié si sûr, aucune suppression de validations |
| G18 | Joueur quitte une intrigue à cause d'un choix moral | Monde évolue selon causes persistées, sans railroading compensatoire |

**Gates qualitatifs de qualification :** au moins 3 campagnes de 300–500 tours, évaluées par plusieurs humains selon une grille archivée (agence, qualité des PNJ, rythme, continuité, utilité des dés). Le *plaisir de jeu* ne peut recevoir un score « validé » qu'après des sessions réelles et des retours humains. Les assertions `0 mutation interdite`, `0 double jet` et `0 décision de PJ inventée dans le corpus` restent des exigences bloquantes ; les taux de rappel et d'agrément sont des seuils d'essai à mesurer.


## 18. Ordre d'implémentation et critères de sortie

| Lot | Livrable | Condition pour avancer |
|---|---|---|
| 1. Fondation | SQLite, tables, événements, commandes, idempotence, fiches, dés | Rejeu et crash tests réussis |
| 2. UARE | Intention libre, lecture contexte, arbitrage, tests, effets et narration | Scénarios social, enquête, sabotage et magie générique passent |
| 3. Monde | Canon initial, faits/beliefs, PNJ, calendrier, quêtes | Longue scène persistante sans contradiction d'état |
| 4. IA | `llama.cpp`, API OpenAI-compatibles, `gm.read`/`gm.propose`, tests de capacités | Contrats valides et repli sûr |
| 5. Interface | Campagnes, fiches, inventaire, journal, paramètres et multijoueur local | Parcours de bout en bout, sauvegardes et confirmations |
| 6. Règles Lite | Classes/capacités sélectionnées, sorts, conditions, combat simplifié | Tests de règles couvertes et combat sans duplication |
| 7. Mémoire avancée | Récupération ciblée, résumés, compactage adaptatif, diagnostics | Rappel à long terme, coût en tokens maîtrisé |
| 8. Stabilisation | Corpus narratif, sécurité, crash/reprise, import/export | Gates d'acceptation atteints |

Cette séquence privilégie **la créativité et la cohérence** avant l'exhaustivité d'un catalogue D&D.

## 19. Continuité V8 → V9 et décisions préservées

| Élément V8 | Décision V9 |
|---|---|
| UARE, actions textuelles libres, D&D 2024 Lite | **Conservés** intégralement ; combat non prioritaire |
| `gm.read` / `gm.propose` JSON Schema | **Conservés** ; champs d’enjeux optionnels ajoutés, anciens messages compatibles |
| Canon initial libre/partiel/complet | **Conservé** ; aucun fait imposé réécrit sans cause |
| SQLite source de vérité, event sourcing, branches et sauvegardes | **Conservés** ; ajout d’agrégats narratifs et des profils PNJ |
| Mémoire et contexte JSON compact | **Conservés** ; sélection orientée tâche et budgets optimisés |
| API `llama.cpp` et OpenAI-compatible | **Conservée** ; circuit d’appel adapté aux risques et complexités |
| PNJ réactifs | **Renforcés** ; motivations, limites, plans, connaissances et initiatives temporelles |
| Narration | **Renforcée** ; rythme volontaire, droit au calme, propositions sans railroading |

**La V9.1 remplace les V8 et V9 en tant que spécification normative complète.** Elle n'exige la lecture d'aucune version antérieure. Les anciens protocoles DCP/1 et TOON ne sont pas actifs, pas plus que le moteur D&D 2024 exhaustif abandonné.

### 19.1 Apports obligatoires de la V9

- **Ajout** du directeur narratif fondé sur la causalité, avec capacité de *ne rien déclencher*.
- **Ajout** de circuits d'appel `direct`, `standard`, `deep` et métriques de coût/latence.
- **Ajout** de profils de PNJ et de projets temporels persistants.
- **Ajout** d'enjeux gradués fixés avant les dés.
- **Renforcement** des tests de créativité, de rythme, d'agence et de continuité après plusieurs centaines de tours.
- **Aucun changement** aux deux outils JSON, à SQLite comme stockage unique, à la liberté d'action ni au rôle mécanique de Rust.

## 20. Références, licence et terminologie

- **SRD 5.2.1 officiel et licence CC-BY-4.0 :** https://www.dndbeyond.com/srd — référence du socle 2024. Le produit devra intégrer l'attribution requise par la licence lorsqu'il réutilise du matériel SRD.
- **Documentation `llama.cpp` server :** https://github.com/ggml-org/llama.cpp/blob/master/tools/server/README.md — API de chat et function calling OpenAI-compatible ; le fonctionnement réel dépend des versions du serveur, des templates et des modèles.
- **JSON Schema :** https://json-schema.org/draft/2020-12 — référence des schémas des outils.
- Les noms et contenus non couverts par le SRD ne sont pas implicitement autorisés par son régime Creative Commons. Éviter les contenus commerciaux propriétaires non licenciés ; distinguer les créations originales des adaptations.

---

## 21. Résumé contractuel V9.1

1. **Liberté d'action d'abord.** Le texte libre est la principale interface de jeu ; aucune liste fermée d'actions du joueur.
2. **UARE unique.** Toute action passe par le même moteur d'arbitrage et de résolution ; le combat n'est qu'un cas particulier.
3. **MJ IA créatif, non souverain.** Le modèle interprète, improvise et propose ; Rust valide, lance les dés, modifie l'état et persiste.
4. **D&D 2024 Lite.** Mécaniques reconnaissables, volontairement simplifiées, sans promesse d'exhaustivité officielle.
5. **Monde causalement persistant.** PNJ et factions possèdent mémoire, croyances, objectifs et échéances ; les conséquences restent vérifiables.
6. **SQLite partout.** État, événements, prompts, profils, API keys chiffrées et paramètres de compactage ; aucun fichier externe de configuration.
7. **JSON compact.** Mémoire et contexte limités au nécessaire ; outils en JSON Schema ; capacité 256K facultative et non utilisée par défaut.
8. **Reprise sûre.** Chaque mutation est transactionnelle et auditée ; crash et retries ne relancent pas les dés ni ne dupliquent les effets.
9. **Intelligence du MJ.** Le directeur narratif suit le rythme, les PNJ et les fils sans imposer un scénario ni une fréquence artificielle de rebondissements.
10. **Fluidité mesurée.** Routeur adaptatif direct/standard/approfondi ; appels et tokens minimisés sans sacrifier la validation.
11. **Conséquences créatives.** Résultats nuancés et coûts convenus avant les jets ; répercussions durables et auditables.
12. **Campagnes longues.** Tests d’agence, de crédibilité des PNJ, de rythme, de mémoire et de refus des quêtes sur plusieurs centaines de tours.
13. **Narration et canon distincts.** Les faits opératoires sont validés avant affichage, les perceptions ne deviennent pas automatiquement vraies, les détails de décor déjà racontés ne sont pas niés silencieusement.
14. **Qualification par le jeu.** Aucune note de plaisir ou promesse de centaines d’heures ne remplace des tests de parties réelles et des mesures de qualité.


## Annexe A — Schémas JSON Schema normatifs et tests

Les cinq schémas ci-dessous sont **normatifs** pour la V9. `gm.propose` est une union fermée à six variantes ; `gm.result` est une union discriminée par `status`. Les exemples du document doivent les respecter. Les contrôles métier (droits, ressources, faits, DD, causalité) s'ajoutent aux contrôles de structure.

Les fournisseurs qui ne prennent pas en charge l'intégralité de `oneOf` ou des sous-schémas conditionnels reçoivent une version **compatible simplifiée** de la déclaration de fonction ; **Rust valide toujours la réponse finale avec ces schémas complets**, sans affaiblir sa sécurité. Une réponse non valide ne déclenche aucun événement.

### A.1 `gm.read/1`

```json
{
  "type": "object",
  "additionalProperties": false,
  "required": [
    "kind"
  ],
  "properties": {
    "kind": {
      "type": "string",
      "enum": [
        "scene",
        "actor",
        "inventory",
        "fact",
        "memory",
        "quest",
        "rule",
        "schedule"
      ]
    },
    "ids": {
      "type": "array",
      "items": {
        "type": "string",
        "minLength": 1,
        "maxLength": 128
      },
      "maxItems": 20
    },
    "query": {
      "type": "string",
      "maxLength": 500
    },
    "limit": {
      "type": "integer",
      "minimum": 1,
      "maximum": 20
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "gm.read/1"
}
```

### A.2 `gm.propose/1`

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "gm.propose/1",
  "oneOf": [
    {
      "type": "object",
      "additionalProperties": false,
      "required": [
        "kind",
        "intent_id",
        "basis_state",
        "args"
      ],
      "properties": {
        "kind": {
          "const": "adjudication"
        },
        "intent_id": {
          "type": "string",
          "minLength": 1,
          "maxLength": 128
        },
        "basis_state": {
          "type": "integer",
          "minimum": 0
        },
        "args": {
          "type": "object",
          "additionalProperties": false,
          "required": [
            "decision",
            "reason",
            "stakes",
            "method"
          ],
          "properties": {
            "decision": {
              "type": "string",
              "enum": [
                "automatic_success",
                "automatic_failure",
                "check",
                "contest",
                "clarify",
                "needs_consent",
                "impossible_now"
              ]
            },
            "reason": {
              "type": "string",
              "minLength": 1,
              "maxLength": 2048
            },
            "stakes": {
              "type": "object",
              "additionalProperties": false,
              "required": [
                "success",
                "failure"
              ],
              "properties": {
                "success": {
                  "type": "string",
                  "minLength": 1,
                  "maxLength": 2048
                },
                "failure": {
                  "type": "string",
                  "minLength": 1,
                  "maxLength": 2048
                },
                "success_with_cost": {
                  "type": "string",
                  "minLength": 1,
                  "maxLength": 2048,
                  "description": "Coût optionnel, annoncé et consenti avant application ; jamais rétroactif"
                },
                "failure_with_progress": {
                  "type": "string",
                  "minLength": 1,
                  "maxLength": 2048,
                  "description": "Progrès non équivalent à une réussite, en cas de quasi-échec éligible"
                },
                "failure_with_complication": {
                  "type": "string",
                  "minLength": 1,
                  "maxLength": 2048,
                  "description": "Complication causale déjà annoncée avant le jet"
                },
                "near_miss_window": {
                  "type": "integer",
                  "minimum": 1,
                  "maximum": 3,
                  "description": "Différence maximale strictement sous le DD pour déclencher failure_with_progress"
                }
              },
              "allOf": [
                {
                  "if": {
                    "required": [
                      "near_miss_window"
                    ]
                  },
                  "then": {
                    "required": [
                      "failure_with_progress"
                    ]
                  }
                }
              ]
            },
            "method": {
              "type": "string",
              "minLength": 1,
              "maxLength": 2048
            },
            "ability": {
              "type": "string",
              "enum": [
                "STR",
                "DEX",
                "CON",
                "INT",
                "WIS",
                "CHA"
              ]
            },
            "dc": {
              "type": "integer",
              "minimum": 1,
              "maximum": 35
            },
            "advantage": {
              "type": "string",
              "enum": [
                "none",
                "advantage",
                "disadvantage"
              ]
            },
            "opponent_id": {
              "type": "string",
              "minLength": 1,
              "maxLength": 128
            },
            "question": {
              "type": "string",
              "minLength": 1,
              "maxLength": 2048
            },
            "player_id": {
              "type": "string",
              "minLength": 1,
              "maxLength": 128
            }
          },
          "allOf": [
            {
              "if": {
                "properties": {
                  "decision": {
                    "const": "check"
                  }
                },
                "required": [
                  "decision"
                ]
              },
              "then": {
                "required": [
                  "ability",
                  "dc",
                  "advantage"
                ]
              }
            },
            {
              "if": {
                "properties": {
                  "decision": {
                    "const": "contest"
                  }
                },
                "required": [
                  "decision"
                ]
              },
              "then": {
                "required": [
                  "ability",
                  "opponent_id"
                ]
              }
            },
            {
              "if": {
                "properties": {
                  "decision": {
                    "enum": [
                      "clarify",
                      "needs_consent"
                    ]
                  }
                },
                "required": [
                  "decision"
                ]
              },
              "then": {
                "required": [
                  "question"
                ]
              }
            },
            {
              "if": {
                "properties": {
                  "decision": {
                    "const": "needs_consent"
                  }
                },
                "required": [
                  "decision"
                ]
              },
              "then": {
                "required": [
                  "player_id"
                ]
              }
            }
          ]
        }
      }
    },
    {
      "type": "object",
      "additionalProperties": false,
      "required": [
        "kind",
        "intent_id",
        "basis_state",
        "args"
      ],
      "properties": {
        "kind": {
          "const": "npc_intent"
        },
        "intent_id": {
          "type": "string",
          "minLength": 1,
          "maxLength": 128
        },
        "basis_state": {
          "type": "integer",
          "minimum": 0
        },
        "args": {
          "type": "object",
          "additionalProperties": false,
          "required": [
            "actor_id",
            "goal",
            "method",
            "target_ids"
          ],
          "properties": {
            "actor_id": {
              "type": "string",
              "minLength": 1,
              "maxLength": 128
            },
            "goal": {
              "type": "string",
              "minLength": 1,
              "maxLength": 2048
            },
            "method": {
              "type": "string",
              "minLength": 1,
              "maxLength": 2048
            },
            "target_ids": {
              "type": "array",
              "items": {
                "type": "string",
                "minLength": 1,
                "maxLength": 128
              },
              "maxItems": 16
            }
          }
        }
      }
    },
    {
      "type": "object",
      "additionalProperties": false,
      "required": [
        "kind",
        "intent_id",
        "basis_state",
        "args"
      ],
      "properties": {
        "kind": {
          "const": "outcome"
        },
        "intent_id": {
          "type": "string",
          "minLength": 1,
          "maxLength": 128
        },
        "basis_state": {
          "type": "integer",
          "minimum": 0
        },
        "args": {
          "type": "object",
          "additionalProperties": false,
          "required": [
            "resolution_id",
            "effect_proposals",
            "narrative_hint"
          ],
          "properties": {
            "resolution_id": {
              "type": "string",
              "minLength": 1,
              "maxLength": 128
            },
            "effect_proposals": {
              "type": "array",
              "items": {
                "type": "object",
                "additionalProperties": false,
                "required": [
                  "kind",
                  "subject_id",
                  "summary"
                ],
                "properties": {
                  "kind": {
                    "type": "string",
                    "enum": [
                      "environment_change",
                      "relation_change",
                      "claim_record",
                      "belief_change",
                      "quest_progress",
                      "item_use",
                      "status_change",
                      "schedule_hint",
                      "narrative_detail"
                    ]
                  },
                  "subject_id": {
                    "type": "string",
                    "minLength": 1,
                    "maxLength": 128
                  },
                  "summary": {
                    "type": "string",
                    "minLength": 1,
                    "maxLength": 2048
                  },
                  "evidence_event_ids": {
                    "type": "array",
                    "items": {
                      "type": "string",
                      "minLength": 1,
                      "maxLength": 128
                    },
                    "maxItems": 16
                  },
                  "precondition_fact_ids": {
                    "type": "array",
                    "items": {
                      "type": "string",
                      "minLength": 1,
                      "maxLength": 128
                    },
                    "maxItems": 16
                  }
                }
              },
              "maxItems": 32
            },
            "narrative_hint": {
              "type": "string",
              "minLength": 1,
              "maxLength": 2048
            }
          }
        }
      }
    },
    {
      "type": "object",
      "additionalProperties": false,
      "required": [
        "kind",
        "intent_id",
        "basis_state",
        "args"
      ],
      "properties": {
        "kind": {
          "const": "world_event"
        },
        "intent_id": {
          "type": "string",
          "minLength": 1,
          "maxLength": 128
        },
        "basis_state": {
          "type": "integer",
          "minimum": 0
        },
        "args": {
          "type": "object",
          "additionalProperties": false,
          "required": [
            "event_type",
            "subject_ids",
            "preconditions",
            "proposed_change"
          ],
          "properties": {
            "event_type": {
              "type": "string",
              "minLength": 1,
              "maxLength": 128
            },
            "subject_ids": {
              "type": "array",
              "items": {
                "type": "string",
                "minLength": 1,
                "maxLength": 128
              },
              "maxItems": 16
            },
            "preconditions": {
              "type": "array",
              "items": {
                "type": "object",
                "additionalProperties": false,
                "required": [
                  "fact_id",
                  "expected_state"
                ],
                "properties": {
                  "fact_id": {
                    "type": "string",
                    "minLength": 1,
                    "maxLength": 128
                  },
                  "expected_state": {
                    "type": [
                      "string",
                      "boolean",
                      "number"
                    ]
                  }
                }
              },
              "maxItems": 16
            },
            "proposed_change": {
              "type": "object",
              "additionalProperties": false,
              "required": [
                "kind",
                "subject_id",
                "summary"
              ],
              "properties": {
                "kind": {
                  "type": "string",
                  "enum": [
                    "fact_proposal",
                    "relationship_proposal",
                    "quest_proposal",
                    "world_entity_proposal",
                    "scene_proposal"
                  ]
                },
                "subject_id": {
                  "type": "string",
                  "minLength": 1,
                  "maxLength": 128
                },
                "summary": {
                  "type": "string",
                  "minLength": 1,
                  "maxLength": 2048
                }
              }
            }
          }
        }
      }
    },
    {
      "type": "object",
      "additionalProperties": false,
      "required": [
        "kind",
        "intent_id",
        "basis_state",
        "args"
      ],
      "properties": {
        "kind": {
          "const": "schedule"
        },
        "intent_id": {
          "type": "string",
          "minLength": 1,
          "maxLength": 128
        },
        "basis_state": {
          "type": "integer",
          "minimum": 0
        },
        "args": {
          "type": "object",
          "additionalProperties": false,
          "required": [
            "trigger",
            "condition",
            "proposed_event",
            "visibility"
          ],
          "properties": {
            "trigger": {
              "oneOf": [
                {
                  "type": "object",
                  "additionalProperties": false,
                  "required": [
                    "type",
                    "tick"
                  ],
                  "properties": {
                    "type": {
                      "const": "time_at"
                    },
                    "tick": {
                      "type": "integer",
                      "minimum": 0
                    }
                  }
                },
                {
                  "type": "object",
                  "additionalProperties": false,
                  "required": [
                    "type",
                    "delta"
                  ],
                  "properties": {
                    "type": {
                      "const": "after_duration"
                    },
                    "delta": {
                      "type": "integer",
                      "minimum": 1
                    }
                  }
                },
                {
                  "type": "object",
                  "additionalProperties": false,
                  "required": [
                    "type",
                    "fact_id",
                    "equals"
                  ],
                  "properties": {
                    "type": {
                      "const": "fact_becomes"
                    },
                    "fact_id": {
                      "type": "string",
                      "minLength": 1,
                      "maxLength": 128
                    },
                    "equals": {
                      "type": [
                        "string",
                        "boolean",
                        "number"
                      ]
                    }
                  }
                },
                {
                  "type": "object",
                  "additionalProperties": false,
                  "required": [
                    "type",
                    "actor_id",
                    "place_id"
                  ],
                  "properties": {
                    "type": {
                      "const": "actor_enters_place"
                    },
                    "actor_id": {
                      "type": "string",
                      "minLength": 1,
                      "maxLength": 128
                    },
                    "place_id": {
                      "type": "string",
                      "minLength": 1,
                      "maxLength": 128
                    }
                  }
                },
                {
                  "type": "object",
                  "additionalProperties": false,
                  "required": [
                    "type",
                    "item_id",
                    "new_owner_id"
                  ],
                  "properties": {
                    "type": {
                      "const": "item_owner_changes"
                    },
                    "item_id": {
                      "type": "string",
                      "minLength": 1,
                      "maxLength": 128
                    },
                    "new_owner_id": {
                      "type": "string",
                      "minLength": 1,
                      "maxLength": 128
                    }
                  }
                },
                {
                  "type": "object",
                  "additionalProperties": false,
                  "required": [
                    "type",
                    "quest_id",
                    "state"
                  ],
                  "properties": {
                    "type": {
                      "const": "quest_state"
                    },
                    "quest_id": {
                      "type": "string",
                      "minLength": 1,
                      "maxLength": 128
                    },
                    "state": {
                      "type": "string",
                      "minLength": 1,
                      "maxLength": 2048
                    }
                  }
                },
                {
                  "type": "object",
                  "additionalProperties": false,
                  "required": [
                    "type",
                    "event_id"
                  ],
                  "properties": {
                    "type": {
                      "const": "explicit_event"
                    },
                    "event_id": {
                      "type": "string",
                      "minLength": 1,
                      "maxLength": 128
                    }
                  }
                }
              ]
            },
            "condition": {
              "type": "object",
              "additionalProperties": false,
              "required": [
                "fact_id",
                "expected_state"
              ],
              "properties": {
                "fact_id": {
                  "type": "string",
                  "minLength": 1,
                  "maxLength": 128
                },
                "expected_state": {
                  "type": [
                    "string",
                    "boolean",
                    "number"
                  ]
                }
              }
            },
            "proposed_event": {
              "type": "object",
              "additionalProperties": false,
              "required": [
                "type",
                "summary"
              ],
              "properties": {
                "type": {
                  "type": "string",
                  "minLength": 1,
                  "maxLength": 128
                },
                "actor_id": {
                  "type": "string",
                  "minLength": 1,
                  "maxLength": 128
                },
                "object_id": {
                  "type": "string",
                  "minLength": 1,
                  "maxLength": 128
                },
                "summary": {
                  "type": "string",
                  "minLength": 1,
                  "maxLength": 2048
                }
              }
            },
            "visibility": {
              "type": "string",
              "enum": [
                "public",
                "party",
                "gm"
              ]
            }
          }
        }
      }
    },
    {
      "type": "object",
      "additionalProperties": false,
      "required": [
        "kind",
        "intent_id",
        "basis_state",
        "args"
      ],
      "properties": {
        "kind": {
          "const": "ask_player"
        },
        "intent_id": {
          "type": "string",
          "minLength": 1,
          "maxLength": 128
        },
        "basis_state": {
          "type": "integer",
          "minimum": 0
        },
        "args": {
          "type": "object",
          "additionalProperties": false,
          "required": [
            "player_id",
            "question",
            "reason"
          ],
          "properties": {
            "player_id": {
              "type": "string",
              "minLength": 1,
              "maxLength": 128
            },
            "question": {
              "type": "string",
              "minLength": 1,
              "maxLength": 2048
            },
            "reason": {
              "type": "string",
              "minLength": 1,
              "maxLength": 2048
            },
            "choices_optional": {
              "type": "array",
              "items": {
                "type": "string",
                "minLength": 1,
                "maxLength": 2048
              },
              "maxItems": 8
            }
          }
        }
      }
    }
  ]
}
```

### A.3 `gm.result/1`

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "gm.result/1",
  "oneOf": [
    {
      "type": "object",
      "additionalProperties": false,
      "required": [
        "status",
        "proposal_id",
        "operation_id",
        "state_version",
        "events"
      ],
      "properties": {
        "status": {
          "const": "accepted"
        },
        "proposal_id": {
          "type": "string",
          "minLength": 1,
          "maxLength": 128
        },
        "operation_id": {
          "type": "string",
          "minLength": 1,
          "maxLength": 128
        },
        "state_version": {
          "type": "integer",
          "minimum": 0
        },
        "events": {
          "type": "array",
          "items": {
            "type": "object",
            "additionalProperties": false,
            "required": [
              "event_id",
              "type",
              "seq"
            ],
            "properties": {
              "event_id": {
                "type": "string",
                "minLength": 1,
                "maxLength": 128
              },
              "type": {
                "type": "string",
                "minLength": 1,
                "maxLength": 128
              },
              "seq": {
                "type": "integer",
                "minimum": 1
              }
            }
          },
          "maxItems": 64
        }
      }
    },
    {
      "type": "object",
      "additionalProperties": false,
      "required": [
        "status",
        "proposal_id",
        "operation_id",
        "state_version",
        "awaiting",
        "prompt"
      ],
      "properties": {
        "status": {
          "const": "pending"
        },
        "proposal_id": {
          "type": "string",
          "minLength": 1,
          "maxLength": 128
        },
        "operation_id": {
          "type": "string",
          "minLength": 1,
          "maxLength": 128
        },
        "state_version": {
          "type": "integer",
          "minimum": 0
        },
        "awaiting": {
          "type": "object",
          "additionalProperties": false,
          "required": [
            "player_id",
            "reason",
            "resolution_id"
          ],
          "properties": {
            "player_id": {
              "type": "string",
              "minLength": 1,
              "maxLength": 128
            },
            "reason": {
              "type": "string",
              "minLength": 1,
              "maxLength": 128
            },
            "resolution_id": {
              "type": "string",
              "minLength": 1,
              "maxLength": 128
            }
          }
        },
        "prompt": {
          "type": "string",
          "minLength": 1,
          "maxLength": 2048
        }
      }
    },
    {
      "type": "object",
      "additionalProperties": false,
      "required": [
        "status",
        "proposal_id",
        "code",
        "detail",
        "state_version",
        "events"
      ],
      "properties": {
        "status": {
          "const": "rejected"
        },
        "proposal_id": {
          "type": "string",
          "minLength": 1,
          "maxLength": 128
        },
        "code": {
          "type": "string",
          "enum": [
            "INVALID_SCHEMA",
            "INVALID_ARGUMENT",
            "STALE_STATE",
            "FORBIDDEN",
            "UNKNOWN_TARGET",
            "RULE_CONFLICT",
            "INSUFFICIENT_RESOURCE",
            "NEEDS_CLARIFICATION",
            "NOT_SUPPORTED",
            "TOO_LARGE",
            "PROVIDER_UNAVAILABLE",
            "RETRY_LIMIT"
          ]
        },
        "detail": {
          "type": "string",
          "minLength": 1,
          "maxLength": 2048
        },
        "state_version": {
          "type": "integer",
          "minimum": 0
        },
        "events": {
          "type": "array",
          "maxItems": 0
        }
      }
    },
    {
      "type": "object",
      "additionalProperties": false,
      "required": [
        "status",
        "proposal_id",
        "state_version",
        "data"
      ],
      "properties": {
        "status": {
          "const": "read_only"
        },
        "proposal_id": {
          "type": "string",
          "minLength": 1,
          "maxLength": 128
        },
        "state_version": {
          "type": "integer",
          "minimum": 0
        },
        "data": {
          "type": "object"
        }
      }
    }
  ]
}
```

### A.4 `canonical.event/1`

```json
{
  "type": "object",
  "additionalProperties": false,
  "required": [
    "event_id",
    "branch_id",
    "seq",
    "tx_id",
    "command_id",
    "type",
    "payload_version",
    "payload",
    "visibility",
    "game_time"
  ],
  "properties": {
    "event_id": {
      "type": "string",
      "minLength": 1,
      "maxLength": 128
    },
    "branch_id": {
      "type": "string",
      "minLength": 1,
      "maxLength": 128
    },
    "seq": {
      "type": "integer",
      "minimum": 1
    },
    "tx_id": {
      "type": "string",
      "minLength": 1,
      "maxLength": 128
    },
    "command_id": {
      "type": "string",
      "minLength": 1,
      "maxLength": 128
    },
    "type": {
      "type": "string",
      "minLength": 1,
      "maxLength": 128
    },
    "payload_version": {
      "type": "integer",
      "minimum": 1
    },
    "payload": {
      "type": "object"
    },
    "visibility": {
      "type": "string",
      "enum": [
        "public",
        "party",
        "gm",
        "character"
      ]
    },
    "game_time": {
      "type": "integer",
      "minimum": 0
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "canonical.event/1"
}
```

### A.5 `dice.roll/1`

```json
{
  "type": "object",
  "additionalProperties": false,
  "required": [
    "roll_id",
    "resolution_id",
    "expression",
    "raw_dice",
    "kept_indices",
    "discarded_indices",
    "bonus",
    "total",
    "reason"
  ],
  "properties": {
    "roll_id": {
      "type": "string",
      "minLength": 1,
      "maxLength": 128
    },
    "resolution_id": {
      "type": "string",
      "minLength": 1,
      "maxLength": 128
    },
    "expression": {
      "type": "string",
      "minLength": 1,
      "maxLength": 2048
    },
    "raw_dice": {
      "type": "array",
      "items": {
        "type": "integer",
        "minimum": 1
      },
      "maxItems": 100
    },
    "kept_indices": {
      "type": "array",
      "items": {
        "type": "integer",
        "minimum": 0
      },
      "maxItems": 100
    },
    "discarded_indices": {
      "type": "array",
      "items": {
        "type": "integer",
        "minimum": 0
      },
      "maxItems": 100
    },
    "bonus": {
      "type": "integer"
    },
    "total": {
      "type": "integer"
    },
    "reason": {
      "type": "string",
      "minLength": 1,
      "maxLength": 2048
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "dice.roll/1"
}
```

### A.6 Contrats de charges utiles d'événements

Les événements partagent `canonical.event/1`. Le champ `payload` est validé par un schéma métier **fermé et versionné** dépendant de `type` ; le schéma d'enveloppe seul n'autorise jamais une mutation. Registre minimal :

| `type` | Champs obligatoires de `payload` | Contrôle de rejeu |
|---|---|---|
| `IntentReceived` | `intent_id`, `actor_id`, `text_ref` | Création unique d'une intention |
| `AdjudicationCommitted` | `intent_id`, `decision`, `stakes_ref`, `dc_or_null` | Référence aux enjeux fixés avant jet |
| `RollCommitted` | `roll_id`, `resolution_id` | Renvoi vers enregistrement `dice.roll/1` immuable |
| `EffectCommitted` | `resolution_id`, `effect_kind`, `subject_id`, `before`, `after` | Réapplique l'effet typé et vérifie les préconditions |
| `ItemTransferred` | `item_instance_id`, `from_actor_id`, `to_actor_id`, `quantity` | Ownership et quantités vérifiés |
| `FactVersioned` | `fact_id`, `old_valid_to`, `new_fact_version`, `source_id` | Historique temporel conservé |
| `ClaimRecorded` | `claim_id`, `speaker_id`, `content_ref`, `listener_ids` | Ne crée pas de vérité canonique |
| `BeliefUpdated` | `holder_id`, `belief_id`, `basis_ids`, `status` | Isolé des connaissances des autres PNJ |
| `TimeAdvanced` | `before_tick`, `after_tick`, `cause_id` | Progression monotone sur la branche |
| `ScheduledEventCreated` | `schedule_id`, `trigger`, `preconditions` | Déclencheur exact, aucune exécution prématurée |
| `ResolutionSuspended` | `resolution_id`, `awaiting`, `continuation_ref` | Reprise sans relancer les étapes passées |
| `CampaignInitialized` | `campaign_id`, `branch_id`, `initial_snapshot_id` | Un seul événement initial par branche racine |

Chaque événement a un `payload_version`, un sérialiseur et un rejeueur Rust versionnés. Les événements historiques d’une base dont le schéma SQLite est pris en charge restent lisibles et rejouables ; cela n’implique **aucune migration** d’une ancienne base incompatible. Le journal est la référence ; un snapshot ne peut écraser des événements divergents.

### A.7 Tests contractuels bloquants

- **Valide :** proposition `adjudication` avec `decision=check`, `ability`, `dc`, `advantage` et enjeux.
- **Valide :** déclencheur `time_at` et événement futur doté de `summary`.
- **Valide :** réponse `pending` avec `awaiting.player_id`, `reason` et `resolution_id`.
- **Invalide :** `accepted` contenant `code=FORBIDDEN`.
- **Invalide :** `pending` sans `awaiting`.
- **Invalide :** `rejected` contenant un événement de dégât.
- **Invalide :** `adjudication:check` sans `dc` ou `advantage`.
- **Invalide métier :** une proposition JSON valide réécrivant le canon, déplaçant le PJ sans autorisation ou modifiant un fait inaccessible.
- **Rejeu :** `RollCommitted` + `EffectCommitted` + `ResolutionSuspended` doit se reconstruire bit-à-bit au niveau métier après crash, sans second dé.


---

## Annexe A.8 — Événements typés supplémentaires et invariants de rejeu

Les contrats d'enveloppe de l'annexe A ne suffisent pas à valider le métier. Les charges utiles des événements critiques doivent être fermées et versionnées. **Au lancement**, les événements de dés et de conséquences doivent être validés par les schémas de référence suivants (exemples de sous-ensemble obligatoire, pas la liste de tous les événements) :

```json
{
  "$schema":"https://json-schema.org/draft/2020-12/schema",
  "title":"event.effect-committed/1",
  "type":"object",
  "additionalProperties":false,
  "required":["resolution_id","effect_id","effect_kind","subject_id","before","after","source_ids"],
  "properties":{
    "resolution_id":{"type":"string","minLength":1},
    "effect_id":{"type":"string","minLength":1},
    "effect_kind":{"enum":["hp","resource","condition","ownership","location","relation","fact","quest","time"]},
    "subject_id":{"type":"string","minLength":1},
    "before":{"type":["object","null"]},
    "after":{"type":["object","null"]},
    "source_ids":{"type":"array","items":{"type":"string","minLength":1},"minItems":1,"uniqueItems":true}
  }
}
```

```json
{
  "$schema":"https://json-schema.org/draft/2020-12/schema",
  "title":"event.resolution-suspended/1",
  "type":"object",
  "additionalProperties":false,
  "required":["resolution_id","reason","awaiting_player_id","state_version","continuation_ref"],
  "properties":{
    "resolution_id":{"type":"string","minLength":1},
    "reason":{"enum":["choice","reaction","consent","clarification","provider_unavailable"]},
    "awaiting_player_id":{"type":["string","null"]},
    "state_version":{"type":"integer","minimum":0},
    "continuation_ref":{"type":"string","minLength":1}
  },
  "allOf":[{"if":{"properties":{"reason":{"enum":["choice","reaction","consent","clarification"]}},"required":["reason"]},"then":{"properties":{"awaiting_player_id":{"type":"string","minLength":1}}}}]
}
```

**Invariants métier complémentaires :** `before` et `after` respectent le type de `effect_kind`, les PV restent dans les bornes autorisées, une propriété d'objet n'a jamais deux propriétaires simultanés sur la même branche, une suspension pour réaction n'est pas consommée deux fois, et tout rejeu depuis la même séquence conduit au même état et au même hash. Les validateurs d'effets spécialisés sont obligatoires avant livraison ; le JSON Schema de base seul n'est pas une preuve de correction.


## Annexe B — Contrats V9.1, compatibilité et cas de conformité

### B.1 Enjeux gradués : validation et causalité

Le contrat `gm.propose/1` d'annexe A.2 est étendu avec **quatre champs optionnels** de `args.stakes` : `success_with_cost`, `failure_with_progress`, `failure_with_complication`, `near_miss_window`. **Compatibilité ascendante :** les anciens messages V8 ne contenant que `success` et `failure` restent valides. Le schéma interdit `near_miss_window` sans `failure_with_progress`. Rust valide en plus les coûts, l’accord préalable du joueur, le type de test et les contraintes mécaniques.

```json
{"kind":"adjudication","intent_id":"A99","basis_state":84,"args":{"decision":"check","reason":"Le scribe cache une information mais hésite","method":"négocier un accès aux archives","ability":"CHA","dc":14,"advantage":"none","stakes":{"success":"accès complet","failure":"accès refusé","failure_with_progress":"le scribe révèle un nom","near_miss_window":2,"success_with_cost":"accès après dépôt volontaire de 5 pièces d'or"}}}
```

Conditions bloquantes : aucun jet ne précède le commit de ces enjeux ; une relance conserve le même `resolution_id` ; `outcome` ne peut produire de succès complet sur un jet raté, sauf nouvel arbitrage explicite avec accord du joueur **sans réécriture du dé**.

### B.2 Profils PNJ (données SQLite, non outil supplémentaire)

Le schéma ci-dessous définit une projection persistante minimale, extensible dans les schémas des nouvelles bases créées par les versions ultérieures de l’application, **sans conversion des bases SQLite anciennes**. Les secrets d’un personnage sont filtrés à la lecture.

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "npc.profile/1",
  "type": "object",
  "additionalProperties": false,
  "required": [
    "npc_id",
    "values",
    "drives",
    "fears",
    "boundaries",
    "voice",
    "knowledge_scope",
    "last_change_event",
    "relationships",
    "goals",
    "agency_level"
  ],
  "properties": {
    "npc_id": {
      "type": "string",
      "minLength": 1
    },
    "values": {
      "type": "array",
      "items": {
        "type": "string",
        "maxLength": 100
      },
      "maxItems": 5
    },
    "drives": {
      "type": "array",
      "items": {
        "type": "string",
        "maxLength": 200
      },
      "maxItems": 4
    },
    "fears": {
      "type": "array",
      "items": {
        "type": "string",
        "maxLength": 120
      },
      "maxItems": 4
    },
    "boundaries": {
      "type": "array",
      "items": {
        "type": "string",
        "maxLength": 200
      },
      "maxItems": 4
    },
    "voice": {
      "type": "string",
      "maxLength": 240
    },
    "knowledge_scope": {
      "type": "array",
      "items": {
        "type": "string"
      },
      "maxItems": 64
    },
    "last_change_event": {
      "type": "string"
    },
    "relationships": {
      "type": "array",
      "items": {
        "type": "object",
        "additionalProperties": false,
        "required": [
          "actor_id",
          "attitude"
        ],
        "properties": {
          "actor_id": {
            "type": "string",
            "minLength": 1
          },
          "attitude": {
            "type": "string",
            "maxLength": 120
          },
          "source_event_id": {
            "type": "string"
          }
        }
      },
      "maxItems": 32
    },
    "goals": {
      "type": "array",
      "items": {
        "type": "string",
        "maxLength": 200
      },
      "maxItems": 6
    },
    "agency_level": {
      "type": "string",
      "enum": [
        "reactive",
        "normal",
        "proactive"
      ]
    }
  }
}
```

### B.3 Plan de direction (données internes, sans mutation immédiate)

`narrative_direction_log` contient : `direction_id`, `campaign_id`, `branch_id`, `scene_id`, `state_version`, `decision` (`no_change|suggest_npc|suggest_event|offer_ellipsis|await_player`), `reason_event_ids`, `candidate_ids`, `risk_to_agency`, `approved`, `created_game_tick`. Une suggestion ne crée jamais d'événement canonique sans validation via `gm.propose` et UARE. Le cas `no_change` est **un résultat normal**.

En cas de changement de branche ou de version d'état, une suggestion pendante devient `STALE_STATE` ; elle ne peut pas être appliquée silencieusement.

**Schéma interne normatif du journal de direction :**

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "narrative.direction/1",
  "type": "object",
  "additionalProperties": false,
  "required": [
    "direction_id",
    "campaign_id",
    "branch_id",
    "scene_id",
    "state_version",
    "decision",
    "reason_event_ids",
    "candidate_ids",
    "risk_to_agency",
    "approved",
    "created_game_tick"
  ],
  "properties": {
    "direction_id": {
      "type": "string",
      "minLength": 1
    },
    "campaign_id": {
      "type": "string",
      "minLength": 1
    },
    "branch_id": {
      "type": "string",
      "minLength": 1
    },
    "scene_id": {
      "type": "string",
      "minLength": 1
    },
    "state_version": {
      "type": "integer",
      "minimum": 0
    },
    "decision": {
      "enum": [
        "no_change",
        "suggest_npc",
        "suggest_event",
        "offer_ellipsis",
        "await_player"
      ]
    },
    "reason_event_ids": {
      "type": "array",
      "items": {
        "type": "string"
      },
      "maxItems": 32
    },
    "candidate_ids": {
      "type": "array",
      "items": {
        "type": "string"
      },
      "maxItems": 20
    },
    "risk_to_agency": {
      "enum": [
        "none",
        "low",
        "high"
      ]
    },
    "approved": {
      "type": "boolean"
    },
    "created_game_tick": {
      "type": "integer",
      "minimum": 0
    }
  },
  "allOf": [
    {
      "if": {
        "properties": {
          "decision": {
            "enum": [
              "suggest_npc",
              "suggest_event"
            ]
          }
        },
        "required": [
          "decision"
        ]
      },
      "then": {
        "properties": {
          "candidate_ids": {
            "minItems": 1
          }
        }
      }
    }
  ]
}
```

### B.4 Matrice de tests minimum V9

| ID | Entrée / situation | Attendu vérifiable |
|---|---|---|
| `PACE-01` | Dialogue calme de 25 tours sans menace | Aucune quête/attaque injectée par obligation de variété |
| `PACE-02` | Joueur demande ellipse de voyage | Temps et événements conditionnels validés ; aucun choix pertinent effacé |
| `AGENCY-01` | Joueurs abandonnent définitivement une mission | Fil `abandoned`, nouveaux horizons accessibles sans retour forcé à la mission |
| `AGENCY-02` | PJ refuse de répondre à une question | Aucune parole ou décision fabriquée à sa place |
| `NPC-01` | PNJ doit garder le secret de sa sœur | Profil et connaissances consultés ; secret non révélé sans cause |
| `NPC-02` | PNJ découvre preuve d'une trahison 220 tours après | Croyance/objectif mis à jour avec `source_event_id`, pas omniscience |
| `UARE-01` | DD 14, jet 12, marge précommittée 2 | Échec avec progrès autorisé, sans réussite totale |
| `UARE-02` | Même jet 12 sans enjeu graduel préalable | Échec normal ; aucun nouveau coût improvisé |
| `FLOW-01` | Saluer un PNJ | Circuit direct, pas de chaîne inutile d'outils |
| `FLOW-02` | Sabotage à plusieurs étapes avec réaction | Circuit approfondi, choix et jets persistants, reprise après crash |
| `WORLD-01` | Faction prépare une enquête mais manque d'indices | Horloge n'avance pas sans déclencheur/ressource ; pas d'événement arbitraire |
| `SEC-01` | Vue joueur sur profil PNJ secret | Champs privés exclus du contexte et des résultats visibles |
| `STATE-01` | Reprise après 500 tours et retour d'un objet | Même détenteur, historique de transfert sourcé et aucune duplication |

### B.5 Conformité et limites

La validation JSON Schema garantit la forme, **pas** la cohérence narrative. Chaque décision exige une validation métier Rust des identifiants, du contexte, des permissions, des effets et de la version de branche. Les tests de qualité du MJ sont représentatifs mais ne garantissent pas tous les comportements d'un modèle réel. Un modèle qui échoue aux seuils reste consultable pour de la prose non canonique, sans droit d'exécuter des mutations.

La V9 ne garantit pas une latence donnée sur tous les matériels ou fournisseurs ; les profils 4B et 9B doivent être mesurés, puis adaptés. Elle ne promet pas non plus un jeu D&D 2024 officiellement complet : c'est **D&D 2024 Lite**.


---

## Annexe C — Gel de conception et démarrage du développement (V9.1-STABLE)

### C.1 Décisions figées

| Sujet | Décision normative |
|---|---|
| Nature | Application **bureau locale** en Rust, avec interface graphique ; un seul processus applicatif pilote le jeu. `llama.cpp` peut fonctionner comme processus serveur indépendant. |
| Jeu | **Actions textuelles entièrement libres**, résolution universelle UARE, D&D 2024 Lite ; aucune obligation de combat ou de quête. |
| Autorité | Rust résout les règles, dés et changements ; MJ IA propose, interprète et raconte, jamais d'écriture directe. |
| Stockage | SQLite **unique source persistante** pour campagnes, réglages, prompts, profils IA, états et traces ; aucun fichier de configuration externe. Le schéma de chaque base doit être compatible avec celui attendu par le binaire ; **aucune migration des anciennes bases**. Les fichiers GGUF, WAL/SHM, exports explicites et ressources embarquées sont des exceptions non configurantes. |
| Mémoire | Faits, déclarations, croyances et connaissances séparés ; compactage non destructif ; contexte **JSON compact**, orienté tâche et borné ; 256K facultatif. |
| LLM | Chat OpenAI-compatible ; `llama.cpp` local par défaut, Albert API et autres profils configurables ; seulement `gm.read` et `gm.propose`, validés par schéma **et** métier. |
| Longévité | Commit à chaque micro-étape significative, journal rejouable, idempotence, reprise exacte après crash et branchement des sauvegardes. |
| Joueurs | 1–6 joueurs locaux dans le produit cible ; identité et permissions par joueur, pas de contrôle implicite des PJ par l'IA. |
| Sécurité | Clés API chiffrées dans SQLite ; secret de déverrouillage non stocké, jamais dans les traces ou dans un prompt. |

**Aucune nouvelle architecture, aucun format personnalisé, aucun nouvel agent et aucun nouveau moteur de règles ne sont requis pour commencer.**

### C.2 Choix techniques initiaux d'implémentation

- **UI :** `eframe/egui` pour une application desktop Rust native, sans navigateur à maintenir. Choix figé pour la première version jouable ; tout remplacement futur exige une décision d'architecture documentée.
- **Persistant :** `rusqlite` avec SQLite, `PRAGMA foreign_keys=ON`, schéma SQL de création embarqué dans le binaire et vérification stricte de sa version à l’ouverture, transactions courtes et mode WAL lorsque compatible avec l’environnement. **Aucune migration SQLite** : tout schéma incompatible est refusé sans écriture ni suppression des données. Un **seul coordinateur d’écriture** ordonne les commandes.
- **HTTP LLM :** `reqwest` et `tokio` pour les opérations asynchrones ; sérialisation `serde`/`serde_json` ; validation JSON Schema côté Rust, indépendamment du serveur de génération.
- **Sécurité :** chiffrement authentifié des secrets (p. ex. XChaCha20-Poly1305) et dérivation de clé depuis une phrase maîtresse (p. ex. Argon2id), avec salt, nonce et métadonnées dans SQLite. Ne jamais enregistrer la phrase maîtresse.
- **Journal :** `tracing` avec masquage des secrets ; les traces persistantes dans SQLite évitent de créer un fichier de configuration. Un journal de diagnostic externe est un export volontaire, désactivé par défaut.
- **Dépendances :** utiliser des versions **verrouillées dans `Cargo.lock`** au développement ; le lockfile de Cargo est un artefact de construction, **pas** un fichier de configuration utilisateur. Les numéros exacts sont sélectionnés lors de la création du dépôt, selon compatibilité compilateur et audits de sécurité.

Ces choix n'impliquent ni connexion réseau à l'ouverture, ni démarrage automatique du modèle. L'utilisateur configure et teste le fournisseur depuis l'interface ; la consultation hors ligne reste disponible.

### C.3 Périmètre de la première tranche *jouable* (vertical slice)

Un premier prototype jouable DOIT permettre, de bout en bout :

1. Créer une campagne **libre, partielle ou complète**, avec un nombre de joueurs déclaré, un contexte initial éditable et une validation des éléments ajoutés par l'IA.
2. Créer au moins un PJ et afficher sa fiche (six caractéristiques, compétences pertinentes, PV, inventaire et historique). Les autres joueurs et fiches entrent dans le même modèle de données.
3. Configurer un profil `llama.cpp` depuis l'interface, sélectionner un modèle et effectuer un **test réel** de chat et de proposition structurée ; une panne laisse la campagne consultable.
4. Jouer une action **libre** (dialogue, enquête, manipulation ou risque), avec un arbitrage UARE, un jet si nécessaire et une réponse narrative cohérente avec les résultats enregistrés.
5. Conserver au minimum un lieu, **trois PNJ différenciés**, leurs connaissances/relations, un objet transférable et un événement différé vérifiable.
6. Sauvegarder automatiquement après chaque mutation validée ; fermer au milieu d'une résolution, rouvrir et reprendre **sans double dé ni perte de ressources**.
7. Consulter les faits publics, l'inventaire et le journal ; sauvegarder et reprendre la campagne ; supprimer une campagne après confirmation.
8. Construire le contexte IA depuis SQLite avec JSON compact et **filtres de visibilité** ; un PNJ ne doit pas apprendre un secret par simple présence dans la base.

**Ce que cette tranche ne promet pas encore :** catalogue étendu de sorts et classes, combat tactique élaboré, validation de 300–500 tours, UI multijoueur entièrement finalisée, export/import avancé et optimisation exhaustive des petits modèles. Ces obligations restent dans le **produit cible** et dans les lots ultérieurs, elles ne sont pas supprimées de la spécification.

### C.4 Ordre des lots et critère de passage

| Lot | Résultat livrable | Démonstration bloquante |
|---|---|---|
| **L0 — Socle** | Interface vide navigable, création SQLite, contrôle de version du schéma, campagnes et réglages sans fichier externe | Création, fermeture, redémarrage, restauration de l’état sur schéma compatible ; contrôles de clés étrangères et refus sans modification d’un schéma incompatible. |
| **L1 — Résolution** | UARE, dés, intents, effets bornés, journal, commandes idempotentes, écran de fiche | Action connue + action inventée ; même état après rejeu ; crash après jet sans relancer. |
| **L2 — Première partie** | Profil `llama.cpp`, `gm.read` / `gm.propose`, créateur de campagne, 3 PNJ, dialogues et contexte JSON minimal | Une scène de 20–30 tours non combattants avec objets, promesses, libre initiative, fermeture/reprise. |
| **L3 — Monde vivant** | Mémoire croyances/claims, directeur narratif, événements temporels, compactage, branchement | PNJ récurrent et conséquence différée ; aucune fuite de secret dans tests de contrôle. |
| **L4 — Jeu complet Lite** | Catalogue réellement implémenté, magie sélectionnée, combat simplifié, 1–6 joueurs, sauvegarde/export et UI définitive | Tests mécaniques et parcours utilisateur complets ; pas de contenu annoncé comme supporté sans résolveur. |
| **L5 — Qualification** | Résistance, benchmarks 4B/9B, test de campagnes longues et corrections | Critères des sections 17.1–17.5 atteints et résultats mesurés ; décisions de livraison tracées. |

**La première étape du développement est L0**, pas une nouvelle version de la spécification. L1 et L2 forment ensemble le premier objectif jouable.

### C.5 Contrat minimal de chaque composant

- Toute commande portant un effet possède `operation_id`, `branch_id`, `expected_state_version` et un acteur autorisé ; un retry retourne le même résultat durable.
- Toute proposition IA est **non fiable** jusqu'aux validations de forme, droits, provenance, règles et causalité ; aucun SQL ou script exécuté depuis du texte LLM.
- Tout jet est effectué **une seule fois** par opération, avec valeurs brutes et choix retenus conservés ; le rejeu ne contacte ni le RNG ni le LLM.
- Tout changement narratif opératoire est engagé avant d'être annoncé comme fait acquis ; un détail descriptif déjà annoncé reste traçable lorsqu'un joueur veut interagir avec lui.
- Toute sélection de mémoire vérifie l'identité logique de la tâche, la visibilité et les preuves de source ; le résumé ne prime jamais sur un fait canonique.
- Chaque résultat d'une étape validée doit survivre à un arrêt brutal, sous réserve des garanties du support de stockage et de SQLite ; ne jamais tenir une transaction ouverte pendant un appel LLM.

### C.6 Gel, gestion des écarts et critères de *definition of done*

- **Changement autorisé sans nouvelle version majeure :** erreur factuelle, ambiguïté dans un champ, test manquant, bug de sécurité, correctif de cohérence ; documenter le motif, les sections affectées et le test ajouté. La politique « aucune migration des anciennes bases SQLite » est une **décision explicite du porteur du projet**, consignée dans l’amendement du 30 octobre 2026 et en §13.7 ; elle ne doit pas être réintroduite comme critère bloquant de L0.
- **Changement soumis à décision explicite :** réduction durable de la liberté d'action, changement du moteur UARE, nouveau format de contexte, abandon de SQLite, passage à un MJ souverain, modification des règles Lite fondamentales ou ajout d'une dépendance à une infrastructure externe.
- **Fin de lot :** compilation reproductible, tests unitaires et d’intégration verts, contrôle de compatibilité du schéma et refus sans modification des anciennes bases incompatibles, absence de mutation invalide, démonstration utilisateur enregistrée et limitations restantes listées.
- **Validation du gameplay :** appréciation par des joueurs humains après sessions réelles ; **aucune** note de plaisir ou garantie de centaines d'heures n'est affirmée par anticipation.

**Règle d'arbitrage :** en cas de divergence, les invariants de sécurité, de non-appropriation des PJ, de persistance et de causalité figurant dans le corps de la V9.1 priment. Cette annexe fixe uniquement les choix de réalisation et les jalons.
