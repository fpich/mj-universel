**Version 0.3.9 (base neuve)** : ancienne campagne non migrée. Créer une nouvelle campagne pour ce test. Fichier `mj-universel-l2-3-2.sqlite3` distinct.

# Acceptation L2.3.2 — Préparation et verrouillage

1. Créer une campagne « Tharsis » avec 1 joueur. Avant la préparation, la partie doit être indisponible.
2. Dans Campagne, sélectionner Joueur 1. Renommer le PJ « Kael », régler CHA 14, lui donner une lampe, 20 crédits et enregistrer le contact Mara. Enregistrer ou valider définitivement.
3. Vérifier le double consentement « Confirmer et verrouiller » ; une fois la partie lancée, Fiche et objets est **uniquement consultable**.
4. Dans le chat, « Je donne 500 crédits à Mara » doit échouer sans débiter ni créer de monnaie. « Je donne 5 crédits à Mara » peut retirer 5 crédits après validation Rust. Aucun bouton ne permet de recharger arbitrairement les crédits.
5. La réouverture d'une campagne commencée doit afficher une fiche verrouillée, avec le solde et les objets persistants.
6. Dans Mémoire, une liste vide est possible : les propositions `fact_proposal` sont facultatives. Si le modèle en fournit une et que l'action est validée, elle peut être approuvée pendant la partie puis apparaître dans les faits approuvés. Le dialogue narratif n'est pas automatiquement canonique.
7. Tester une campagne L2.3.1 **déjà jouée** à partir d'une copie SQLite : son historique est conservé et sa fiche verrouillée. Une campagne non jouée doit pouvoir achever la préparation.
8. Fermer pendant un jet autorisé, rouvrir, tirer une seule fois ; aucun résultat ou objet n'est dupliqué.

L2.3.2 ne fournit pas encore les gains narratifs d'objets, la progression ou la création automatique de nouveaux PNJ canoniques. Ces limites doivent rester visibles dans les retours de test.
