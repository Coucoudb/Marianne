// frontend/scripts/app.js
// Application principale Marianne — Frontend Tauri

const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;

// ─── Sécurité : sanitization du HTML généré par marked ─────────────────────────
function sanitizeHtml(html) {
    // Remove javascript: links, on* event handlers, and dangerous tags
    const div = document.createElement('div');
    div.innerHTML = html;
    // Remove script/iframe/object/embed tags
    div.querySelectorAll('script, iframe, object, embed, form').forEach(el => el.remove());
    // Remove event handlers and javascript: hrefs
    div.querySelectorAll('*').forEach(el => {
        for (const attr of [...el.attributes]) {
            if (attr.name.startsWith('on') || 
                (attr.name === 'href' && attr.value.trim().toLowerCase().startsWith('javascript:')) ||
                (attr.name === 'src' && attr.value.trim().toLowerCase().startsWith('javascript:'))) {
                el.removeAttribute(attr.name);
            }
        }
    });
    return div.innerHTML;
}

function safeMarkedParse(text) {
    return sanitizeHtml(marked.parse(text));
}

// ─── État de l'application ─────────────────────────────────────────────────────
const state = {
    isModelLoaded: false,
    isGenerating: false,
    currentConversationId: null,
    currentStreamingMessage: null,
    tokenBuffer: '',
    stagedFiles: [],  // fichiers en attente d'envoi [{path, name}]
};

// ─── Éléments DOM ──────────────────────────────────────────────────────────────
const elements = {
    messages: document.getElementById('messages'),
    userInput: document.getElementById('user-input'),
    sendBtn: document.getElementById('send-btn'),
    statusDot: document.querySelector('.status-dot'),
    statusText: document.querySelector('.status-text'),
    setupModal: document.getElementById('setup-modal'),
    downloadBtn: document.getElementById('download-btn'),
    downloadProgress: document.getElementById('download-progress'),
    progressFill: document.getElementById('progress-fill'),
    progressText: document.getElementById('progress-text'),
    settingsBtn: document.getElementById('settings-btn'),
    settingsPanel: document.getElementById('settings-panel'),
    settingsDevice: document.getElementById('settings-device'),
    settingsModel: document.getElementById('settings-model'),
    toggleGpu: document.getElementById('toggle-gpu'),
    toggleCpu: document.getElementById('toggle-cpu'),
    gpuSelect: document.getElementById('gpu-select'),
    gpuSelectionSection: document.getElementById('gpu-selection-section'),
    settingsHint: document.getElementById('settings-hint'),
};

// ─── Initialisation ────────────────────────────────────────────────────────────
document.addEventListener('DOMContentLoaded', async () => {
    setupEventListeners();
    setupTauriListeners();
    await checkModelStatus();
});

function setupEventListeners() {
    // Envoi par Enter (Shift+Enter pour retour à la ligne)
    elements.userInput.addEventListener('keydown', (e) => {
        if (e.key === 'Enter' && !e.shiftKey) {
            e.preventDefault();
            sendMessage();
        }
    });

    // Bouton envoyer (géré dynamiquement via onclick pour alterner send/stop)
    elements.sendBtn.onclick = sendMessage;

    // Activer/désactiver le bouton selon le contenu
    elements.userInput.addEventListener('input', () => {
        updateSendButtonState();
        autoResizeTextarea();
    });

    // Bouton téléchargement
    elements.downloadBtn.addEventListener('click', downloadModel);

    // Bouton upload document
    document.getElementById('upload-btn').addEventListener('click', openDocumentPicker);

    // Bouton paramètres
    elements.settingsBtn.addEventListener('click', toggleSettings);
    document.addEventListener('click', (e) => {
        if (!elements.settingsPanel.contains(e.target) && !elements.settingsBtn.contains(e.target)) {
            closeSettings();
        }
    });

    // Toggle GPU / CPU
    elements.toggleGpu.addEventListener('click', () => {
        if (!elements.toggleGpu.disabled) setDevicePreference('Gpu');
    });
    elements.toggleCpu.addEventListener('click', () => {
        setDevicePreference('Cpu');
    });

    // Sélection GPU spécifique
    elements.gpuSelect.addEventListener('change', () => {
        setGpuSelection(elements.gpuSelect.value);
    });

    // Drag & drop sur la zone de messages
    setupDragAndDrop();
}

