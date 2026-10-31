# Démarrage du développement — MJ universel Rust

**Référence gelée :** `specification_mj_universel_rust_v9_1_stable.md` (annexe C ; politique SQLite sans migration des anciennes bases).  
**Statut :** plan de développement initial ; aucune fonctionnalité n'est encore implémentée.

## Mission du produit

Créer une application desktop Rust où les joueurs décrivent librement leurs actions ; l'IA joue un MJ vivant et non dirigiste ; Rust arbitre les actions et les dés ; SQLite conserve l'état complet, les souvenirs et tous les paramètres.

## Décisions verrouillées

- Une application Rust native, `eframe/egui` pour l'interface.
- `rusqlite` et SQLite, source de vérité unique, sans `.env` ni fichier de configuration utilisateur.
- `reqwest`/`tokio` pour `llama.cpp` et les fournisseurs compatibles OpenAI ; deux outils JSON, `gm.read` et `gm.propose`.
- JSON compact et contexte adaptatif ; aucun DCP/1 ni TOON ; 256K reste un maximum facultatif.
- D&D 2024 Lite, UARE au centre, combats optionnels.
- Campagnes créées à partir d'un contexte libre, partiel ou complet ; propositions IA validées par l'utilisateur.

## Ordre concret

| Étape | Travail | Résultat vérifiable |
|---|---|---|
| 1 | Initialiser workspace Cargo et UI desktop | L'application se lance ; écran de sélection des campagnes. |
| 2 | Mettre en place SQLite + schéma de création embarqué et contrôle de version sans migration | Création, chargement, suppression avec confirmation ; refus non destructif des anciens schémas incompatibles ; aucune configuration externe. |
| 3 | Ajouter journal d'événements + idempotence + RNG | Actions rejouables ; aucun deuxième tirage après crash/retry. |
| 4 | Implémenter UARE minimal + fiche PJ | Intention libre, test d20 si utile, résultat et inventaire consultables. |
| 5 | Brancher `llama.cpp` et les contrats JSON | Une scène avec trois PNJ et actions créatives ; zéro mutation IA directe. |
| 6 | Ajouter initialisation par l'utilisateur, mémoire et effets différés | Réouverture d'une campagne avec relations et conséquences conservées. |
| 7 | Élargir D&D Lite, multijoueur et UI | Catalogue implémenté et testé ; 1–6 joueurs et réglages depuis l'UI. |
| 8 | Tester les longues campagnes | Mesures de latence, mémoire, agence et gameplay humain documentées. |

## Premier test de bout en bout

Créer une campagne « Le village de Valombre » avec un PJ, un marchand, une garde et une prêtresse. Le joueur décide librement de négocier une information, voler ou échanger un objet et quitter le lieu. Le MJ ne propose pas systématiquement un combat. L'application doit retenir une promesse et une croyance d'un PNJ ; après fermeture et réouverture, tous les faits sont retrouvés ; un jet déjà tiré ne change pas.

## Hors périmètre immédiat, pas supprimé

Couverture avancée des sorts, UI multijoueur complète, import/export enrichi, campagnes de plusieurs centaines de tours. Tout demeure exigé pour la qualification finale.

## Critère de réussite du premier prototype

La scène de test est jouable en texte libre, le MJ garde une initiative crédible, l'état officiel reste sous contrôle Rust, la mémoire est persistée et les tests de reprise sont verts. Le plaisir de jeu ne sera évalué qu'avec de véritables joueurs.
