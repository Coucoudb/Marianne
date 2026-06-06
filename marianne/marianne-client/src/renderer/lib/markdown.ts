import { marked } from 'marked';

function sanitizeHtml(html: string): string {
  const div = document.createElement('div');
  div.innerHTML = html;
  div.querySelectorAll('script, iframe, object, embed, form').forEach(el => el.remove());
  div.querySelectorAll('*').forEach(el => {
    for (const attr of [...el.attributes]) {
      if (
        attr.name.startsWith('on') ||
        (attr.name === 'href' && attr.value.trim().toLowerCase().startsWith('javascript:')) ||
        (attr.name === 'src' && attr.value.trim().toLowerCase().startsWith('javascript:'))
      ) {
        el.removeAttribute(attr.name);
      }
    }
  });
  return div.innerHTML;
}

export function parseMarkdown(text: string): string {
  if (!text) return '';
  const raw = marked.parse(text) as string;
  return sanitizeHtml(raw);
}