function setupTauriListeners() {
    // Streaming de tokens
    listen('stream-token', ({ payload }) => {
        if (!state.currentStreamingMessage) return;

        // Effacer l'indicateur "réfléchit" au premier token reçu
        if (state.tokenBuffer === '') {
            const contentEl = state.currentStreamingMessage.querySelector('.message-content');
            if (contentEl) contentEl.innerHTML = '';
        }

        state.tokenBuffer += payload.token;
        const contentEl = state.currentStreamingMessage.querySelector('.message-content');
        if (contentEl) {
            contentEl.innerHTML = safeMarkedParse(state.tokenBuffer);
            scrollToBottom();
        }
    });

    // Fin de génération
    listen('generation-done', ({ payload }) => {
        if (!state.currentStreamingMessage) return;

        state.currentStreamingMessage.classList.remove('streaming');

        // Ré-afficher la réponse nettoyée (supprime les notes parasites du streaming)
        const contentEl = state.currentStreamingMessage.querySelector('.message-content');
        if (contentEl && payload.full_response) {
            contentEl.innerHTML = safeMarkedParse(payload.full_response);
        }

        // Pied de message : sources + stats
        const footerEl = document.createElement('div');
        footerEl.className = 'message-footer';

        // Sources cliquables avec nom de domaine lisible
        if (payload.sources && payload.sources.length > 0) {
            const sourcesEl = document.createElement('div');
            sourcesEl.className = 'sources-list';
            sourcesEl.innerHTML = '<span class="sources-label">📚 Sources</span>' +
                '<div class="sources-chips">' +
                payload.sources.map(s => {
                    const display = formatSourceLabel(s);
                    return `<a class="source-chip" href="#" data-url="${s}" title="${s}">${display}</a>`;
                }).join('') +
                '</div>';
            // Ouvrir dans le navigateur système via Tauri shell
            sourcesEl.querySelectorAll('.source-chip').forEach(chip => {
                chip.addEventListener('click', (e) => {
                    e.preventDefault();
                    const url = chip.dataset.url;
                    if (url && window.__TAURI__?.shell?.open) {
                        window.__TAURI__.shell.open(url);
                    }
                });
            });
            footerEl.appendChild(sourcesEl);
        }

        // Stats compactes
        const statsEl = document.createElement('div');
        statsEl.className = 'generation-stats';
        statsEl.innerHTML = `
            <span class="stat-item"><span class="stat-icon">⏱️</span>${(payload.time_ms / 1000).toFixed(1)}s</span>
            <span class="stat-item"><span class="stat-icon">📝</span>${payload.tokens_generated} tokens</span>
        `;
        footerEl.appendChild(statsEl);

        state.currentStreamingMessage.appendChild(footerEl);

        // Reset état
        state.isGenerating = false;
        state.currentStreamingMessage = null;
        state.tokenBuffer = '';
        showSendButton();
        elements.userInput.focus();
    });

    // Progression du téléchargement
    listen('download-progress', ({ payload }) => {
        elements.progressFill.style.width = `${payload.percent}%`;
        elements.progressText.textContent =
            `${payload.downloaded_mb} Mo / ${payload.total_mb} Mo (${payload.percent}%)`;

        // Aussi mettre à jour la barre de progression du catalogue de modèles
        const modelFill = document.getElementById('model-progress-fill');
        const modelText = document.getElementById('model-progress-text');
        if (modelFill) modelFill.style.width = `${payload.percent}%`;
        if (modelText) modelText.textContent = `${payload.downloaded_mb}/${payload.total_mb} Mo (${payload.percent}%)`;
    });

    // Modèle prêt
    listen('model-ready', async () => {
        setStatus('ready', 'Marianne est prête');
        state.isModelLoaded = true;
        elements.setupModal.style.display = 'none';
        elements.sendBtn.disabled = !elements.userInput.value.trim();
        await updateDeviceBadge();
        checkCorpusUpdate();
    });

    // Recherche web — confiance
    listen('confidence-info', ({ payload }) => {
        if (!state.currentStreamingMessage) return;
        if (payload.web_search_triggered) {
            const badge = document.createElement('div');
            badge.className = 'web-search-badge';
            badge.innerHTML = `<span class="confidence-score">🔍 Confiance ${Math.round(payload.score * 100)}% — recherche web en cours...</span>`;
            state.currentStreamingMessage.appendChild(badge);
        }
    });

    // Recherche web — statut
    listen('web-search-status', ({ payload }) => {
        if (!state.currentStreamingMessage) return;
        const badge = state.currentStreamingMessage.querySelector('.web-search-badge');
        if (badge && payload.status === 'done') {
            if (payload.sources_count > 0) {
                badge.innerHTML = `<span class="confidence-score web-done">🌐 ${payload.sources_count} source(s) web officielle(s) trouvée(s)</span>`;
            } else {
                badge.innerHTML = `<span class="confidence-score web-empty">⚠️ Aucune source web trouvée — réponse basée sur le corpus local</span>`;
            }
        }
    });

    // Mode hors-ligne — notification claire
    listen('offline-mode', ({ payload }) => {
        if (!state.currentStreamingMessage) return;
        const badge = state.currentStreamingMessage.querySelector('.web-search-badge');
        if (badge) {
            badge.innerHTML = `<span class="confidence-score offline">📡 ${payload.message}</span>`;
        }
    });

    // Contradiction web/corpus — alerte utilisateur
    listen('contradiction-warning', ({ payload }) => {
        if (!state.currentStreamingMessage) return;
        const warning = document.createElement('div');
        warning.className = 'contradiction-badge';
        warning.innerHTML = `<span class="contradiction-text">${payload.message}</span>`;
        state.currentStreamingMessage.appendChild(warning);
    });

    // Mise à jour corpus — notification
    listen('corpus-update-status', ({ payload }) => {
        if (payload.status === 'done' && payload.updated > 0) {
            showCorpusUpdateToast(payload.updated);
        }
    });
}

