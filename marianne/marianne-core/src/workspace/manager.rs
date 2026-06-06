use crate::workspace::agent::Agent;
use crate::workspace::skill::Skill;
use std::path::{Path, PathBuf};
use tokio::fs;

pub struct WorkspaceManager {
    agents_dir: PathBuf,
    skills_dir: PathBuf,
}

impl WorkspaceManager {
    pub fn new(base_dir: &Path) -> Self {
        let agents_dir = base_dir.join("agents");
        let skills_dir = base_dir.join("skills");
        Self { agents_dir, skills_dir }
    }

    pub async fn init(&self) -> std::io::Result<()> {
        fs::create_dir_all(&self.agents_dir).await?;
        fs::create_dir_all(&self.skills_dir).await?;
        Ok(())
    }

    pub async fn list_agents(&self) -> anyhow::Result<Vec<Agent>> {
        let mut agents = Vec::new();
        let mut entries = fs::read_dir(&self.agents_dir).await?;
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.extension().unwrap_or_default() == "json" {
                if let Ok(content) = fs::read_to_string(&path).await {
                    if let Ok(agent) = serde_json::from_str(&content) {
                        agents.push(agent);
                    }
                }
            }
        }
        Ok(agents)
    }

    pub async fn save_agent(&self, agent: &Agent) -> anyhow::Result<()> {
        let path = self.agents_dir.join(format!("{}.json", agent.id));
        let content = serde_json::to_string_pretty(agent)?;
        fs::write(path, content).await?;
        Ok(())
    }

    // Identique pour Skills (JSON pour l'instant pour la simplicité, on passera au Markdown si besoin)
    pub async fn list_skills(&self) -> anyhow::Result<Vec<Skill>> {
        let mut skills = Vec::new();
        let mut entries = fs::read_dir(&self.skills_dir).await?;
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.extension().unwrap_or_default() == "json" {
                if let Ok(content) = fs::read_to_string(&path).await {
                    if let Ok(skill) = serde_json::from_str(&content) {
                        skills.push(skill);
                    }
                }
            }
        }
        Ok(skills)
    }

    pub async fn save_skill(&self, skill: &Skill) -> anyhow::Result<()> {
        let path = self.skills_dir.join(format!("{}.json", skill.id));
        let content = serde_json::to_string_pretty(skill)?;
        fs::write(path, content).await?;
        Ok(())
    }
}
