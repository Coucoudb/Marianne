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
    pub system_prompt: String,
    pub skills: Vec<String>, // IDs des skills
    pub tools: Vec<String>,  // Noms des tools (ex: "read_file", "write_file")
    pub web_search: Option<WebSearchConfig>,
    pub working_directory: Option<String>,
}