// ─── Logique métier ────────────────────────────────────────────────────────────
async function checkModelStatus() {
    try {
        const status = await invoke('check_model_status');

        if (!status.model_downloaded) {
            elements.setupModal.style.display = 'flex';
            setStatus('loading', 'Modèle non installé');
        } else if (!status.model_loaded) {
            setStatus('loading', 'Chargement du modèle...');
            await invoke('load_model');
            setStatus('loading', 'Initialisation du RAG...');
            await invoke('initialize_rag').catch(e => console.warn('RAG init:', e));
            state.isModelLoaded = true;
            setStatus('ready', 'Marianne est prête');
            await updateDeviceBadge();
            checkCorpusUpdate();
        } else {
            state.isModelLoaded = true;
            setStatus('ready', 'Marianne est prête');
            await updateDeviceBadge();
        }
    } catch (error) {
        console.error('Erreur init:', error);
        // En cas d'erreur, proposer le téléchargement
        elements.setupModal.style.display = 'flex';
        setStatus('error', `Erreur : ${error}`);
    }
}

async function downloadModel() {
    elements.downloadBtn.disabled = true;
    elements.downloadBtn.textContent = 'Téléchargement en cours...';
    elements.downloadProgress.style.display = 'block';

    try {
        await invoke('download_model');
        setStatus('loading', 'Chargement du modèle...');
        await invoke('load_model');
        setStatus('loading', 'Initialisation du RAG...');
        await invoke('initialize_rag').catch(e => console.warn('RAG init:', e));
        state.isModelLoaded = true;
        setStatus('ready', 'Marianne est prête');
        elements.setupModal.style.display = 'none';
        await updateDeviceBadge();
        checkCorpusUpdate();
    } catch (error) {
        setStatus('error', `Erreur : ${error}`);
        elements.downloadBtn.disabled = false;
        elements.downloadBtn.textContent = 'Réessayer';
    }
}

async function sendMessage() {
    const message = elements.userInput.value.trim();
    const hasFiles = state.stagedFiles.length > 0;
    if ((!message && !hasFiles) || state.isGenerating || !state.isModelLoaded) return;

    // Si des fichiers sont en attente, déléguer à la logique documents
    if (hasFiles) {
        return sendMessageWithDocuments();
    }

    state.isGenerating = true;
    showStopButton();

    // Afficher le message utilisateur
    appendMessage('user', message);
    elements.userInput.value = '';
    autoResizeTextarea();

    // Préparer la zone de réponse (streaming)
    const assistantEl = appendMessage('assistant', '', true);
    state.currentStreamingMessage = assistantEl;
    state.tokenBuffer = '';

    // Indicateur "en réflexion" pendant le prefill
    const contentEl = assistantEl.querySelector('.message-content');
    contentEl.innerHTML = '<span class="thinking">Marianne réfléchit...</span>';

    try {
        const convId = await invoke('send_message', {
            request: {
                message,
                conversation_id: state.currentConversationId,
                max_tokens: 1024,
            },
        });
        state.currentConversationId = convId;
    } catch (error) {
        assistantEl.querySelector('.message-content').textContent =
            `❌ Erreur : ${error}`;
        assistantEl.classList.remove('streaming');
        state.isGenerating = false;
        state.currentStreamingMessage = null;
        showSendButton();
    }
}

async function stopGeneration() {
    if (!state.isGenerating) return;
    try {
        await invoke('stop_generation');
    } catch (e) {
        console.warn('Erreur stop_generation:', e);
    }
}

function showStopButton() {
    elements.sendBtn.disabled = false;
    elements.sendBtn.classList.add('stop-mode');
    elements.sendBtn.innerHTML = `
        <svg width="18" height="18" viewBox="0 0 24 24" fill="currentColor">
            <rect x="6" y="6" width="12" height="12" rx="2"/>
        </svg>`;
    elements.sendBtn.onclick = stopGeneration;
}

