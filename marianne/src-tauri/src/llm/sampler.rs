// src-tauri/src/llm/sampler.rs
use candle_core::{Result, Tensor};

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
    pub fn new(temperature: f64, top_p: f64, repeat_penalty: f32, repeat_last_n: usize) -> Self {
        Self {
            temperature,
            top_p,
            repeat_penalty,
            repeat_last_n,
        }
    }

    /// Appliquer la pénalité de répétition sur les logits
    pub fn apply_repeat_penalty(&self, logits: &mut Vec<f32>, recent_tokens: &[u32]) {
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

    /// Sampling complet : température + top-p (nucleus sampling)
    ///
    /// - temperature = 0 → argmax déterministe
    /// - temperature > 0 → softmax puis nucleus sampling top-p
    pub fn sample(&self, logits_tensor: &Tensor, recent_tokens: &[u32]) -> Result<u32> {
        let logits = logits_tensor.to_vec1::<f32>()?;
        let mut logits = logits;

        // 1. Appliquer la pénalité de répétition
        self.apply_repeat_penalty(&mut logits, recent_tokens);

        // 2. Si température ~0, argmax déterministe
        if self.temperature < 1e-7 {
            return Ok(logits
                .iter()
                .enumerate()
                .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
                .map(|(idx, _)| idx as u32)
                .unwrap_or(0));
        }

        // 3. Appliquer la température
        let inv_temp = 1.0 / self.temperature as f32;
        for logit in logits.iter_mut() {
            *logit *= inv_temp;
        }

        // 4. Softmax
        let max_logit = logits.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
        let mut probs: Vec<f32> = logits.iter().map(|&l| (l - max_logit).exp()).collect();
        let sum: f32 = probs.iter().sum();
        for p in probs.iter_mut() {
            *p /= sum;
        }

        // 5. Top-p (nucleus) sampling
        let mut indexed_probs: Vec<(usize, f32)> =
            probs.iter().enumerate().map(|(i, &p)| (i, p)).collect();
        indexed_probs.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

        let mut cumulative = 0.0f32;
        let mut nucleus: Vec<(usize, f32)> = Vec::new();
        for (idx, prob) in indexed_probs {
            cumulative += prob;
            nucleus.push((idx, prob));
            if cumulative >= self.top_p as f32 {
                break;
            }
        }

        // 6. Re-normaliser le nucleus
        let nucleus_sum: f32 = nucleus.iter().map(|(_, p)| p).sum();
        for (_, p) in nucleus.iter_mut() {
            *p /= nucleus_sum;
        }

        // 7. Tirer aléatoirement selon la distribution
        let random: f32 = rand_f32();
        let mut acc = 0.0f32;
        for (idx, prob) in &nucleus {
            acc += prob;
            if acc >= random {
                return Ok(*idx as u32);
            }
        }

        // Fallback : dernier du nucleus
        Ok(nucleus.last().map(|(idx, _)| *idx as u32).unwrap_or(0))
    }
}

/// Générateur pseudo-aléatoire simple (xorshift32) — pas de dépendance externe
/// Suffisant pour le sampling LLM (pas besoin de crypto-qualité)
fn rand_f32() -> f32 {
    use std::cell::Cell;
    thread_local! {
        static STATE: Cell<u32> = Cell::new(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .subsec_nanos()
        );
    }
    STATE.with(|s| {
        let mut x = s.get();
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        s.set(x);
        (x as f32) / (u32::MAX as f32)
    })
}
