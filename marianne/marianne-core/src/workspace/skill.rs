use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Skill {
    pub id: String,
    pub name: String,
    pub description: String,
    pub content: String, // Le contenu markdown du skill (corps du fichier)
    /// Glob pattern optionnel pour le chargement contextuel (ex: "**/*.rs")
    #[serde(default)]
    pub scope: Option<String>,
}

/// Métadonnées YAML frontmatter d'un fichier .skill.md
#[derive(Debug, Deserialize, Serialize)]
struct SkillFrontmatter {
    name: String,
    description: String,
    #[serde(default)]
    scope: Option<String>,
}

impl Skill {
    /// Parser un fichier .skill.md (YAML frontmatter + corps Markdown)
    ///
    /// Format attendu :
    /// ```text
    /// ---
    /// name: Conventions Rust
    /// description: Règles de codage Rust
    /// scope: "**/*.rs"
    /// ---
    /// # Contenu du skill en Markdown
    /// ...
    /// ```
    pub fn from_markdown(id: &str, raw: &str) -> Result<Self, String> {
        let (frontmatter_str, body) = split_frontmatter(raw)
            .ok_or_else(|| format!("Fichier skill '{}' : frontmatter YAML manquant (délimiteurs --- requis)", id))?;

        let fm: SkillFrontmatter = serde_yaml::from_str(&frontmatter_str)
            .map_err(|e| format!("Fichier skill '{}' : YAML invalide — {}", id, e))?;

        Ok(Self {
            id: id.to_string(),
            name: fm.name,
            description: fm.description,
            content: body.trim().to_string(),
            scope: fm.scope,
        })
    }

    /// Sérialiser en format .skill.md (YAML frontmatter + corps Markdown)
    pub fn to_markdown(&self) -> String {
        let mut yaml_fields = format!("name: {:?}\ndescription: {:?}", self.name, self.description);
        if let Some(ref scope) = self.scope {
            yaml_fields.push_str(&format!("\nscope: {:?}", scope));
        }
        format!("---\n{}\n---\n\n{}\n", yaml_fields, self.content)
    }

    /// Vérifier si un chemin de fichier correspond au scope de ce skill.
    /// Retourne true si le skill n'a pas de scope (toujours applicable).
    pub fn matches_scope(&self, file_path: &str) -> bool {
        match &self.scope {
            None => true,
            Some(pattern) => {
                glob::Pattern::new(pattern)
                    .map(|p| p.matches(file_path))
                    .unwrap_or(false)
            }
        }
    }
}

/// Séparer le YAML frontmatter du corps Markdown.
/// Le frontmatter est délimité par `---` au début et à la fin.
fn split_frontmatter(raw: &str) -> Option<(String, String)> {
    let trimmed = raw.trim_start();

    // Le fichier doit commencer par ---
    if !trimmed.starts_with("---") {
        return None;
    }

    // Chercher le second ---
    let after_first = &trimmed[3..].trim_start_matches(['\r', '\n']);
    let end_pos = after_first.find("\n---")?;

    let frontmatter = after_first[..end_pos].to_string();
    let body = after_first[end_pos + 4..].to_string(); // +4 pour "\n---"

    Some((frontmatter, body))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_markdown() {
        let raw = r#"---
name: Conventions Rust
description: Règles de codage
scope: "**/*.rs"
---

# Règles
- Pas de unwrap()
"#;
        let skill = Skill::from_markdown("rust-conventions", raw).unwrap();
        assert_eq!(skill.id, "rust-conventions");
        assert_eq!(skill.name, "Conventions Rust");
        assert_eq!(skill.description, "Règles de codage");
        assert_eq!(skill.scope, Some("**/*.rs".to_string()));
        assert!(skill.content.contains("Pas de unwrap()"));
    }

    #[test]
    fn test_to_markdown_roundtrip() {
        let skill = Skill {
            id: "test".to_string(),
            name: "Test Skill".to_string(),
            description: "A test".to_string(),
            content: "# Content\nHello".to_string(),
            scope: None,
        };
        let md = skill.to_markdown();
        let parsed = Skill::from_markdown("test", &md).unwrap();
        assert_eq!(parsed.name, skill.name);
        assert_eq!(parsed.content, skill.content);
    }

    #[test]
    fn test_matches_scope() {
        let skill = Skill {
            id: "test".into(),
            name: "test".into(),
            description: "test".into(),
            content: "test".into(),
            scope: Some("**/*.rs".into()),
        };
        assert!(skill.matches_scope("src/main.rs"));
        assert!(!skill.matches_scope("src/main.ts"));

        let no_scope = Skill { scope: None, ..skill };
        assert!(no_scope.matches_scope("anything.txt"));
    }
}
