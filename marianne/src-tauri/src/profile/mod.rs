use std::path::Path;

/// Préférence de device pour le modèle LLM
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub enum DevicePreference {
    /// Utiliser le GPU si disponible (défaut)
    Gpu,
    /// Forcer le mode CPU
    Cpu,
}

impl Default for DevicePreference {
    fn default() -> Self {
        Self::Gpu
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct UserProfile {
    pub first_name: Option<String>,
    pub professional_status: ProfessionalStatus,
    pub family_status: FamilyStatus,
    pub department: Option<String>,
    pub topics_of_interest: Vec<String>,
    pub language_level: LanguageLevel,
    pub updated_at: i64,
    /// Préférence GPU / CPU — appliquée au prochain démarrage
    #[serde(default)]
    pub device_preference: DevicePreference,
    /// Identifiant du modèle sélectionné (ex: "phi-3-mini-q4")
    #[serde(default = "default_model_id")]
    pub selected_model: String,
}

fn default_model_id() -> String {
    "phi-3-mini-q4".to_string()
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum ProfessionalStatus {
    Salarie,
    AutoEntrepreneur,
    Chomeur,
    Retraite,
    Etudiant,
    Fonctionnaire,
    NonRenseigne,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum LanguageLevel {
    Simple,
    Standard,
    Expert,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum FamilyStatus {
    Celibataire,
    EnCouple,
    MarieOuPacse,
    Parent { children_count: u8 },
    NonRenseigne,
}

impl UserProfile {
    /// Générer le contexte personnalisé à injecter dans le prompt système
    pub fn to_context_string(&self) -> String {
        let mut parts = Vec::new();

        if let Some(name) = &self.first_name {
            parts.push(format!("L'utilisateur s'appelle {}.", name));
        }

        let status = match &self.professional_status {
            ProfessionalStatus::Salarie => "salarié(e)",
            ProfessionalStatus::AutoEntrepreneur => "auto-entrepreneur/euse",
            ProfessionalStatus::Chomeur => "demandeur/euse d'emploi",
            ProfessionalStatus::Retraite => "retraité(e)",
            ProfessionalStatus::Etudiant => "étudiant(e)",
            ProfessionalStatus::Fonctionnaire => "fonctionnaire",
            ProfessionalStatus::NonRenseigne => return parts.join(" "),
        };
        parts.push(format!("Il/elle est {}.", status));

        if let Some(dept) = &self.department {
            parts.push(format!("Il/elle réside dans le département {}.", dept));
        }

        let level_instruction = match self.language_level {
            LanguageLevel::Simple => "Utilise un langage très simple, sans jargon administratif.",
            LanguageLevel::Standard => "Utilise un langage clair et accessible.",
            LanguageLevel::Expert => "Tu peux utiliser les termes juridiques et administratifs précis.",
        };
        parts.push(level_instruction.to_string());

        parts.join(" ")
    }

    /// Charger depuis le fichier de configuration local
    pub fn load(data_dir: &Path) -> Self {
        let path = data_dir.join("profile.json");
        std::fs::read_to_string(path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    /// Sauvegarder
    pub fn save(&self, data_dir: &Path) -> anyhow::Result<()> {
        let path = data_dir.join("profile.json");
        std::fs::write(path, serde_json::to_string_pretty(self)?)?;
        Ok(())
    }
}

impl Default for UserProfile {
    fn default() -> Self {
        Self {
            first_name: None,
            professional_status: ProfessionalStatus::NonRenseigne,
            family_status: FamilyStatus::NonRenseigne,
            department: None,
            topics_of_interest: Vec::new(),
            language_level: LanguageLevel::Standard,
            updated_at: 0,
            device_preference: DevicePreference::default(),
            selected_model: default_model_id(),
        }
    }
}
