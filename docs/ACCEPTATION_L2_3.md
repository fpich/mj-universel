> **ARCHIVE HISTORIQUE L2.3.1.** Dans L2.3.2, préparer les joueurs et les biens sur l’écran **Campagne** AVANT de commencer la partie, puis utiliser `ACCEPTATION_L2_3_2.md`.

# L2.3 — Plan de test manuel (à effectuer sous Linux)

## A. Avant démarrage

1. **Sauvegarder** le fichier `mj-universel-l2.sqlite3` depuis Paramètres. Ne pas rétrograder ensuite la même base vers L2.2.2 (schéma 23).
2. Dans le dossier du projet : `cargo check && cargo test && cargo run`.
3. Dans une campagne non archivée, ouvrir **Jouer · Chat RP**.
4. Dans **Fiche et objets**, ajouter le contact connu `Mara` (si pas déjà présent), puis créditer Kael de `20 crédits` et `2 bière` dans **Ressources et biens génériques**. Les quantités et noms sont totalement libres ; l'opération est une **édition utilisateur journalisée**, pas un gain narratif.
5. Vérifier le stock initial à l'écran et qu'il survit à la fermeture.

## B. Action libre et monnaie inexistante

Dans le chat : `Je donne 500 crédits à Mara pour obtenir ses informations.`

**Attendu :** la demande ne peut pas produire de transfert. Si le LLM propose une réussite et un effet `transfer`, Rust la rejette pour stock insuffisant (20 disponibles) ; aucune action ni aucun solde ne change. Le MJ peut aussi refuser l'action en raison du manque de moyens. **Jamais** de narration affirmant le paiement comme acquis.

## C. Transfert réel, sans liste de monnaies

Dans le chat : `Je donne 5 crédits à Mara.`

**Attendu :** si le LLM fournit `effect.kind = transfer` avec `asset=crédits`, `quantity=5`, `recipient=Mara`, Rust ramène Kael à **15 crédits**, enregistre **5 crédits** chez Mara, et journalise l'effet dans le chat. Répéter cette action constitue une nouvelle dépense ; réappliquer la **même clé technique** ne doit pas débiter deux fois.

## D. Objet physique et consommation

Dans le chat : `Je bois une bière que je possède.`

**Attendu :** si le modèle propose la consommation de 1 bière, Kael passe de 2 à 1, avec une écriture transactionnelle. En absence de bière, aucun effet n'est validé et aucune mécanique de « saoul » n'est fabriquée automatiquement.

Tester aussi avec un objet d'inventaire L1, par exemple `corde`, dont la quantité doit diminuer si le MJ propose un effet `consume` avec ce nom exact.

## E. Promesse ≠ paiement

Dans le chat : `Je promets à Mara de lui payer 500 crédits plus tard.`

**Attendu :** si le MJ propose `effect.kind = promise`, une promesse est journalisée, sans variation du solde. Elle apparaît dans **Fiche et objets > Promesses**. Le MJ ne déclare pas le paiement déjà effectué.

## F. Jet suspendu et effet lié

Tester une action où la conséquence de la réussite comprend un effet matériel (si le LLM en propose un). Le DD, les issues et le transfert éventuel doivent être **figés avant le lancer**. S'il manque la ressource au moment de la proposition, **pas de jet en attente**. Dans le cas valide, fermer / rouvrir, tirer une seule fois ; succès → appliquer au plus un effet, échec → aucune mutation matérielle.

## G. Continuité et limites

- Lancer une vingtaine d'actions : questions simples, refus, promesses, consommation, un jet de persuasion, puis retour sur l'échange antérieur. La cohérence narrative est à évaluer humainement.
- Les contacts doivent être enregistrés explicitement (pas de génération autonome de PNJ canonique à ce stade).
- Cette tranche L2.3 ne sait pas encore traiter de manière sûre un échange **bilatéral** (achat avec contrepartie), un effet composé, un gain depuis une source vérifiée, une condition mécanique nouvelle, les négociations de prix à étapes ni les croyances individuelles persistantes. Le refus est préférable à la création arbitraire d'un effet.
- LLM non déterministe : l'acceptation mécanique nécessite un JSON structuré valide ; les tours d'essai peuvent être arbitrés autrement par le modèle sans constituer nécessairement un bug.
