<script lang="ts">
  import { onMount } from 'svelte';
  import * as backend from '../lib/backend';
  import type { UserProfile, ProfessionalStatus, FamilyStatus, LanguageLevel, DevicePreference, GpuSelection } from '../lib/types';

  let profile: UserProfile = {
    first_name: '',
    age: null,
    professional_status: null,
    family_status: null,
    department: null,
    topics_of_interest: [],
    language_level: 'Standard',
    device_preference: 'Gpu',
    gpu_selection: 'Auto',
    selected_model: null,
    updated_at: Date.now() / 1000,
  };

  let loading = true;
  let saving = false;
  let error: string | null = null;
  let success = false;

  // Family status helpers
  let familyStatusType: 'Celibataire' | 'EnCouple' | 'Parent' | 'ParentIsolé' | 'CoupleAvecEnfants' = 'Celibataire';
  let childrenCount = 1;

  const professionalStatuses: ProfessionalStatus[] = [
    'Salarie', 'ChomeurIndemise', 'ChomeurNonIndemise', 'EtudiantApprentis',
    'Retraite', 'Independant', 'FonctionPublique', 'Autre'
  ];

  const topicsOptions = [
    'logement', 'impots', 'travail', 'sante', 'famille', 'retraite', 'chomage', 'allocations'
  ];

  onMount(async () => {
    try {
      profile = await backend.getProfile();
      // Parse family_status
      if (profile.family_status) {
        if (typeof profile.family_status === 'string') {
          familyStatusType = profile.family_status as any;
        } else if (typeof profile.family_status === 'object') {
          if ('Parent' in profile.family_status) {
            familyStatusType = 'Parent';
            childrenCount = profile.family_status.Parent.children_count;
          } else if ('ParentIsolé' in profile.family_status) {
            familyStatusType = 'ParentIsolé';
            childrenCount = profile.family_status.ParentIsolé.children_count;
          } else if ('CoupleAvecEnfants' in profile.family_status) {
            familyStatusType = 'CoupleAvecEnfants';
            childrenCount = profile.family_status.CoupleAvecEnfants.children_count;
          }
        }
      }
    } catch (err) {
      error = `Erreur lors du chargement : ${err}`;
      console.error(err);
    } finally {
      loading = false;
    }
  });

  function toggleTopic(topic: string) {
    if (profile.topics_of_interest.includes(topic)) {
      profile.topics_of_interest = profile.topics_of_interest.filter(t => t !== topic);
    } else {
      profile.topics_of_interest = [...profile.topics_of_interest, topic];
    }
  }

  async function handleSubmit() {
    saving = true;
    error = null;
    success = false;

    // Build family_status
    let familyStatus: FamilyStatus;
    if (familyStatusType === 'Celibataire' || familyStatusType === 'EnCouple') {
      familyStatus = familyStatusType;
    } else if (familyStatusType === 'Parent') {
      familyStatus = { Parent: { children_count: childrenCount } };
    } else if (familyStatusType === 'ParentIsolé') {
      familyStatus = { ParentIsolé: { children_count: childrenCount } };
    } else {
      familyStatus = { CoupleAvecEnfants: { children_count: childrenCount } };
    }

    const updatedProfile: UserProfile = {
      ...profile,
      family_status: familyStatus,
      updated_at: Date.now() / 1000,
    };

    try {
      await backend.updateProfile(updatedProfile);
      success = true;
      setTimeout(() => { success = false; }, 3000);
    } catch (err) {
      error = `Erreur lors de la sauvegarde : ${err}`;
      console.error(err);
    } finally {
      saving = false;
    }
  }
</script>