function showSendButton() {
    elements.sendBtn.classList.remove('stop-mode');
    elements.sendBtn.innerHTML = `
        <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M22 2L11 13M22 2l-7 20-4-9-9-4 20-7z"/>
        </svg>`;
    elements.sendBtn.onclick = sendMessage;
    updateSendButtonState();
}

// ─── Helpers UI ────────────────────────────────────────────────────────────────
function appendMessage(role, content, isStreaming = false) {
    // Masquer le message de bienvenue au premier message
    const welcome = elements.messages.querySelector('.welcome-message');
    if (welcome) welcome.remove();

    const messageEl = document.createElement('div');
    messageEl.className = `message ${role}${isStreaming ? ' streaming' : ''}`;

    const contentEl = document.createElement('div');
    contentEl.className = 'message-content';

    if (role === 'assistant' && content) {
        contentEl.innerHTML = safeMarkedParse(content);
    } else {
        contentEl.textContent = content;
    }

    messageEl.appendChild(contentEl);
    elements.messages.appendChild(messageEl);
    scrollToBottom();

    return messageEl;
}

function scrollToBottom() {
    elements.messages.scrollTop = elements.messages.scrollHeight;
}

function setStatus(type, text) {
    elements.statusDot.className = `status-dot ${type === 'ready' ? 'ready' : type === 'error' ? 'error' : ''}`;
    elements.statusText.textContent = text;
}

function autoResizeTextarea() {
    const textarea = elements.userInput;
    textarea.style.height = 'auto';
    textarea.style.height = Math.min(textarea.scrollHeight, 120) + 'px';
}

// ─── Paramètres ────────────────────────────────────────────────────────────────
function toggleSettings() {
    const isOpen = elements.settingsPanel.style.display !== 'none';
    if (isOpen) {
        closeSettings();
    } else {
        elements.settingsPanel.style.display = 'block';
        elements.settingsBtn.classList.add('active');
        loadDevicePreference();
        loadInstalledModels();
    }
}

function closeSettings() {
    elements.settingsPanel.style.display = 'none';
    elements.settingsBtn.classList.remove('active');
}

async function updateDeviceBadge() {
    try {
        const info = await invoke('get_device_info');
        elements.settingsDevice.textContent = info.label;
        // Afficher le nom du modèle actif depuis le registre
        const installed = await invoke('list_installed_models');
        const active = installed.find(e => e.active);
        elements.settingsModel.textContent = active ? active.model.name : 'Aucun';
    } catch (_) {
        elements.settingsDevice.textContent = '—';
        elements.settingsModel.textContent = '—';
    }
}

async function loadDevicePreference() {
    try {
        const pref = await invoke('get_device_preference');

        // Mettre à jour les boutons du toggle
        elements.toggleGpu.classList.toggle('active', pref.preference === 'Gpu');
        elements.toggleCpu.classList.toggle('active', pref.preference === 'Cpu');

        // Désactiver le bouton GPU si pas de GPU disponible
        elements.toggleGpu.disabled = !pref.gpu_available;
        if (!pref.gpu_available) {
            elements.settingsHint.textContent = 'GPU non détecté sur cette machine';
            elements.gpuSelectionSection.style.display = 'none';
        } else {
            elements.settingsHint.textContent = 'Appliqué au prochain démarrage';
            // Charger la liste des GPU si mode GPU actif
            if (pref.preference === 'Gpu') {
                await loadGpuDevices();
            }
        }
    } catch (_) {
        // Silencieux
    }
}

async function loadGpuDevices() {
    try {
        const gpuInfo = await invoke('list_gpu_devices');
        const select = elements.gpuSelect;

        // Ne montrer la section que s'il y a plusieurs GPU
        if (gpuInfo.devices.length > 1) {
            elements.gpuSelectionSection.style.display = '';

            // Reconstruire les options
            select.innerHTML = '';
            const autoOpt = document.createElement('option');
            autoOpt.value = 'Auto';
            autoOpt.textContent = 'Auto (premier détecté)';
            select.appendChild(autoOpt);

            const allOpt = document.createElement('option');
            allOpt.value = 'AllGpus';
            allOpt.textContent = `Tous les GPU (${gpuInfo.devices.length})`;
            select.appendChild(allOpt);

            for (const dev of gpuInfo.devices) {
                const opt = document.createElement('option');
                opt.value = `Specific:${dev.index}`;
                opt.textContent = `${dev.name} (${dev.vram_free_mb} Mo VRAM)`;
                select.appendChild(opt);
            }

            // Sélectionner la valeur actuelle
            if (gpuInfo.selection === 'Auto') {
                select.value = 'Auto';
            } else if (gpuInfo.selection === 'AllGpus') {
                select.value = 'AllGpus';
            } else if (gpuInfo.selection && gpuInfo.selection.Specific !== undefined) {
                select.value = `Specific:${gpuInfo.selection.Specific}`;
            }
        } else {
            elements.gpuSelectionSection.style.display = 'none';
        }
    } catch (e) {
        console.warn('Erreur chargement GPU:', e);
        elements.gpuSelectionSection.style.display = 'none';
    }
}

