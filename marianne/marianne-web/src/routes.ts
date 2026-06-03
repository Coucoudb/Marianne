import ChatPage from './pages/ChatPage.svelte';
import HistoryPage from './pages/HistoryPage.svelte';
import DocumentsPage from './pages/DocumentsPage.svelte';
import SettingsPage from './pages/SettingsPage.svelte';

export const routes = {
  '/': ChatPage,
  '/history': HistoryPage,
  '/documents': DocumentsPage,
  '/settings': SettingsPage,
  '/settings/*': SettingsPage,
};
