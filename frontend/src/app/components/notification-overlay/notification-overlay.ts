import { Component, OnDestroy, OnInit, computed, signal } from '@angular/core';
import { CommonModule } from '@angular/common';
import { MatIconModule } from '@angular/material/icon';
import { DomSanitizer, SafeHtml } from '@angular/platform-browser';
import { Ipc } from '../../services/ipc';
import { UnlistenFn } from '@tauri-apps/api/event';
import { convertFileSrc } from '@tauri-apps/api/core';
import { getCurrentWindow, LogicalSize, LogicalPosition } from '@tauri-apps/api/window';

type NotificationPayload = {
  id: string;
  thread_id?: string;
  threadId?: string;
  subject: string;
  snippet?: string | null;
  sender?: string | null;
  recipient?: string | null;
  receivedAt?: string | null;
  received_at?: string | null;
  url: string;
  body?: string | null;
};

@Component({
  selector: 'app-notification-overlay',
  imports: [CommonModule, MatIconModule],
  templateUrl: './notification-overlay.component.html',
  styleUrls: ['./notification-overlay.component.scss'],
})
export class NotificationOverlay implements OnInit, OnDestroy {
  visible = signal<boolean>(false);
  notification = signal<NotificationPayload | null>(null);
  current = computed(() => (this.visible() ? this.notification() : null));
  settings: any | null = null;
  unlistenFns: UnlistenFn[] = [];
  private dateFormatter: Intl.DateTimeFormat | null = null;
  isExpanded = signal<boolean>(false);
  sidebarMessages = signal<NotificationPayload[]>([]);
  sidebarLoading = signal<boolean>(false);
  sidebarError = signal<string | null>(null);
  sidebarBulkAction = signal<boolean>(false);
  selectedPreview = signal<NotificationPayload | null>(null);
  displayedMessage = computed<NotificationPayload | null>(() => this.selectedPreview() ?? this.notification());
  canUsePrimaryActions = computed<boolean>(() => {
    const selected = this.selectedPreview();
    const current = this.notification();
    if (!selected) {
      return true;
    }
    if (!current) {
      return false;
    }
    return selected.id === current.id;
  });
  safeBody = computed<SafeHtml | null>(() => {
    const n = this.displayedMessage();
    if (!this.isExpanded()) {
      return null;
    }
    const prepared = this.prepareBodyHtml(n?.body ?? n?.snippet ?? null);
    if (!prepared) {
      return null;
    }
    return this.sanitizer.bypassSecurityTrustHtml(prepared);
  });
  private readonly windowRef = getCurrentWindow();
  constructor(
    private readonly ipc: Ipc,
    private readonly sanitizer: DomSanitizer
  ) {
  }

  async ngOnInit() {
    const state = await this.ipc.invoke<{ settings: any; authorised: boolean }>('initialise');
    this.settings = state.settings;
    this.applyOpacity();

    this.unlistenFns.push(await this.ipc.on('gmail://notification', async (n: NotificationPayload) => {
      console.debug('[gmail notification]', JSON.stringify(n, null, 2));
      console.debug('[gmail notification body length]', n?.body?.length ?? 0);
      console.debug('[gmail notification snippet length]', n?.snippet?.length ?? 0);
       this.selectedPreview.set(null);
       this.isExpanded.set(false);
       this.notification.set(n);
       this.visible.set(true);
       await this.playSound();
    }));
    this.unlistenFns.push(await this.ipc.on('gmail://settings', (s: any) => {
      this.settings = s;
      this.applyOpacity();
    }));

    await this.restoreCurrent();

    // Добавляем обработчики для изменения прозрачности при наведении
    const appRoot = document.querySelector('app-root') as HTMLElement;
    if (appRoot) {
      appRoot.addEventListener('mouseenter', () => {
        appRoot.style.backgroundColor = 'rgba(255, 255, 255, 1)';
      });
      appRoot.addEventListener('mouseleave', () => {
        // В expanded-режиме не делаем прозрачным
        if (!this.isExpanded()) {
          this.applyOpacity();
        }
      });
    }
  }

  ngOnDestroy() {
    this.unlistenFns.forEach(u => u());
  }

