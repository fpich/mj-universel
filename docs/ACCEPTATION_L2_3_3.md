# Recette L2.3.3 — Stabilisation technique (V9.1)

| Test | Résultat attendu |
|---|---|
| `cargo check && cargo test` | Compilation et tests réussis sur machine Linux |
| Consommer 1 eau sur 1 | Stock à 0, aucune contrainte SQLite violée |
| Nouvelle consommation d'eau sur stock épuisé | Refus sans événement `action_resolved` ni débit |
| Rejouer le même `action_key` déjà résolu | Même résultat, aucun double débit |
| Transférer une ration à un contact déjà connu | Stock PJ diminue, stock PNJ augmente |
| Transférer une ration à un PNJ nommé par le MJ dans un récit validé | PNJ enregistré avec source + transfert atomique |
| Transférer à un nom inconnu seulement écrit par le joueur | Rejet sans création de PNJ |
| `roll_mode:public` avec `auto_success` | Résolution sans dé, valeur mécaniquement inerte neutralisée |
| `roll_mode:public` avec `ability_check` | Rejet JSON ou demande unique de correction, aucun jet automatique |
| Départ explicite vers `docks sud` avec effet `move` | Un événement `actor_moved` et emplacement courant enregistré |
| Destination inventée par le MJ | Effet rejeté, aucune position modifiée |
| Fermeture/réouverture | Fiches, stocks, engagements, jets en attente et lieu canonique conservés |

**Limite de recette :** absence de Cargo ici ; les tests ci-dessus sont à vérifier dans le terminal et en jeu. Les fonctionnalités longues de mémoire, croyances et autonomie de PNJ restent L3.
