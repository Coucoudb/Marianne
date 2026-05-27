const SOURCE_LABELS: Record<string, string> = {
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

export function formatSourceLabel(url: string): string {
  try {
    const hostname = new URL(url).hostname
      .replace(/^www\./, '')
      .replace(/^www2\./, '');
    for (const [domain, label] of Object.entries(SOURCE_LABELS)) {
      if (hostname.includes(domain)) return label;
    }
    return hostname
      .replace(/\.gouv\.fr$/, '')
      .replace(/\.fr$/, '')
      .replace(/\.com$/, '');
  } catch {
    return url.length > 40 ? url.substring(0, 37) + '…' : url;
  }
}