async function setGpuSelection(value) {
    try {
        let selection;
        if (value === 'Auto') {
            selection = 'Auto';
        } else if (value === 'AllGpus') {
            selection = 'AllGpus';
        } else if (value.startsWith('Specific:')) {
            const idx = parseInt(value.split(':')[1], 10);
            selection = { Specific: idx };
        } else {
            return;
        }
        await invoke('set_gpu_selection', { selection });
        elements.settingsHint.textContent = '✓ Appliqué au prochain démarrage';
    } catch (e) {
        console.warn('Erreur sauvegarde sélection GPU:', e);
    }
}

async function setDevicePreference(preference) {
    try {
        await invoke('set_device_preference', { preference });
        elements.toggleGpu.classList.toggle('active', preference === 'Gpu');
        elements.toggleCpu.classList.toggle('active', preference === 'Cpu');
        elements.settingsHint.textContent = '✓ Appliqué au prochain démarrage';

        // Montrer/masquer la sélection GPU selon le mode
        if (preference === 'Gpu') {
            await loadGpuDevices();
        } else {
            elements.gpuSelectionSection.style.display = 'none';
        }
    } catch (e) {
        console.warn('Erreur sauvegarde préférence device:', e);
    }
}

// ─── Gestion des modèles — HuggingFace ─────────────────────────────────────────
let modelDownloading = false;
let searchTimeout = null;

async function loadInstalledModels() {
    const container = document.getElementById('model-catalog');
    if (!container) return;

    try {
        const installed = await invoke('list_installed_models');
        container.innerHTML = '';

        if (installed.length === 0) {
            container.innerHTML = '<p class="settings-hint">Aucun modèle installé</p>';
            return;
        }

        for (const entry of installed) {
            const card = document.createElement('div');
            card.className = `model-card${entry.active ? ' active' : ''}`;

            const badgeClass = entry.active ? 'active' : 'downloaded';
            const badgeText = entry.active ? 'Actif' : `${entry.model.size_mb} Mo`;

            card.innerHTML = `
                <div class="model-card-header">
                    <span class="model-card-name">${escapeHtml(entry.model.name)}</span>
                    <span class="model-card-badge ${badgeClass}">${badgeText}</span>
                </div>
                <div class="model-card-meta">
                    <span>${escapeHtml(entry.model.repo_id)}</span>
                </div>
                <div class="model-card-actions"></div>
            `;

            const actionsDiv = card.querySelector('.model-card-actions');

            if (!entry.active) {
                const activateBtn = document.createElement('button');
                activateBtn.className = 'model-btn primary';
                activateBtn.textContent = 'Activer';
                activateBtn.addEventListener('click', () => activateInstalledModel(entry.model.id));
                actionsDiv.appendChild(activateBtn);
            }

            const delBtn = document.createElement('button');
            delBtn.className = 'model-btn danger';
            delBtn.textContent = 'Supprimer';
            delBtn.addEventListener('click', () => deleteInstalledModel(entry.model.id));
            actionsDiv.appendChild(delBtn);

            container.appendChild(card);
        }
    } catch (e) {
        container.innerHTML = '<p class="settings-hint">Impossible de charger les modèles</p>';
        console.warn('Erreur chargement modèles:', e);
    }
}

async function activateInstalledModel(modelId) {
    try {
        await invoke('select_model', { modelId });
        loadInstalledModels();
        updateDeviceBadge();
        const hint = document.getElementById('settings-hint');
        if (hint) hint.textContent = '⚠ Redémarrez pour charger le nouveau modèle';
    } catch (e) {
        alert('Erreur : ' + e);
    }
}

async function deleteInstalledModel(modelId) {
    if (!confirm('Supprimer ce modèle ? Vous devrez le retélécharger pour l\'utiliser à nouveau.')) {
        return;
    }
    try {
        await invoke('delete_model', { modelId });
        loadInstalledModels();
        updateDeviceBadge();
    } catch (e) {
        alert('Erreur lors de la suppression : ' + e);
    }
}

