export function formatSourceLabel(url: string): string {
  try {
    const parsed = new URL(url);
    const domain = parsed.hostname.replace(/^www\./, '');
    
    // French government domains
    if (domain.endsWith('.gouv.fr')) {
      if (domain.includes('service-public')) return 'Service-Public.fr';
      if (domain.includes('caf.fr')) return 'CAF.fr';
      if (domain.includes('urssaf.fr')) return 'URSSAF.fr';
      if (domain.includes('impots.gouv.fr')) return 'Impôts.gouv.fr';
      if (domain.includes('ameli.fr')) return 'Ameli.fr';
      return domain;
    }

    // Specific sites
    if (domain.includes('legifrance')) return 'Légifrance';
    if (domain.includes('travail-emploi')) return 'Travail-Emploi.gouv.fr';
    if (domain.includes('economie.gouv.fr')) return 'Économie.gouv.fr';
    
    return domain;
  } catch {
    return url;
  }
}

export function openUrl(url: string): void {
  // In Electron, we can use shell.openExternal
  // But for security, we'll open in the default browser
  window.open(url, '_blank', 'noopener,noreferrer');
}
