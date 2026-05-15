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

    // Bouton envoyer
    elements.sendBtn.addEventListener('click', sendMessage);

    // Activer/désactiver le bouton selon le contenu
    elements.userInput.addEventListener('input', () => {
        elements.sendBtn.disabled = !elements.userInput.value.trim() || state.isGenerating;
        autoResizeTextarea();
    });

    // Bouton téléchargement
    elements.downloadBtn.addEventListener('click', downloadModel);
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

        // Afficher les stats et sources
        const statsEl = document.createElement('div');
        statsEl.className = 'generation-stats';
        statsEl.innerHTML = `
            <span>⏱️ ${(payload.time_ms / 1000).toFixed(1)}s</span>
            <span>📝 ${payload.tokens_generated} tokens</span>
        `;

        if (payload.sources && payload.sources.length > 0) {
            const sourcesEl = document.createElement('div');
            sourcesEl.className = 'sources-list';
            sourcesEl.innerHTML = '📚 Sources : ' +
                payload.sources.map(s => `<span>${s}</span>`).join('');
            statsEl.appendChild(sourcesEl);
        }

        state.currentStreamingMessage.appendChild(statsEl);

        // Reset état
        state.isGenerating = false;
        state.currentStreamingMessage = null;
        state.tokenBuffer = '';
        elements.sendBtn.disabled = !elements.userInput.value.trim();
        elements.userInput.focus();
    });

    // Progression du téléchargement
    listen('download-progress', ({ payload }) => {
        elements.progressFill.style.width = `${payload.percent}%`;
        elements.progressText.textContent =
            `${payload.downloaded_mb} Mo / ${payload.total_mb} Mo (${payload.percent}%)`;
    });

    // Modèle prêt
    listen('model-ready', async () => {
        setStatus('ready', 'Marianne est prête');
        state.isModelLoaded = true;
        elements.setupModal.style.display = 'none';
        elements.sendBtn.disabled = !elements.userInput.value.trim();
        await updateDeviceBadge();
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
    elements.sendBtn.disabled = true;

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
        elements.sendBtn.disabled = false;
    }
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

const DEVICE_META = {
    cuda:  { icon: '🟢', label: 'GPU CUDA' },
    metal: { icon: '🔵', label: 'GPU Metal' },
    cpu:   { icon: '🟠', label: 'CPU' },
};

async function updateDeviceBadge() {
    try {
        const info = await invoke('get_device_info');
        const badge = document.getElementById('device-badge');
        const iconEl = document.getElementById('device-icon');
        const labelEl = document.getElementById('device-label');
        const meta = DEVICE_META[info.backend] || DEVICE_META.cpu;

        badge.className = `device-badge ${info.backend}`;
        iconEl.textContent = meta.icon;
        labelEl.textContent = info.label;
        badge.style.display = 'flex';
    } catch (_) {
        // Modèle pas encore chargé — on masque le badge
    }
}
