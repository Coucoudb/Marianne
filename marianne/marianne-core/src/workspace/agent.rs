use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSearchConfig {
    pub enabled: bool,
    pub custom_domains: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Agent {
    pub id: String,
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub system_prompt: String, // Le contenu markdown de l'agent (corps du fichier)
    #[serde(default)]
    pub skills: Vec<String>, // IDs des skills
    #[serde(default)]
    pub tools: Vec<String>,  // Noms des tools (ex: "read_file", "write_file")
    pub web_search: Option<WebSearchConfig>,
    pub working_directory: Option<String>,
}

/// Métadonnées YAML frontmatter d'un fichier .agent.md
#[derive(Debug, Deserialize, Serialize)]
struct AgentFrontmatter {
    name: String,
    description: String,
    #[serde(default)]
    skills: Vec<String>,
    #[serde(default)]
    tools: Vec<String>,
    web_search: Option<WebSearchConfig>,
    working_directory: Option<String>,
}

impl Agent {
    /// Parser un fichier .agent.md (YAML frontmatter + corps Markdown)
    pub fn from_markdown(id: &str, raw: &str) -> Result<Self, String> {
        let (frontmatter_str, body) = split_frontmatter(raw)
            .ok_or_else(|| format!("Fichier agent '{}' : frontmatter YAML manquant (délimiteurs --- requis)", id))?;

        let fm: AgentFrontmatter = serde_yaml::from_str(&frontmatter_str)
            .map_err(|e| format!("Fichier agent '{}' : YAML invalide — {}", id, e))?;

        Ok(Self {
            id: id.to_string(),
            name: fm.name,
            description: fm.description,
            system_prompt: body.trim().to_string(),
            skills: fm.skills,
            tools: fm.tools,
            web_search: fm.web_search,
            working_directory: fm.working_directory,
        })
    }

    /// Sérialiser en format .agent.md (YAML frontmatter + corps Markdown)
    pub fn to_markdown(&self) -> String {
        let fm = AgentFrontmatter {
            name: self.name.clone(),
            description: self.description.clone(),
            skills: self.skills.clone(),
            tools: self.tools.clone(),
            web_search: self.web_search.clone(),
            working_directory: self.working_directory.clone(),
        };
        
        let yaml_str = serde_yaml::to_string(&fm).unwrap_or_default();
        // serde_yaml adds "---" at the beginning, we want to strip it to have uniform formatting if we want
        let clean_yaml = yaml_str.trim_start_matches("---\n");
        
        format!("---\n{}---\n\n{}\n", clean_yaml, self.system_prompt)
    }
}

/// Séparer le YAML frontmatter du corps Markdown.
fn split_frontmatter(raw: &str) -> Option<(String, String)> {
    let trimmed = raw.trim_start();

    if !trimmed.starts_with("---") {
        return None;
    }

    let after_first = &trimmed[3..].trim_start_matches(['\r', '\n']);
    let end_pos = after_first.find("\n---")?;

    let frontmatter = after_first[..end_pos].to_string();
    let body = after_first[end_pos + 4..].to_string();

    Some((frontmatter, body))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_markdown() {
        let raw = r#"---
name: Agent Dev Rust
description: Expert en développement
tools:
  - read_file
skills:
  - rust-conventions
---

Tu es un expert Rust.
"#;
        let agent = Agent::from_markdown("rust-dev", raw).unwrap();
        assert_eq!(agent.id, "rust-dev");
        assert_eq!(agent.name, "Agent Dev Rust");
        assert_eq!(agent.tools, vec!["read_file".to_string()]);
        assert_eq!(agent.skills, vec!["rust-conventions".to_string()]);
        assert_eq!(agent.system_prompt, "Tu es un expert Rust.");
    }
}
