> **Archive historique** : ce protocole concerne la version L2.4. Pour L2.4.2, suivre `ACCEPTATION_L2_4_2.md`. La devise principale obligatoire a ete retiree.

# L2.4 — protocole de clôture V9.1 (à exécuter sur Linux)

Cette livraison conserve les résultats de L0, L1, L2.3.2 et L2.3.3, mais **doit être retestée**.

## Installation

- Utiliser un nouveau répertoire et la nouvelle base `mj-universel-l2-4.sqlite3` ; ne pas migrer de campagnes d'essai anciennes.
- Configurer le modèle OpenAI-compatible et la clé dans SQLite. Exécuter `cargo check && cargo test && cargo run`.

## 15 contrôles avant gel

1. Créer une campagne « Vigo 1800 », deux joueurs ; les fiches sont initialement préparatoires.
2. Avec Mistral/llama.cpp, cliquer « Proposer 3 personnages » pour le premier joueur : trois propositions prévisualisées, aucune enregistrée sans clic.
3. Choisir un marin : sexe/genre, âge, taille, poids, apparence, voix, langues, personnalité et lieu initial enregistrés.
4. Vérifier FOR/DEX/CON/INT/SAG/CHA = permutation 8/10/12/13/14/15 ; niveau 1 ; PV/CA calculés côté Rust.
5. Vérifier les objets, les quantités et la monnaie initiale en SQLite ; aucun objet ou argent fantôme.
6. Demander trois nouvelles propositions pour le deuxième PJ : même devise principale, autres professions et dotations possibles.
7. Tenter d'appliquer une proposition utilisant une monnaie différente : rejet sans modification partielle.
8. Ajouter une monnaie secondaire ; elle ne se confond pas avec la principale ni avec une ressource arbitraire.
9. Avant le début du jeu, personnaliser le profil individuel ; verrouiller puis vérifier l'absence d'édition directe.
10. Olivia et Fabien commencent dans deux lieux distincts : le MJ n'invente ni proximité ni distance métrique.
11. Déplacer un seul PJ via un effet `move` : l'autre garde son lieu ; fermer puis rouvrir.
12. « Je vais voir Robet le charcutier » : accepter ou refuser l'existence du PNJ sans obligation ; si acceptée, événement `npc_introduced` une seule fois ; pas de titre ou de faveur gratuits.
13. Tester les crédits, la dernière ration, les stocks nuls, la promesse et une dépense impossible.
14. Résoudre un jet d20 puis abandonner un second ; fermer/reprendre sans relancer le hasard.
15. Effectuer une scène de 20–30 tours **sur L2.4** avec 3 PNJ, objets et promesses ; vérifier la cohérence du journal et les effets après redémarrage.

## Critère de sortie

Pas d'erreur Cargo, pas de mutation SQLite non validée, pas d'objet ou de monnaie créée par la seule narration, aucun champ physique forcé à modifier les scores mécaniques, aucune fuite de localisation privée dans une réponse au deuxième joueur, et reprise complète après fermeture.

## Limites délibérées

- L2.4 ne livre pas une économie de taux de change, le commerce à deux effets, les capacités de classes exhaustives ni la mémoire avancée du monde : ces fonctions demandent des règles ou des validateurs supplémentaires.
- `introduce_npc` enregistre une identité et un rôle ordinaire, pas encore une biographie et une personnalité persistante de PNJ ; les affirmations libres purement textuelles du MJ ne sont pas rendues canoniques automatiquement.
- L'absence d'un PNJ du registre ne prouve jamais qu'il n'existe pas ; seul le MJ arbitre sa plausibilité, avec Rust pour enregistrer une acceptation.
- Les capacités multijoueurs complètes (connaissances secrètes, consentement sur les autres PJ, interface définitive) restent au programme ultérieur V9.1.

**Statut : candidate de test, gel final à confirmer sur la machine de développement.**