// ─── Recherche HuggingFace ─────────────────────────────────────────────────────
function onHfSearchInput(e) {
    const query = e.target.value.trim();
    clearTimeout(searchTimeout);
    if (query.length < 2) {
        document.getElementById('hf-search-results').innerHTML = '';
        return;
    }
    // Debounce 500ms
    searchTimeout = setTimeout(() => searchHuggingFace(query), 500);
}

async function searchHuggingFace(query) {
    const resultsContainer = document.getElementById('hf-search-results');
    resultsContainer.innerHTML = '<p class="settings-hint">Recherche en cours...</p>';

    try {
        const results = await invoke('search_huggingface', { query });
        resultsContainer.innerHTML = '';

        if (results.length === 0) {
            resultsContainer.innerHTML = '<p class="settings-hint">Aucun modèle GGUF trouvé</p>';
            return;
        }

        for (const result of results) {
            const card = document.createElement('div');
            card.className = 'model-card hf-result';

            const downloadsFormatted = formatNumber(result.downloads);
            const likesFormatted = formatNumber(result.likes);

            card.innerHTML = `
                <div class="model-card-header">
                    <span class="model-card-name">${escapeHtml(result.name)}</span>
                    <span class="model-card-badge not-downloaded">⬇ ${downloadsFormatted}</span>
                </div>
                <div class="model-card-desc">${escapeHtml(result.repo_id)}</div>
                <div class="model-card-meta">
                    <span>❤️ ${likesFormatted}</span>
                    <span>•</span>
                    <span>${escapeHtml(result.description)}</span>
                </div>
                <div class="model-card-actions">
                    <button class="model-btn primary">Voir les fichiers</button>
                </div>
            `;

            card.querySelector('.model-btn').addEventListener('click', () => {
                showGgufFiles(result.repo_id, result.name);
            });

            resultsContainer.appendChild(card);
        }
    } catch (e) {
        resultsContainer.innerHTML = `<p class="settings-hint">Erreur : ${e}</p>`;
    }
}

async function showGgufFiles(repoId, modelName) {
    const resultsContainer = document.getElementById('hf-search-results');
    resultsContainer.innerHTML = '<p class="settings-hint">Chargement des fichiers GGUF...</p>';

    try {
        const files = await invoke('get_model_gguf_files', { repoId });
        resultsContainer.innerHTML = '';

        if (files.length === 0) {
            resultsContainer.innerHTML = '<p class="settings-hint">Aucun fichier GGUF trouvé dans ce repo</p>';
            return;
        }

        // Bouton retour
        const backBtn = document.createElement('button');
        backBtn.className = 'model-btn';
        backBtn.textContent = '← Retour aux résultats';
        backBtn.style.marginBottom = '8px';
        backBtn.addEventListener('click', () => {
            const input = document.getElementById('hf-search-input');
            if (input && input.value.trim()) searchHuggingFace(input.value.trim());
        });
        resultsContainer.appendChild(backBtn);

        // Titre
        const title = document.createElement('p');
        title.className = 'settings-hint';
        title.style.fontWeight = '600';
        title.style.color = 'var(--text-primary)';
        title.textContent = `${modelName} — ${files.length} fichier(s) GGUF`;
        resultsContainer.appendChild(title);

        for (const file of files) {
            const card = document.createElement('div');
            card.className = 'model-card';

            card.innerHTML = `
                <div class="model-card-header">
                    <span class="model-card-name">${escapeHtml(file.quantization)}</span>
                    <span class="model-card-badge not-downloaded">${file.size_mb} Mo</span>
                </div>
                <div class="model-card-desc">${escapeHtml(file.filename)}</div>
                <div class="model-card-actions">
                    <button class="model-btn primary">Installer</button>
                </div>
            `;

            card.querySelector('.model-btn').addEventListener('click', () => {
                installHfModel(repoId, file.filename, `${modelName} (${file.quantization})`);
            });

            resultsContainer.appendChild(card);
        }
    } catch (e) {
        resultsContainer.innerHTML = `<p class="settings-hint">Erreur : ${e}</p>`;
    }
}

async function installHfModel(repoId, filename, name) {
    if (modelDownloading) return;
    modelDownloading = true;

    const progressEl = document.getElementById('model-progress');
    const progressFill = document.getElementById('model-progress-fill');
    const progressText = document.getElementById('model-progress-text');
    const actionsEl = document.getElementById('model-actions');

    if (progressEl) progressEl.style.display = 'flex';
    if (actionsEl) actionsEl.style.display = 'block';
    if (progressFill) progressFill.style.width = '0%';
    if (progressText) progressText.textContent = 'Démarrage...';

    try {
        await invoke('download_hf_model', { repoId, filename, name });
        if (progressText) progressText.textContent = '✓ Installé !';
        setTimeout(() => {
            if (progressEl) progressEl.style.display = 'none';
            if (actionsEl) actionsEl.style.display = 'none';
            loadInstalledModels();
            updateDeviceBadge();
            // Vider les résultats de recherche
            document.getElementById('hf-search-results').innerHTML = '';
            document.getElementById('hf-search-input').value = '';
        }, 1500);
    } catch (e) {
        if (progressText) progressText.textContent = `Erreur : ${e}`;
        console.error('Erreur installation modèle:', e);
    } finally {
        modelDownloading = false;
    }
}