<section class="profile-page">
  <div class="page-header">
    <h2>👤 Profil utilisateur</h2>
    <p class="page-subtitle">Configurez vos informations personnelles pour une expérience personnalisée</p>
  </div>

  {#if loading}
    <p class="loading">Chargement du profil...</p>
  {:else}
    <form class="profile-form" on:submit|preventDefault={handleSubmit}>
      <div class="form-group">
        <label for="first_name">Prénom</label>
        <input type="text" id="first_name" bind:value={profile.first_name} placeholder="Marie" />
      </div>

      <div class="form-group">
        <label for="age">Âge</label>
        <input type="number" id="age" bind:value={profile.age} min="0" max="120" placeholder="32" />
      </div>

      <div class="form-group">
        <label for="professional_status">Statut professionnel</label>
        <select id="professional_status" bind:value={profile.professional_status}>
          <option value={null}>Non renseigné</option>
          {#each professionalStatuses as status}
            <option value={status}>{status}</option>
          {/each}
        </select>
      </div>

      <div class="form-group">
        <label for="family_status_type">Situation familiale</label>
        <select id="family_status_type" bind:value={familyStatusType}>
          <option value="Celibataire">Célibataire</option>
          <option value="EnCouple">En couple</option>
          <option value="Parent">Parent</option>
          <option value="ParentIsolé">Parent isolé</option>
          <option value="CoupleAvecEnfants">Couple avec enfants</option>
        </select>
      </div>

      {#if familyStatusType === 'Parent' || familyStatusType === 'ParentIsolé' || familyStatusType === 'CoupleAvecEnfants'}
        <div class="form-group">
          <label for="children_count">Nombre d'enfants</label>
          <input type="number" id="children_count" bind:value={childrenCount} min="1" max="20" />
        </div>
      {/if}

      <div class="form-group">
        <label for="department">Département</label>
        <input type="text" id="department" bind:value={profile.department} placeholder="75" maxlength="3" />
      </div>

      <fieldset class="form-group">
        <legend>Sujets d'intérêt</legend>
        <div class="topics-grid">
          {#each topicsOptions as topic}
            <button
              type="button"
              class="topic-btn"
              class:selected={profile.topics_of_interest.includes(topic)}
              on:click={() => toggleTopic(topic)}
            >
              {topic}
            </button>
          {/each}
        </div>
      </fieldset>

      <div class="form-group">
        <label for="language_level">Niveau de langue</label>
        <select id="language_level" bind:value={profile.language_level}>
          <option value="Simple">Simple</option>
          <option value="Standard">Standard</option>
          <option value="Technique">Technique</option>
        </select>
      </div>

      <div class="form-group">
        <label for="device_preference">Préférence device</label>
        <select id="device_preference" bind:value={profile.device_preference}>
          <option value="Gpu">GPU</option>
          <option value="Cpu">CPU</option>
        </select>
      </div>

      <div class="form-actions">
        {#if error}
          <p class="error-msg">{error}</p>
        {/if}
        {#if success}
          <p class="success-msg">✅ Profil enregistré avec succès</p>
        {/if}
        <button type="submit" class="submit-btn" disabled={saving}>
          {saving ? 'Enregistrement...' : 'Enregistrer'}
        </button>
      </div>
    </form>
  {/if}
</section>

<style>
  .profile-page {
    padding: var(--spacing-lg);
    max-width: 900px;
    margin: 0 auto;
    height: 100%;
    overflow-y: auto;
    background: var(--bg-primary);
  }

  .page-header {
    margin-bottom: var(--spacing-xl);
  }

  .page-header h2 {
    font-size: 1.75rem;
    color: var(--text-primary);
    margin: 0 0 var(--spacing-sm) 0;
  }

  .page-subtitle {
    color: var(--text-secondary);
    font-size: 0.95rem;
    margin: 0;
  }

  .loading {
    text-align: center;
    padding: var(--spacing-xl);
    color: var(--text-secondary);
  }

  .profile-form {
    background: var(--bg-secondary);
    padding: var(--spacing-lg);
    border-radius: var(--radius-md);
    box-shadow: var(--shadow-sm);
  }

  .form-group {
    margin-bottom: var(--spacing-lg);
  }

  fieldset.form-group {
    border: none;
    padding: 0;
    margin: 0 0 var(--spacing-lg) 0;
  }

  fieldset.form-group legend {
    display: block;
    font-weight: 600;
    margin-bottom: var(--spacing-sm);
    color: var(--text-primary);
    padding: 0;
  }

  .form-group label {
    display: block;
    font-weight: 600;
    margin-bottom: var(--spacing-sm);
    color: var(--text-primary);
  }

  .form-group input,
  .form-group select {
    width: 100%;
    padding: var(--spacing-sm) var(--spacing-md);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    font-size: 0.95rem;
    font-family: inherit;
  }

  .form-group input:focus,
  .form-group select:focus {
    outline: none;
    border-color: var(--accent);
  }

  .topics-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(120px, 1fr));
    gap: var(--spacing-sm);
  }

  .topic-btn {
    padding: var(--spacing-sm) var(--spacing-md);
    border: 1px solid var(--border);
    background: var(--bg-primary);
    border-radius: var(--radius-sm);
    cursor: pointer;
    font-size: 0.9rem;
    transition: all 0.2s;
  }

  .topic-btn.selected {
    background: var(--bleu-france);
    color: var(--blanc);
    border-color: var(--bleu-france);
  }

  .topic-btn:hover {
    border-color: var(--bleu-france);
  }

  .form-actions {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-md);
    margin-top: var(--spacing-xl);
  }

  .submit-btn {
    padding: var(--spacing-md) var(--spacing-lg);
    background: var(--bleu-france);
    color: var(--blanc);
    border: none;
    border-radius: var(--radius-sm);
    font-size: 1rem;
    font-weight: 600;
    cursor: pointer;
    transition: background 0.2s;
  }

  .submit-btn:hover {
    background: var(--bleu-france-light);
  }

  .submit-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .error-msg {
    color: var(--error);
    text-align: center;
  }

  .success-msg {
    color: var(--success);
    text-align: center;
    font-weight: 600;
  }
</style>
