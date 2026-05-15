// src-tauri/src/llm/sampler.rs

/// Stratégie de sampling pour le LLM
pub struct Sampler {
    pub temperature: f64,
    pub top_p: f64,
    pub repeat_penalty: f32,
    pub repeat_last_n: usize,
}

impl Default for Sampler {
    fn default() -> Self {
        Self {
            temperature: 0.15,
            top_p: 0.9,
            repeat_penalty: 1.15,
            repeat_last_n: 64,
        }
    }
}

impl Sampler {
    /// Appliquer la pénalité de répétition sur les logits
    pub fn apply_repeat_penalty(&self, logits: &mut [f32], recent_tokens: &[u32]) {
        let start = recent_tokens.len().saturating_sub(self.repeat_last_n);
        for &token_id in &recent_tokens[start..] {
            if let Some(logit) = logits.get_mut(token_id as usize) {
                if *logit > 0.0 {
                    *logit /= self.repeat_penalty;
                } else {
                    *logit *= self.repeat_penalty;
                }
            }
        }
    }

    /// Sampling top-p (nucleus sampling) avec température
    pub fn sample(&self, logits: &[f32]) -> u32 {
        // TODO Phase 2 : implémenter sampling complet avec température + top-p
        // Pour l'instant, argmax simple
        logits
            .iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
            .map(|(idx, _)| idx as u32)
            .unwrap_or(0)
    }
}