function formatNumber(n) {
    if (n >= 1_000_000) return (n / 1_000_000).toFixed(1) + 'M';
    if (n >= 1_000) return (n / 1_000).toFixed(1) + 'k';
    return n.toString();
}

function escapeHtml(text) {
    const div = document.createElement('div');
    div.textContent = text;
    return div.innerHTML;
}

// ─── Documents : upload et drag & drop ─────────────────────────────────────────
async function openDocumentPicker() {
    if (state.isGenerating || !state.isModelLoaded) return;

    try {
        const { open } = window.__TAURI__.dialog;
        const selected = await open({
            filters: [{ name: 'Documents', extensions: ['pdf', 'txt', 'md'] }],
            multiple: true,
        });
        if (selected) {
            // `selected` est un tableau si multiple: true, ou une string si un seul fichier
            const paths = Array.isArray(selected) ? selected : [selected];
            for (const filePath of paths) {
                stageFile(filePath);
            }
        }
    } catch (error) {
        console.error('Erreur sélection fichier:', error);
    }
}

function stageFile(filePath) {
    // Éviter les doublons
    if (state.stagedFiles.some(f => f.path === filePath)) return;

    const name = filePath.split(/[\\/]/).pop() || 'document';
    state.stagedFiles.push({ path: filePath, name });
    renderStagedFiles();
    updateSendButtonState();
    elements.userInput.focus();
}

function removeStagedFile(filePath) {
    state.stagedFiles = state.stagedFiles.filter(f => f.path !== filePath);
    renderStagedFiles();
    updateSendButtonState();
}

function renderStagedFiles() {
    const container = document.getElementById('staged-files');
    if (state.stagedFiles.length === 0) {
        container.style.display = 'none';
        container.innerHTML = '';
        return;
    }

    container.style.display = 'flex';
    container.innerHTML = state.stagedFiles.map(f => `
        <div class="staged-file-chip" data-path="${f.path.replace(/"/g, '&quot;')}">
            <span class="file-icon">📄</span>
            <span class="file-name" title="${f.name}">${f.name}</span>
            <button class="remove-file" title="Retirer">&times;</button>
        </div>
    `).join('');

    container.querySelectorAll('.remove-file').forEach(btn => {
        btn.addEventListener('click', (e) => {
            e.stopPropagation();
            const chip = btn.closest('.staged-file-chip');
            removeStagedFile(chip.dataset.path);
        });
    });
}

function updateSendButtonState() {
    const hasText = elements.userInput.value.trim().length > 0;
    const hasFiles = state.stagedFiles.length > 0;
    elements.sendBtn.disabled = (!hasText && !hasFiles) || state.isGenerating;
}

async function sendMessageWithDocuments() {
    const message = elements.userInput.value.trim();
    const files = [...state.stagedFiles];

    // Réinitialiser les fichiers en attente
    state.stagedFiles = [];
    renderStagedFiles();

    state.isGenerating = true;
    showStopButton();

    // Afficher le message utilisateur avec les fichiers
    const fileLabels = files.map(f => `📄 ${f.name}`).join(', ');
    const displayMessage = message
        ? `${fileLabels}\n\n${message}`
        : fileLabels;
    appendMessage('user', displayMessage);
    elements.userInput.value = '';
    autoResizeTextarea();

    // Préparer la zone de réponse
    const assistantEl = appendMessage('assistant', '', true);
    state.currentStreamingMessage = assistantEl;
    state.tokenBuffer = '';

    const contentEl = assistantEl.querySelector('.message-content');
    contentEl.innerHTML = '<span class="thinking">Marianne analyse le(s) document(s)...</span>';

    try {
        // Extraire le texte de chaque fichier
        const extractions = [];
        for (const file of files) {
            const result = await invoke('extract_document', {
                request: { file_path: file.path, question: null },
            });
            extractions.push(result);
        }

        // Construire le prompt combiné
        let prompt;
        if (extractions.length === 1) {
            const doc = extractions[0];
            const question = message || "Explique ce document en langage clair et dis-moi ce que je dois faire.";
            prompt = `Voici un document administratif français (${doc.file_name}) :\n\n---\n${doc.text}\n---\n\nQuestion : ${question}`;
        } else {
            const docsText = extractions.map((doc, i) =>
                `── Document ${i + 1} : ${doc.file_name} ──\n${doc.text}`
            ).join('\n\n');
            const question = message || "Explique ces documents en langage clair et dis-moi ce que je dois faire.";
            prompt = `Voici ${extractions.length} documents administratifs français :\n\n${docsText}\n\n---\n\nQuestion : ${question}`;
        }

        // Envoyer via le pipeline chat normal
        const convId = await invoke('send_message', {
            request: {
                message: prompt,
                conversation_id: state.currentConversationId,
                max_tokens: 1024,
            },
        });
        state.currentConversationId = convId;
    } catch (error) {
        assistantEl.querySelector('.message-content').textContent = `❌ ${error}`;
        assistantEl.classList.remove('streaming');
        state.isGenerating = false;
        state.currentStreamingMessage = null;
        showSendButton();
    }
}

