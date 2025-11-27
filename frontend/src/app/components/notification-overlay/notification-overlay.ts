import { Component, OnDestroy, OnInit, computed, signal } from '@angular/core';
import { CommonModule } from '@angular/common';
import { MatIconModule } from '@angular/material/icon';
import { DomSanitizer, SafeHtml } from '@angular/platform-browser';
import { TauriDragWindowDirective } from '../tauri-drag-window.directive';
import { Ipc } from '../../services/ipc';
import { UnlistenFn } from '@tauri-apps/api/event';
import { convertFileSrc } from '@tauri-apps/api/core';
import { getCurrentWindow, LogicalSize, LogicalPosition, PhysicalPosition } from '@tauri-apps/api/window';
import { WebviewWindow } from '@tauri-apps/api/webviewWindow';

type NotificationPayload = {
  id: string;
  thread_id: string;
  subject: string;
  snippet?: string | null;
  sender?: string | null;
  recipient?: string | null;
  receivedAt?: string | null;
  url: string;
  body?: string | null;
};

@Component({
  selector: 'app-notification-overlay',
  imports: [CommonModule, MatIconModule, TauriDragWindowDirective, TauriDragWindowDirective],
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
  safeBody = computed<SafeHtml | null>(() => {
    const n = this.current();
    const expanded = this.isExpanded(); // Явная зависимость от isExpanded
    console.log('safeBody computed: current=', n ? 'exists' : 'null', 'isExpanded=', expanded, 'hasBody=', !!n?.body);
    if (!n?.body) {
      console.log('safeBody: no body found');
      return null;
    }
    console.log('safeBody: body length=', n.body.length, 'first 100 chars=', n.body.substring(0, 100));
    return this.sanitizer.bypassSecurityTrustHtml(n.body);
  });

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
        this.applyOpacity();
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
    console.log('notification', n);
    this.notification.set(null);
    this.visible.set(false);
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
    console.log('notification', n);
    this.notification.set(null);
    this.visible.set(false);
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
      await getCurrentWindow().hide();
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

  getSafeBody(body: string | null | undefined): SafeHtml {
    console.log('getSafeBody called with body length=', body?.length || 0);
    if (!body) {
      return '';
    }
    return this.sanitizer.bypassSecurityTrustHtml(body);
  }

  async toggleExpand() {
    const tauriWindow: WebviewWindow | any = getCurrentWindow();
    const isCurrentlyExpanded = this.isExpanded();
    console.log('toggleExpand: current state=', isCurrentlyExpanded, 'notification=', this.notification());

    if (!isCurrentlyExpanded) {
      console.log('toggleExpand: expanding window');
      // Разворачиваем окно до размеров из настроек или на весь экран
      let expandedWidth = this.settings?.notification_expanded_width;
      let expandedHeight = this.settings?.notification_expanded_height;

      // Если размеры не заданы в настройках, используем размер экрана
      if (!expandedWidth || !expandedHeight || expandedWidth === 0 || expandedHeight === 0) {
        try {
          const monitor = await tauriWindow.currentMonitor();
          if (monitor) {
            expandedWidth = monitor.size.width;
            expandedHeight = monitor.size.height;
            // Позиционируем окно на весь экран
            await tauriWindow.setPosition(new PhysicalPosition(monitor.position.x, monitor.position.y));
          } else {
            // Fallback размеры, если монитор не определен
            expandedWidth = 1200;
            expandedHeight = 800;
          }
        } catch (error) {
          console.error('Failed to get monitor info', error);
          expandedWidth = 1200;
          expandedHeight = 800;
        }
      }

      await tauriWindow.setSize(new LogicalSize(expandedWidth, expandedHeight));

      // Центрируем окно, если это не полноэкранный режим
      if (expandedWidth < 1900) {
        try {
          const monitor = await tauriWindow.currentMonitor();
          if (monitor) {
            const x = monitor.position.x + (monitor.size.width - expandedWidth) / 2;
            const y = monitor.position.y + (monitor.size.height - expandedHeight) / 2;
            await tauriWindow.setPosition(new LogicalPosition(Math.max(0, x), Math.max(0, y)));
          }
        } catch (error) {
          console.error('Failed to center window', error);
        }
      }

      this.isExpanded.set(true);
      console.log('toggleExpand: window expanded, isExpanded=', this.isExpanded());
    } else {
      console.log('toggleExpand: collapsing window');
      // Сворачиваем окно обратно
      const normalWidth = this.settings?.notification_width ?? 650;
      const normalHeight = this.settings?.notification_height ?? 150;
      await tauriWindow.setSize(new LogicalSize(normalWidth, normalHeight));

      // Возвращаем окно в правый нижний угол
      try {
        const monitor = await tauriWindow.currentMonitor();
        if (monitor) {
          const margin = 64;
          const x = monitor.position.x + monitor.size.width - normalWidth - margin;
          const y = monitor.position.y + monitor.size.height - normalHeight - margin;
          await tauriWindow.setPosition(new LogicalPosition(Math.max(monitor.position.x, x), Math.max(monitor.position.y, y)));
        }
      } catch (error) {
        console.error('Failed to reposition window', error);
      }

      this.isExpanded.set(false);
      console.log('toggleExpand: window collapsed, isExpanded=', this.isExpanded());
    }
  }
}
