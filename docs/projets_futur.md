# ⚖️ Accès universel au droit — Vision complète

---

## Le problème réel, chiffré

En France aujourd'hui :

- **67 % des personnes** confrontées à un problème juridique ne consultent jamais un professionnel — faute d'argent, de temps, ou d'oser
- Un avocat généraliste coûte **150–400 €/heure**
- L'aide juridictionnelle est sous-financée et prend des mois
- Comprendre son bail, contester son licenciement, répondre à un huissier — ce sont des problèmes **quotidiens, massifs, solubles**, que les gens affrontent seuls avec Google

Ce n'est pas un problème de loi. **C'est un problème d'interface entre la loi et les gens.**

---

## Le "PayPal moment"

PayPal n'a pas créé l'argent. Il a créé une **couche d'accès** qui a rendu le transfert d'argent aussi simple qu'un email.

Ton produit ne crée pas la loi. Il crée une **couche d'accès** qui rend la compréhension de ses droits aussi simple que Wikipedia.

> *"La loi est publique. La comprendre ne devrait pas coûter 300 €."*

La rupture n'est pas technologique — elle est **philosophique** : le savoir juridique sort des cabinets et entre dans la poche de chacun.

---

## Ce que le produit fait vraiment

Trois fonctions distinctes, jamais confondues :

**1. Diagnostic** → *"J'ai ce problème, quels sont mes droits ?"*
L'IA identifie le domaine juridique, les textes applicables, les délais, les recours possibles. Elle cite ses sources article par article (Légifrance).

**2. Génération de documents** → *"Aide-moi à écrire cette lettre"*
Lettre de mise en demeure, recours gracieux, saisine du médiateur, courrier à l'huissier — générés depuis des templates validés, personnalisés à la situation.

**3. Préparation** → *"Je passe devant le tribunal dans 3 semaines"*
Structurer ses arguments, comprendre la procédure, anticiper les questions du juge.

**Ce qu'il ne fait pas :** Il n'est pas avocat. Il n'assure pas, il n'engage pas sa responsabilité. Il *informe*. Cette distinction existe en droit français : l'**information juridique** n'est pas le **conseil juridique** réglementé. C'est le même statut que Légifrance elle-même.

---

## Architecture technique — ton territoire naturel

La bonne nouvelle : tu as déjà construit 80 % de la stack avec Marianne.

### Le corpus — open data parfait pour le RAG

```
Légifrance API          → Code Civil, Code du Travail, Code de la Consommation...
Cour de cassation       → Jurisprudence (API publique)
Conseil d'État          → Jurisprudence administrative
EUR-marianne                 → Règlements européens applicables
data.gouv.fr            → Circulaires, décrets, formulaires officiels
```

Les textes juridiques sont **le meilleur corpus possible pour le RAG** :
- Structure naturelle par articles et alinéas → chunking trivial
- Références explicites entre articles → graphe de dépendances (ton `petgraph`)
- Vocabulaire contrôlé et stable → BM25 très efficace
- Tout est déjà public et en français

### Workspace Cargo

```
marianne-workspace/
├── marianne-core/           # Moteur RAG juridique, parsing Légifrance, NER légal
├── marianne-tauri/          # App desktop local-first (confidentiel)
├── marianne-server/         # API Axum — mode serveur municipal/associatif
└── marianne-web/            # Frontend Angular (ton expertise directe)
```

### Innovations techniques par rapport à Marianne

**Versioning temporel du corpus**
```rust
// Une loi peut changer. L'article L1234-5 en 2022 ≠ 2024.
pub struct LegalChunk {
    article_id: String,
    content: String,
    effective_from: NaiveDate,
    effective_until: Option<NaiveDate>,  // None = en vigueur
    source_url: String,                  // Lien Légifrance direct
}
```

**Graphe de dépendances juridiques**
```rust
// "L'article L1237-19 renvoie à L1237-15 qui renvoie à..."
// Traversée multi-hop avec petgraph comme dans Marianne
// Mais ici le graphe EST la structure de la loi
graph.add_edge(article_a, article_b, ReferenceType::Modifies);
graph.add_edge(article_a, article_c, ReferenceType::Exceptions);
```

