# Critères d'acceptation — L1

| ID | Critère | Contrôle |
|---|---|---|
| L1-01 | `cargo check` sans erreurs | Compiler sur la machine cible |
| L1-02 | `cargo test` sans échec | Tester le moteur, les dés et SQLite |
| L1-03 | Première exécution sans migration L0 | Nouveau fichier `mj-universel-l1.sqlite3` |
| L1-04 | Campagne avec 1–6 participants | Création et fiches automatiques |
| L1-05 | Caractéristiques et PV persistent | Enregistrement, fermeture et reprise |
| L1-06 | Ajout/retrait inventaire journalisé | Contrôle en UI et dans SQLite |
| L1-07 | Dés N d M + mod et avantage | Jeux de tests déterministes |
| L1-08 | Action libre sans jet et avec DD | Résolution UI et journalisation |
| L1-09 | Idempotence des actions | Deux fois même clé = une seule mutation |
| L1-10 | Cible hors campagne rejetée | Aucune mutation ni événement |
| L1-11 | Suppression campagne en cascade | Aucun personnage ni événement restant |
| L1-12 | Pas de tromperie sur le MJ IA | UI affiche « arbitrage manuel L1 » |

**Validation du livrable en environnement de création** : schéma SQLite vérifié avec Python (`quick_check`, FK, unicité, persistance, cascade). **Compilation Cargo non exécutée** dans cet environnement ; L1-01 et L1-02 restent à confirmer par l'utilisateur.
