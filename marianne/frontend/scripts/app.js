// frontend/scripts/app.js
// Application principale Marianne — Frontend Tauri

const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;

// ─── État de l'application ─────────────────────────────────────────────────────
const state = {
    isModelLoaded: false,
    isGenerating: false,
    currentConversationId: null,
    currentStreamingMessage: null,
    tokenBuffer: '',
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
        elements.sendBtn.disabled = !elements.userInput.value.trim() || state.isGenerating;
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
            contentEl.innerHTML = marked.parse(state.tokenBuffer);
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
            contentEl.innerHTML = marked.parse(payload.full_response);
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
    if (!message || state.isGenerating || !state.isModelLoaded) return;

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
    elements.sendBtn.disabled = !elements.userInput.value.trim();
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
        contentEl.innerHTML = marked.parse(content);
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
        loadModelCatalog();
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
        // Afficher le nom du modèle actif depuis le catalogue
        const catalog = await invoke('get_model_catalog');
        const active = catalog.find(e => e.active);
        elements.settingsModel.textContent = active ? active.info.name : 'Aucun';
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
        } else {
            elements.settingsHint.textContent = 'Appliqué au prochain démarrage';
        }
    } catch (_) {
        // Silencieux
    }
}

async function setDevicePreference(preference) {
    try {
        await invoke('set_device_preference', { preference });
        elements.toggleGpu.classList.toggle('active', preference === 'Gpu');
        elements.toggleCpu.classList.toggle('active', preference === 'Cpu');
        elements.settingsHint.textContent = '✓ Appliqué au prochain démarrage';
    } catch (e) {
        console.warn('Erreur sauvegarde préférence device:', e);
    }
}

// ─── Gestion des modèles ───────────────────────────────────────────────────────
let modelDownloading = false;

async function loadModelCatalog() {
    const container = document.getElementById('model-catalog');
    if (!container) return;

    try {
        const catalog = await invoke('get_model_catalog');
        container.innerHTML = '';

        for (const entry of catalog) {
            const card = document.createElement('div');
            card.className = `model-card${entry.active ? ' active' : ''}`;

            let badgeClass = 'not-downloaded';
            let badgeText = `${entry.info.size_mb} Mo`;
            if (entry.active) {
                badgeClass = 'active';
                badgeText = 'Actif';
            } else if (entry.downloaded) {
                badgeClass = 'downloaded';
                badgeText = 'Téléchargé';
            }

            card.innerHTML = `
                <div class="model-card-header">
                    <span class="model-card-name">${entry.info.name}</span>
                    <span class="model-card-badge ${badgeClass}">${badgeText}</span>
                </div>
                <div class="model-card-desc">${entry.info.description}</div>
                <div class="model-card-meta">
                    <span>${entry.info.parameters}</span>
                    <span>•</span>
                    <span>Contexte : ${entry.info.context_length} tokens</span>
                </div>
                <div class="model-card-actions"></div>
            `;

            const actionsDiv = card.querySelector('.model-card-actions');

            if (entry.active) {
                const delBtn = document.createElement('button');
                delBtn.className = 'model-btn danger';
                delBtn.textContent = 'Supprimer';
                delBtn.addEventListener('click', () => deleteCatalogModel(entry.info.id));
                actionsDiv.appendChild(delBtn);
            } else if (entry.downloaded) {
                const activateBtn = document.createElement('button');
                activateBtn.className = 'model-btn primary';
                activateBtn.textContent = 'Activer';
                activateBtn.addEventListener('click', () => selectCatalogModel(entry.info.id));
                actionsDiv.appendChild(activateBtn);

                const delBtn = document.createElement('button');
                delBtn.className = 'model-btn danger';
                delBtn.textContent = 'Supprimer';
                delBtn.addEventListener('click', () => deleteCatalogModel(entry.info.id));
                actionsDiv.appendChild(delBtn);
            } else {
                const dlBtn = document.createElement('button');
                dlBtn.className = 'model-btn primary';
                dlBtn.textContent = 'Télécharger';
                dlBtn.addEventListener('click', () => downloadCatalogModel(entry.info.id));
                actionsDiv.appendChild(dlBtn);
            }

            container.appendChild(card);
        }
    } catch (e) {
        container.innerHTML = '<p class="settings-hint">Impossible de charger le catalogue</p>';
        console.warn('Erreur chargement catalogue:', e);
    }
}