  private applyOpacity() {
    const opacity = this.settings?.notification_opacity ?? 0.95;
    // Применяем прозрачность к app-root
    const appRoot = document.querySelector('app-root') as HTMLElement;
    if (appRoot) {
      appRoot.style.backgroundColor = `rgba(255, 255, 255, ${opacity})`;
    }
  }

  async open() {
    const n = this.notification();
    if (!n) return;
    this.notification.set(null);
    this.visible.set(false);
    this.isExpanded.set(false);
    this.selectedPreview.set(null);
    await this.hideWindow();
    try {
      await this.ipc.invoke('open_in_browser', { url: n.url });
    } catch (error) {
      console.error('failed to open in browser', error);
    }
  }

  async markRead() {
    const n = this.notification();
    if (!n) return;
    this.notification.set(null);
    this.visible.set(false);
    this.isExpanded.set(false);
    this.selectedPreview.set(null);
    await this.hideWindow();
    try {
      await this.ipc.invoke('mark_message_read', { messageId: n.id });
    } catch (error) {
      console.error('failed to mark read', error);
    }
  }

  async dismiss() {
    const n = this.notification();
    this.notification.set(null);
    this.visible.set(false);
    this.isExpanded.set(false);
    this.selectedPreview.set(null);
    await this.hideWindow();
    if (n?.id) {
      await this.ipc.invoke('dismiss_notification', { messageId: n.id });
    } else {
      await this.ipc.invoke('dismiss_notification');
    }
  }

  async snooze() {
    this.notification.set(null);
    this.visible.set(false);
    this.isExpanded.set(false);
    this.selectedPreview.set(null);
    await this.hideWindow();
    try {
      await this.ipc.invoke('snooze');
    } catch (error) {
      console.error('failed to snooze', error);
    }
  }

  private async restoreCurrent() {
    try {
      const current = await this.ipc.invoke<NotificationPayload | null>('current_notification');
      if (current) {
        console.debug('[gmail notification:restore]', JSON.stringify(current, null, 2));
        this.notification.set(current);
        this.visible.set(true);
      }
    } catch {
      // ignore
    }
  }

  private async playSound() {
    if (!this.settings?.sound_enabled || !this.settings?.sound_path) return;
    try {
      const src = await this.resolveSoundSource(this.settings.sound_path);
      if (!src) {
        return;
      }
      const audio = new Audio(src);
      audio.volume = this.settings.playback_volume ?? 0.7;
      await audio.play();
    } catch (e) {
      // noop
    }
  }

  private async resolveSoundSource(path: string): Promise<string | null> {
    if (!path) return null;
    if (path.startsWith('http://') || path.startsWith('https://') || path.startsWith('data:')) {
      return path;
    }
    if (this.isAbsolutePath(path)) {
      return convertFileSrc(path);
    }
    return this.normalizeRelativePath(path);
  }

  private isAbsolutePath(path: string): boolean {
    return /^[a-zA-Z]:\\/.test(path) || path.startsWith('\\\\') || path.startsWith('/') || path.startsWith('file:');
  }

  private normalizeRelativePath(path: string): string {
    const sanitized = path.replace(/^[/\\]+/, '').replace(/\\/g, '/');
    return `/${sanitized}`;
  }

  private async hideWindow() {
    try {
      await this.windowRef.hide();
    } catch {
      // ignore
    }
  }

  formatDate(value: string | null | undefined | any): string {
    if (!value) {
      return '';
    }
    if (!this.dateFormatter) {
      this.dateFormatter = new Intl.DateTimeFormat(undefined, {
        dateStyle: 'medium',
        timeStyle: 'short',
      });
    }
    try {
      return this.dateFormatter.format(new Date(value));
    } catch {
      return value;
    }
  }

  decodeHtmlEntities(text: string | null | undefined): string {
    if (!text) return '';
    const textarea = document.createElement('textarea');
    textarea.innerHTML = text;
    return textarea.value;
  }

