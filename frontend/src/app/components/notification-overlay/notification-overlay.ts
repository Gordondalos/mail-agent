import { Component, OnDestroy, OnInit, computed, signal, ElementRef } from '@angular/core';
import { CommonModule } from '@angular/common';
import { MatIconModule } from '@angular/material/icon';
import { TauriDragWindowDirective } from '../tauri-drag-window.directive';
import { Ipc } from '../../services/ipc';
import { UnlistenFn } from '@tauri-apps/api/event';
import { convertFileSrc } from '@tauri-apps/api/core';
import { getCurrentWindow } from '@tauri-apps/api/window';

type NotificationPayload = {
  id: string;
  thread_id: string;
  subject: string;
  snippet?: string | null;
  sender?: string | null;
  receivedAt?: string | null;
  url: string;
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

  constructor(
    private readonly ipc: Ipc,
    private elementRef: ElementRef
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
    const shell = this.elementRef.nativeElement.querySelector('.alert-shell');
    if (shell) {
      shell.addEventListener('mouseenter', () => {
        shell.style.opacity = '1';
      });
      shell.addEventListener('mouseleave', () => {
        this.applyOpacity();
      });
    }
  }

  ngOnDestroy() {
    this.unlistenFns.forEach(u => u());
  }

  private applyOpacity() {
    const opacity = this.settings?.notification_opacity ?? 0.95;
    const shell = this.elementRef.nativeElement.querySelector('.alert-shell');
    if (shell) {
      shell.style.opacity = opacity.toString();
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
}