async function downloadCatalogModel(modelId) {
    if (modelDownloading) return;
    modelDownloading = true;

    const progressEl = document.getElementById('model-progress');
    const progressFill = document.getElementById('model-progress-fill');
    const progressText = document.getElementById('model-progress-text');
    const actionsEl = document.getElementById('model-actions');

    if (progressEl) progressEl.style.display = 'flex';
    if (actionsEl) actionsEl.style.display = 'block';

    try {
        await invoke('download_selected_model', { modelId });
        if (progressText) progressText.textContent = '✓ Terminé';
        // Recharger le catalogue
        setTimeout(() => {
            if (progressEl) progressEl.style.display = 'none';
            if (actionsEl) actionsEl.style.display = 'none';
            loadModelCatalog();
        }, 1500);
    } catch (e) {
        if (progressText) progressText.textContent = 'Erreur';
        console.error('Erreur téléchargement modèle:', e);
    } finally {
        modelDownloading = false;
    }
}

async function deleteCatalogModel(modelId) {
    if (!confirm('Supprimer ce modèle ? Vous devrez le retélécharger pour l\'utiliser à nouveau.')) {
        return;
    }

    try {
        await invoke('delete_model', { modelId });
        loadModelCatalog();
        updateDeviceBadge();
    } catch (e) {
        alert('Erreur lors de la suppression : ' + e);
    }
}

async function selectCatalogModel(modelId) {
    try {
        await invoke('select_model', { modelId });
        loadModelCatalog();
        updateDeviceBadge();
        const hint = document.getElementById('settings-hint');
        if (hint) hint.textContent = '⚠ Redémarrez pour charger le nouveau modèle';
    } catch (e) {
        alert('Erreur : ' + e);
    }
}

// ─── Documents : upload et drag & drop ─────────────────────────────────────────
async function openDocumentPicker() {
    if (state.isGenerating || !state.isModelLoaded) return;

    try {
        const { open } = window.__TAURI__.dialog;
        const selected = await open({
            filters: [{ name: 'Documents', extensions: ['pdf', 'txt', 'md'] }],
            multiple: false,
        });
        if (selected) {
            await analyzeDocument(selected);
        }
    } catch (error) {
        console.error('Erreur sélection fichier:', error);
    }
}

async function analyzeDocument(filePath) {
    if (state.isGenerating || !state.isModelLoaded) return;

    try {
        const result = await invoke('extract_document', {
            request: { file_path: filePath, question: null },
        });

        // Afficher le fichier comme message utilisateur
        appendMessage('user', `📄 Analyse de **${result.file_name}** (${result.char_count} caractères)`);

        // Envoyer le prompt d'analyse via le pipeline chat normal
        state.isGenerating = true;
        elements.sendBtn.disabled = true;

        const assistantEl = appendMessage('assistant', '', true);
        state.currentStreamingMessage = assistantEl;
        state.tokenBuffer = '';

        const contentEl = assistantEl.querySelector('.message-content');
        contentEl.innerHTML = '<span class="thinking">Marianne analyse le document...</span>';

        const convId = await invoke('send_message', {
            request: {
                message: result.prompt,
                conversation_id: state.currentConversationId,
                max_tokens: 1024,
            },
        });
        state.currentConversationId = convId;
    } catch (error) {
        appendMessage('assistant', `❌ ${error}`);
        state.isGenerating = false;
        elements.sendBtn.disabled = false;
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

        // Tauri 2 : les fichiers droppés sont dans l'événement natif
        const files = e.dataTransfer?.files;
        if (files && files.length > 0) {
            const file = files[0];
            const ext = file.name.split('.').pop()?.toLowerCase();
            if (['pdf', 'txt', 'md'].includes(ext)) {
                // En Tauri 2, on utilise le path natif si disponible
                if (file.path) {
                    await analyzeDocument(file.path);
                }
            } else {
                appendMessage('assistant', '⚠️ Format non supporté. Utilisez un fichier PDF, TXT ou MD.');
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