  async toggleExpand(event?: Event) {
    event?.preventDefault();
    event?.stopPropagation();

    const nextExpanded = !this.isExpanded();
    try {
      await this.ipc.invoke('toggle_alert_window', { expanded: nextExpanded });
      this.isExpanded.set(nextExpanded);
      // В expanded — белый фон без прозрачности, в collapsed — вернуть opacity
      const appRoot = document.querySelector('app-root') as HTMLElement;
      if (appRoot) {
        appRoot.style.backgroundColor = nextExpanded
          ? 'rgba(255, 255, 255, 1)'
          : `rgba(255, 255, 255, ${this.settings?.notification_opacity ?? 0.95})`;
      }
      if (nextExpanded) {
        void this.loadUnreadList(true);
      } else {
        this.selectedPreview.set(null);
      }
    } catch (error) {
      console.error('[overlay.toggleExpand] toggle_alert_window failed', error);
    }
  }

  selectSidebarMessage(message: NotificationPayload) {
    this.selectedPreview.set(message);
  }

  async markAllSidebarMessagesRead() {
    const ids = this.sidebarMessages().map((item) => item.id);
    if (!ids.length || this.sidebarBulkAction()) {
      return;
    }
    this.sidebarBulkAction.set(true);
    this.sidebarError.set(null);
    try {
      await this.ipc.invoke('mark_messages_read', { message_ids: ids });
      this.selectedPreview.set(null);
      await this.loadUnreadList(true);
    } catch (error) {
      console.error('failed to mark messages read', error);
      this.sidebarError.set('Не удалось пометить письма прочитанными');
    } finally {
      this.sidebarBulkAction.set(false);
    }
  }

  refreshUnreadList() {
    void this.loadUnreadList(true);
  }

  private async loadUnreadList(force = false): Promise<void> {
    console.debug('[sidebar.load] request', {
      force,
      expanded: this.isExpanded(),
      loading: this.sidebarLoading(),
      existing: this.sidebarMessages().length,
    });
     if (this.sidebarLoading()) {
      console.debug('[sidebar.load] skip: already loading');
      return;
    }
    if (!force && this.sidebarMessages().length) {
      console.debug('[sidebar.load] skip: cached list used');
      return;
    }
    this.sidebarLoading.set(true);
    this.sidebarError.set(null);
    try {
      const items = await this.ipc.invoke<NotificationPayload[]>('list_unread', { limit: 10 });
      console.debug('[sidebar.load] response', {
        count: items.length,
        items: items.map((item) => ({
          id: item.id,
          subject: item.subject,
          bodyLength: item.body?.length ?? 0,
          snippetLength: item.snippet?.length ?? 0,
        })),
      });
       this.sidebarMessages.set(items);
      if (this.isExpanded()) {
        const currentSelection = this.selectedPreview();
        if (!currentSelection || !items.some((item) => item.id === currentSelection.id)) {
          const currentNotification = this.notification();
          const replacement =
            items.find((item) => (currentNotification ? item.id === currentNotification.id : false)) ??
            items[0] ??
            null;
          this.selectedPreview.set(replacement);
        }
      }
    } catch (error) {
      console.error('failed to load unread list', error);
      this.sidebarError.set('Не удалось загрузить письма');
    } finally {
      this.sidebarLoading.set(false);
      console.debug('[sidebar.load] done', {
        loading: this.sidebarLoading(),
        count: this.sidebarMessages().length,
        error: this.sidebarError(),
      });
    }
  }

  private prepareBodyHtml(body: string | null): string | null {
    if (!body) {
      return null;
    }
    const normalized = this.normalizeEmailHtml(body);
    // Для plain text не используем сырой innerHTML: сохраняем переносы строк.
    if (!this.looksLikeHtml(normalized)) {
      return `<pre class="mail-plain">${this.escapeHtml(normalized)}</pre>`;
    }
    return normalized;
  }

  private normalizeEmailHtml(value: string): string {
    if (!/^\s*<(?:!doctype\s+html|html|body)\b/i.test(value)) {
      return value;
    }

    try {
      const doc = new DOMParser().parseFromString(value, 'text/html');
      return doc.body?.innerHTML?.trim() || value;
    } catch {
      return value;
    }
  }

  private looksLikeHtml(value: string): boolean {
    return /<\/?[a-z][\s\S]*>/i.test(value);
  }

  private escapeHtml(value: string): string {
    return value
      .replace(/&/g, '&amp;')
      .replace(/</g, '&lt;')
      .replace(/>/g, '&gt;')
      .replace(/"/g, '&quot;')
      .replace(/'/g, '&#39;');
  }
}