function setupDragAndDrop() {
    const messagesEl = elements.messages;

    messagesEl.addEventListener('dragover', (e) => {
        e.preventDefault();
        messagesEl.classList.add('drag-active');
    });

    messagesEl.addEventListener('dragleave', (e) => {
        if (!messagesEl.contains(e.relatedTarget)) {
            messagesEl.classList.remove('drag-active');
        }
    });

    messagesEl.addEventListener('drop', async (e) => {
        e.preventDefault();
        messagesEl.classList.remove('drag-active');

        const files = e.dataTransfer?.files;
        if (files && files.length > 0) {
            for (const file of files) {
                const ext = file.name.split('.').pop()?.toLowerCase();
                if (['pdf', 'txt', 'md'].includes(ext)) {
                    if (file.path) {
                        stageFile(file.path);
                    }
                } else {
                    appendMessage('assistant', `⚠️ Fichier « ${file.name} » ignoré — format non supporté. Utilisez PDF, TXT ou MD.`);
                }
            }
        }
    });
}

// ─── Corpus : mise à jour automatique ──────────────────────────────────────────
function showCorpusUpdateToast(updatedCount) {
    const toast = document.createElement('div');
    toast.className = 'corpus-toast';
    toast.innerHTML = `📚 Corpus légal mis à jour — ${updatedCount} fiche(s) actualisée(s)`;
    document.body.appendChild(toast);
    setTimeout(() => {
        toast.classList.add('fade-out');
        setTimeout(() => toast.remove(), 500);
    }, 5000);
}

async function checkCorpusUpdate() {
    try {
        const needsUpdate = await invoke('check_corpus_update');
        if (needsUpdate && state.isModelLoaded) {
            // Lancer la mise à jour en arrière-plan (non bloquant)
            invoke('update_corpus').catch(e =>
                console.warn('Mise à jour corpus échouée:', e)
            );
        }
    } catch (_) {
        // Silencieux si échec
    }
}

// ─── Formatage des sources ─────────────────────────────────────────────────────
const SOURCE_LABELS = {
    'service-public.gouv.fr': 'Service-Public.fr',
    'legifrance.gouv.fr': 'Légifrance',
    'urssaf.fr': 'URSSAF',
    'caf.fr': 'CAF',
    'ameli.fr': 'Ameli',
    'francetravail.fr': 'France Travail',
    'impots.gouv.fr': 'Impôts',
    'info-retraite.fr': 'Info Retraite',
    'ants.gouv.fr': 'ANTS',
    'france-renov.gouv.fr': 'France Rénov',
    'defenseurdesdroits.fr': 'Défenseur des Droits',
    'justice.fr': 'Justice.fr',
    'rappel.conso.gouv.fr': 'RappelConso',
    'info.gouv.fr': 'Info.gouv',
    'data.gouv.fr': 'Data.gouv',
    'assemblee-nationale.fr': 'Assemblée nationale',
    'senat.fr': 'Sénat',
    'vie-publique.fr': 'Vie publique',
    'economie.gouv.fr': 'Economie.gouv',
    'banque-france.fr': 'Banque de France',
    'lafinancepourtous.com': 'La Finance pour Tous',
    'amf-france.org': 'AMF',
    'insee.fr': 'INSEE',
};

function formatSourceLabel(url) {
    try {
        const hostname = new URL(url).hostname.replace(/^www\./, '').replace(/^www2\./, '');
        for (const [domain, label] of Object.entries(SOURCE_LABELS)) {
            if (hostname.includes(domain)) return label;
        }
        // Fallback : nom de domaine nettoyé
        return hostname.replace(/\.gouv\.fr$/, '').replace(/\.fr$/, '').replace(/\.com$/, '');
    } catch {
        return url.length > 40 ? url.substring(0, 37) + '…' : url;
    }
}