**Threshold de confiance adapté au droit**
```rust
// ≥ 0.85 → réponse avec citation directe
// 0.70–0.85 → réponse + "vérifiez avec un professionnel"
// < 0.70 → "situation compmariannee, consultez un avocat spécialisé"
// + domaines exclus : droit pénal, droit fiscal compmariannee → toujours orienter
```

**Explicabilité totale** — le différenciateur clé vs ChatGPT :
```
Réponse : "Votre employeur a 1 mois pour vous remettre vos documents de fin de contrat."
Sources  : Art. L1234-19 Code du Travail (vérifié le 12/06/2025)
           Art. R1234-9 (précision sur le solde de tout compte)
→ [Voir sur Légifrance] [Voir la jurisprudence associée]
```

---

## Roadmap — découpage pragmatique

### V0.1 — Le beachhead (3 mois)
**Droit du travail uniquement.** C'est le domaine le plus consulté, le plus douloureux, et le mieux documenté.
- RAG sur Code du Travail + jurisprudence Prud'homale
- 5 cas d'usage : licenciement, heures sup, harcèlement, période d'essai, rupture conventionnelle
- Interface desktop Tauri, 100 % local
- Génération de 3 lettres-types validées par un juriste

### V0.2 — Extension (6 mois)
- Droit locatif (Code Civil + loi ALUR)
- Droit de la consommation
- API Axum pour partenaires
- Angular web app (mode serveur)

### V1.0 — Produit (12 mois)
- 6 domaines couverts
- Partenariats syndicats (CGT, CFDT ont des services juridiques débordés)
- Intégration `Démarches Simplifiées` API gouvernementale
- Fine-tuning LoRA sur cas anonymisés

### V2.0 — Expansion (18–24 mois)
- Belgique, Québec (même langue, droits différents — même architecture)
- Modèle LLM spécialisé droit français entraîné from scratch
- Marketplace de juristes pour les cas compmariannees (revenus d'orientation)

---

## Les risques et comment les neutraliser

**⚠️ Risque réglementaire :** Le conseil juridique est réservé aux avocats (loi du 31 déc. 1971).
→ **Réponse :** Le produit fait de l'*information* (statut légal de Légifrance), jamais du *conseil*. Afficher systématiquement les sources, pas d'avis personnalisé engageant. Faire valider la ligne éditoriale par un avocat dès V0.1.

**⚠️ Hallucinations :** Une mauvaise réponse peut nuire réellement à quelqu'un.
→ **Réponse :** Score de confiance affiché à l'utilisateur. Jamais de réponse sans source citée. Domaines à risque (pénal, fiscal) → redirection systématique vers un professionnel. Clause de non-responsabilité juridiquement solide.

**⚠️ Corpus outdaté :** Les lois changent.
→ **Réponse :** L'API Légifrance expose les dates d'entrée en vigueur. Re-indexation automatique hebdomadaire. Affichage de la date de vérification de chaque source.

**⚠️ GPT-4 fait déjà ça :**
→ **Réponse :** Trois avantages défensifs durables : (1) **Confidentialité locale** — ta question sur ton divorce ne passe par aucun serveur américain. (2) **Spécialisation** — GPT hallucine sur les articles précis du Code du Travail, ton système cite la source. (3) **Souveraineté** — argument massif auprès des mairies et de l'État français.

---

## Pourquoi maintenant et pas dans 5 ans

- Les LLMs atteignent en 2024–2025 le seuil de fiabilité suffisant pour le raisonnement structuré sur des corpus fermés
- L'API Légifrance est meilleure que jamais
- L'**AI Act européen** favorise les systèmes explicables, locaux, souverains — c'est ton moat compétitif face aux Big Tech
- Le Qwen2.5-Coder 32B que tu vas faire tourner localement est *exactement* le type de modèle adapté au raisonnement juridique structuré
- La crise de confiance dans les institutions pousse les gens vers des outils qui leur redonnent du contrôle direct