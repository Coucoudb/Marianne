use crate::workspace::agent::Agent;
use crate::workspace::skill::Skill;
use serde::Deserialize;
use std::path::{Path, PathBuf};
use tokio::fs;

#[derive(Debug, Clone, Deserialize, Default, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum SaveLevel {
    Global,
    #[default]
    Server,
    Project,
}

pub struct WorkspaceManager {
    global_dir: PathBuf,
    server_dir: PathBuf,
    project_dir: parking_lot::RwLock<Option<PathBuf>>,
}

impl WorkspaceManager {
    pub fn new(server_base_dir: &Path) -> Self {
        let global_dir = dirs::home_dir()
            .map(|h| h.join(".marianne"))
            .unwrap_or_else(|| server_base_dir.to_path_buf());

        Self {
            global_dir,
            server_dir: server_base_dir.to_path_buf(),
            project_dir: parking_lot::RwLock::new(None),
        }
    }

    pub fn set_project_dir(&self, dir: Option<PathBuf>) {
        *self.project_dir.write() = dir;
    }

    pub async fn init(&self) -> std::io::Result<()> {
        fs::create_dir_all(self.global_dir.join("agents")).await.ok();
        fs::create_dir_all(self.global_dir.join("skills")).await.ok();
        fs::create_dir_all(self.server_dir.join("agents")).await?;
        fs::create_dir_all(self.server_dir.join("skills")).await?;
        Ok(())
    }

