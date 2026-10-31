# Lot L2 — Acceptation

## Tests automatisés attendus

- L0 et L1 : tous les tests historiques passent avec la nouvelle base L2.
- JSON : une proposition valide est acceptée, une valeur DD hors bornes ou un effet interdit est rejeté.
- IA : l'outil de lecture ne peut lire que les sections autorisées, et `gm_propose` ne modifie pas les données SQLite.
- Persistance : enregistrement atomique d'une décision IA, du jet et du résultat ; idempotence garantie ; faits narratifs soumis à validation avant canonisation.
- Clé API : chiffrement authentifié et échec de déverrouillage avec une mauvaise phrase maîtresse.
- Reprise : les résultats d'actions demeurent dans SQLite après redémarrage.

## Contrôles manuels complémentaires

- Modèle local réellement lancé et connecté (non simulé).
- Action triviale → arbitrage sans jet ; action incertaine → d20 effectué dans Rust.
- Déconnexion réseau → erreur claire, aucun jet ni état de campagne modifié.
- Réponse IA non JSON / appel d'outil non reconnu → échec propre et possibilité de jouer en manuel.
- Écran Paramètres : URL, modèle, température, timeout et contexte persistants en SQLite.

## Hors du lot L2

Le directeur narratif complet, la simulation autonomes des PNJ, le compactage sémantique automatique, l'ensemble des effets D&D Lite, la génération de monde guidée et les arbitrages mécaniques complexes restent à réaliser. Les interactions IA de L2 sont limitées à une décision d'arbitrage par action et à des propositions de faits soumis à approbation.
