use std::path::Path;

#[derive(Debug)]
pub enum DocumentType {
    Pdf,
    Image(ImageFormat),
    Text,
    Unknown,
}

#[derive(Debug)]
pub enum ImageFormat {
    Jpeg,
    Png,
    Webp,
}

pub struct DocumentExtractor;

impl DocumentExtractor {
    /// Détecter le type depuis l'extension
    pub fn detect_type(path: &Path) -> DocumentType {
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();

        match ext.as_str() {
            "pdf" => DocumentType::Pdf,
            "jpg" | "jpeg" => DocumentType::Image(ImageFormat::Jpeg),
            "png" => DocumentType::Image(ImageFormat::Png),
            "webp" => DocumentType::Image(ImageFormat::Webp),
            "txt" | "md" => DocumentType::Text,
            _ => DocumentType::Unknown,
        }
    }

    /// Extraire le texte depuis un PDF
    pub fn extract_pdf(path: &Path) -> anyhow::Result<String> {
        let bytes = std::fs::read(path)?;

        let text = pdf_extract::extract_text_from_mem(&bytes)
            .map_err(|e| anyhow::anyhow!("Erreur extraction PDF : {}", e))?;

        // Nettoyer les artefacts courants dans les PDF administratifs
        let cleaned = text
            .lines()
            .filter(|l| !l.trim().is_empty())
            .collect::<Vec<_>>()
            .join("\n");

        tracing::info!("PDF extrait : {} caractères depuis {:?}", cleaned.len(), path);
        Ok(cleaned)
    }

    /// Placeholder pour extraction depuis image (nécessite OCR/Vision)
    pub fn extract_image(_path: &Path) -> anyhow::Result<String> {
        anyhow::bail!(
            "L'analyse d'images nécessite un modèle multimodal. Utilisez copier-coller pour l'instant."
        )
    }
}