    async fn read_agents_from_dir(dir: &Path) -> anyhow::Result<Vec<Agent>> {
        let mut agents = Vec::new();
        if !dir.exists() {
            return Ok(agents);
        }
        let mut entries = fs::read_dir(dir).await?;
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or_default();

            if ext == "json" {
                if let Ok(content) = fs::read_to_string(&path).await {
                    if let Ok(agent) = serde_json::from_str(&content) {
                        agents.push(agent);
                    }
                }
            } else if ext == "md" && path.file_name().unwrap_or_default().to_string_lossy().ends_with(".agent.md") {
                let id = path.file_stem().unwrap().to_string_lossy().trim_end_matches(".agent").to_string();
                if let Ok(content) = fs::read_to_string(&path).await {
                    if let Ok(agent) = Agent::from_markdown(&id, &content) {
                        agents.push(agent);
                    }
                }
            }
        }
        Ok(agents)
    }

    pub async fn list_agents(&self) -> anyhow::Result<Vec<Agent>> {
        let mut all_agents = std::collections::HashMap::new();

        // 1. Global
        if let Ok(agents) = Self::read_agents_from_dir(&self.global_dir.join("agents")).await {
            for a in agents {
                all_agents.insert(a.id.clone(), a);
            }
        }

        // 2. Server
        if let Ok(agents) = Self::read_agents_from_dir(&self.server_dir.join("agents")).await {
            for a in agents {
                all_agents.insert(a.id.clone(), a);
            }
        }

        // 3. Project
        let proj_dir = self.project_dir.read().clone();
        if let Some(proj_dir) = proj_dir {
            if let Ok(agents) = Self::read_agents_from_dir(&proj_dir.join(".marianne").join("agents")).await {
                for a in agents {
                    all_agents.insert(a.id.clone(), a);
                }
            }
        }

        Ok(all_agents.into_values().collect())
    }

    pub async fn save_agent(&self, agent: &Agent, level: SaveLevel) -> anyhow::Result<()> {
        let dir = match level {
            SaveLevel::Global => self.global_dir.join("agents"),
            SaveLevel::Server => self.server_dir.join("agents"),
            SaveLevel::Project => {
                let proj = self.project_dir.read().clone().ok_or_else(|| {
                    anyhow::anyhow!("Aucun projet actif. Impossible de sauvegarder au niveau projet.")
                })?;
                proj.join(".marianne").join("agents")
            }
        };

        fs::create_dir_all(&dir).await?;

        let path_md = dir.join(format!("{}.agent.md", agent.id));
        let content_md = agent.to_markdown();
        fs::write(&path_md, content_md).await?;

        // Supprimer l'ancien JSON s'il existe pour éviter les doublons
        let path_json = dir.join(format!("{}.json", agent.id));
        if path_json.exists() {
            let _ = fs::remove_file(path_json).await;
        }

        Ok(())
    }

    pub async fn delete_agent(&self, id: &str) -> anyhow::Result<()> {
        let dir = self.server_dir.join("agents");
        let path_md = dir.join(format!("{}.agent.md", id));
        let path_json = dir.join(format!("{}.json", id));

        let mut deleted = false;
        if path_md.exists() {
            fs::remove_file(path_md).await?;
            deleted = true;
        }
        if path_json.exists() {
            fs::remove_file(path_json).await?;
            deleted = true;
        }

        if !deleted {
            anyhow::bail!("Agent introuvable");
        }
        Ok(())
    }

    async fn read_skills_from_dir(dir: &Path) -> anyhow::Result<Vec<Skill>> {
        let mut skills = Vec::new();
        if !dir.exists() {
            return Ok(skills);
        }
        let mut entries = fs::read_dir(dir).await?;
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or_default();

            if ext == "json" {
                if let Ok(content) = fs::read_to_string(&path).await {
                    if let Ok(skill) = serde_json::from_str(&content) {
                        skills.push(skill);
                    }
                }
            } else if ext == "md" && path.file_name().unwrap_or_default().to_string_lossy().ends_with(".skill.md") {
                let id = path.file_stem().unwrap().to_string_lossy().trim_end_matches(".skill").to_string();
                if let Ok(content) = fs::read_to_string(&path).await {
                    if let Ok(skill) = Skill::from_markdown(&id, &content) {
                        skills.push(skill);
                    }
                }
            }
        }
        Ok(skills)
    }

    pub async fn list_skills(&self) -> anyhow::Result<Vec<Skill>> {
        let mut all_skills = std::collections::HashMap::new();

        // 1. Global
        if let Ok(skills) = Self::read_skills_from_dir(&self.global_dir.join("skills")).await {
            for s in skills {
                all_skills.insert(s.id.clone(), s);
            }
        }

        // 2. Server
        if let Ok(skills) = Self::read_skills_from_dir(&self.server_dir.join("skills")).await {
            for s in skills {
                all_skills.insert(s.id.clone(), s);
            }
        }

        // 3. Project
        let proj_dir = self.project_dir.read().clone();
        if let Some(proj_dir) = proj_dir {
            if let Ok(skills) = Self::read_skills_from_dir(&proj_dir.join(".marianne").join("skills")).await {
                for s in skills {
                    all_skills.insert(s.id.clone(), s);
                }
            }
        }

        Ok(all_skills.into_values().collect())
    }

    pub async fn save_skill(&self, skill: &Skill, level: SaveLevel) -> anyhow::Result<()> {
        let dir = match level {
            SaveLevel::Global => self.global_dir.join("skills"),
            SaveLevel::Server => self.server_dir.join("skills"),
            SaveLevel::Project => {
                let proj = self.project_dir.read().clone().ok_or_else(|| {
                    anyhow::anyhow!("Aucun projet actif. Impossible de sauvegarder au niveau projet.")
                })?;
                proj.join(".marianne").join("skills")
            }
        };

        fs::create_dir_all(&dir).await?;

        let path_md = dir.join(format!("{}.skill.md", skill.id));
        let content_md = skill.to_markdown();
        fs::write(&path_md, content_md).await?;

        // Supprimer l'ancien JSON s'il existe pour éviter les doublons
        let path_json = dir.join(format!("{}.json", skill.id));
        if path_json.exists() {
            let _ = fs::remove_file(path_json).await;
        }

        Ok(())
    }

    pub async fn delete_skill(&self, id: &str) -> anyhow::Result<()> {
        let dir = self.server_dir.join("skills");
        let path_md = dir.join(format!("{}.skill.md", id));
        let path_json = dir.join(format!("{}.json", id));

        let mut deleted = false;
        if path_md.exists() {
            fs::remove_file(path_md).await?;
            deleted = true;
        }
        if path_json.exists() {
            fs::remove_file(path_json).await?;
            deleted = true;
        }

        if !deleted {
            anyhow::bail!("Skill introuvable");
        }
        Ok(())
    }
}
