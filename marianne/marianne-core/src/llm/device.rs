// marianne-core/src/llm/device.rs
// Exposition des informations sur les devices llama_cpp backend (GPU/CPU).

// Réexporter les types nécessaires depuis llama_cpp_2
pub use llama_cpp_2::{LlamaBackendDevice, LlamaBackendDeviceType};

/// Liste tous les devices backend disponibles sur cette machine (GPU + CPU)
pub fn list_backend_devices() -> Vec<LlamaBackendDevice> {
    llama_cpp_2::list_llama_ggml_backend_devices()
}

/// Détecte si un GPU est disponible sur cette machine
pub fn is_gpu_available() -> bool {
    list_backend_devices()
        .iter()
        .any(|d| matches!(
            d.device_type,
            LlamaBackendDeviceType::Gpu
                | LlamaBackendDeviceType::IntegratedGpu
                | LlamaBackendDeviceType::Accelerator
        ))
}

/// Liste les GPU utilisables par llama-cpp, avec la même logique de filtrage que LlmEngine.
///
/// Si des GPU dédiés (`Gpu`) sont présents, seuls ceux-ci sont retournés.
/// Sinon, tous les GPU (y compris intégrés) sont retournés en fallback.
///
/// Les indices retournés correspondent exactement aux indices attendus par `main_gpu`
/// dans llama-cpp — ils doivent être utilisés pour construire les options de sélection GPU
/// exposées à l'utilisateur.
pub fn list_usable_gpu_devices() -> Vec<LlamaBackendDevice> {
    let all = list_backend_devices();
    let all_gpu: Vec<LlamaBackendDevice> = all
        .into_iter()
        .filter(|d| {
            matches!(
                d.device_type,
                LlamaBackendDeviceType::Gpu
                    | LlamaBackendDeviceType::IntegratedGpu
                    | LlamaBackendDeviceType::Accelerator
            )
        })
        .collect();

    let dedicated: Vec<LlamaBackendDevice> = all_gpu
        .iter()
        .filter(|d| matches!(d.device_type, LlamaBackendDeviceType::Gpu))
        .cloned()
        .collect();

    if !dedicated.is_empty() {
        dedicated
    } else {
        all_gpu
    }
}

/// Diagnostic : détecte si llama.cpp a été compilé sans support GPU
/// malgré la présence de matériel GPU dans le système
pub fn diagnose_gpu_compilation() -> Option<String> {
    // Si llama.cpp voit déjà des GPU, pas de problème
    if is_gpu_available() {
        return None;
    }

    #[cfg(target_os = "windows")]
    {
        use std::process::Command;
        
        // Vérifier si wmic détecte des GPU
        if let Ok(output) = Command::new("wmic")
            .args(["path", "win32_VideoController", "get", "name"])
            .output()
        {
            let text = String::from_utf8_lossy(&output.stdout);
            let gpus: Vec<&str> = text.lines()
                .map(|l| l.trim())
                .filter(|l| !l.is_empty() && !l.eq_ignore_ascii_case("name"))
                .collect();
            
            if !gpus.is_empty() {
                let gpu_list = gpus.join(", ");
                let advice = if gpu_list.to_lowercase().contains("nvidia") {
                    "Recompilez avec: cargo build --release --features cuda"
                } else {
                    "Recompilez avec: cargo build --release --features vulkan"
                };
                
                return Some(format!(
                    "GPU système détecté ({}) mais invisible pour llama.cpp. {}",
                    gpu_list, advice
                ));
            }
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        use std::process::Command;
        
        // Linux: essayer lspci pour détecter les GPU
        #[cfg(target_os = "linux")]
        if let Ok(output) = Command::new("lspci").output() {
            let text = String::from_utf8_lossy(&output.stdout).to_lowercase();
            if text.contains("nvidia") || text.contains("amd") || text.contains("radeon") {
                let advice = if text.contains("nvidia") {
                    "Recompilez avec: cargo build --release --features cuda"
                } else {
                    "Recompilez avec: cargo build --release --features vulkan"
                };
                return Some(format!(
                    "GPU système détecté mais invisible pour llama.cpp. {}", advice
                ));
            }
        }
        
        // macOS: essayer system_profiler pour détecter les GPU
        #[cfg(target_os = "macos")]
        if let Ok(output) = Command::new("system_profiler")
            .args(["SPDisplaysDataType"])
            .output()
        {
            let text = String::from_utf8_lossy(&output.stdout).to_lowercase();
            if text.contains("amd") || text.contains("radeon") || text.contains("nvidia") {
                return Some(
                    "GPU système détecté mais invisible pour llama.cpp. \
                     Recompilez avec: cargo build --release --features metal".to_string()
                );
            }
        }
    }

    None
}
