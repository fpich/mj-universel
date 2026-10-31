# Acceptation v0.3.16 — reprise du dialogue cyberpunk à 3 PJ

Objectif : éviter que le MJ IA traite un PJ comme un PNJ et qu'il affirme
un déplacement sans validation Rust. Aucun changement de schéma (27).

## Reproduction contrôlée

1. Créer trois PJ : Takeshi Kuroda (jardins suspendus tour Hikari),
   Rin Tachibana (marché souterrain d'Asagiri) et Akira Tachibana
   (ailleurs, ou au marché uniquement après mouvement validé).
2. Takeshi : « je suis où ? » → utiliser sa position SQLite, pas inventer
   un lieu précis si aucune position n'a été validée.
3. Rin : « Comment rejoindre Takeshi Kuroda ? » → ne pas divulguer
   automatiquement la position privée de Takeshi ni inventer un itinéraire.
4. Akira : « je veux aller au marché souterrain d'Asagiri » → succès de
   déplacement seulement avec `effect.kind=move` validé et journalisé.
   Le nom Asagari/Asagiri n'est PAS réconcilié automatiquement par Rust.
5. Akira : « dire bonjour à Rin Tachibana » → le MJ ne fait PAS parler,
   sourire, regarder ou manipuler l'étal par Rin. Sa réponse attend un
   tour du joueur incarnant Rin.
6. Rin : « envoyer chier Akira » → l'insulte peut être narrée ; aucune
   décision, parole, déplacement ni réaction interne n'est attribuée à Akira.
7. Redémarrer et vérifier que les lieux et le journal sont conservés.

## Contrôles programmatiques ajoutés

- Le prompt rappelle que **tous les PJ** sont contrôlés par les joueurs.
- Validation pré-commit de la narration (à partir des noms et scènes SQLite),
  indépendamment de la réponse du fournisseur, y compris sur les deux
  narrations candidates d'un jet.
- Tentative de correction narrative limitée, sans effet mécanique ajouté.
- Préservation de l'idempotence si un PJ s'est déplacé après une action déjà
  committée ; le contrôle original d'identité de requête/proposition demeure.
- Tests unitaires de présence, réaction inventée, pronom de continuation,
  cas de sous-chaîne (« seringue »/« Rin »), mouvement et position inconnue.

## Limitations et vérification

Ce contrôle lexical est une protection conservatrice, **pas** un analyseur
sémantique exhaustif du français. Il peut laisser passer une paraphrase
inattendue ou rejeter un passage littéraire ambigu. Les sous-lieux du même
marché ne sont pas encore reliés par un graphe de lieux canoniques :
la co-présence n'est confirmée que par la clé de scène normalisée.

À exécuter sous Rust :

```bash
cargo check
cargo test --lib
```

Puis jouer les 7 étapes ci-dessus avec un fournisseur LLM, vérifier les
cas `rejected` sans événement ni déplacement et vérifier le journal SQLite.
