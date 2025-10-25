import { Component, OnDestroy, OnInit, signal } from '@angular/core';
import { CommonModule } from '@angular/common';
import { Ipc } from '../../services/ipc';
import { UnlistenFn } from '@tauri-apps/api/event';
import { convertFileSrc } from '@tauri-apps/api/core';

type NotificationPayload = {
  id: string;
  thread_id: string;
  subject: string;
  snippet?: string | null;
  sender?: string | null;
  received_at?: string | null;
  url: string;
};

@Component({
  selector: 'app-notification-overlay',
  imports: [CommonModule],
  template: `
    <div class="alert-shell" [class.hidden]="!visible()">
      <div class="alert-card" data-tauri-drag-region>
        <button
          class="alert-close"
          type="button"
          aria-label="Закрыть уведомление"
          (click)="dismiss()"
          data-tauri-drag-region="false"
        >
          ×
        </button>
        <div class="alert-content" data-tauri-drag-region="false">
          <div class="alert-title">{{ notification()?.subject || '(без темы)' }}</div>
          <div class="alert-meta">{{ notification()?.sender || 'Gmail' }}</div>
          <div class="alert-snippet">{{ notification()?.snippet || '' }}</div>
        </div>
        <div class="alert-actions" data-tauri-drag-region="false">
          <button class="open" (click)="open()">Перейти</button>
          <button class="read" (click)="markRead()">Прочитано</button>
          <button class="dismiss" (click)="dismiss()">Скрыть</button>
        </div>
      </div>
    </div>
  `,
  styles: `
    * {
      box-sizing: border-box;
    }
    :host {
      color: #0f172a;
      background-color: white;
      display: block;
    }
    ::ng-deep {
      body {
        margin: 0;
        padding: 0;

        font-family: system-ui, -apple-system, 'Segoe UI', sans-serif; }
    }


      .alert-shell {
        width: 100%; height: 100%;
        display:flex; align-items:stretch; justify-content:center; padding:16px; box-sizing:border-box; }
    .hidden { display: none; }
    .alert-card { position:relative; width:100%; border-radius:16px; background: linear-gradient(135deg, rgba(15,23,42,0.95), rgba(37,99,235,0.92)); color:#f8fafc; box-shadow: 0 24px 64px rgba(15,23,42,0.45); padding:18px 22px; display:grid; grid-template-columns:1fr auto; gap:12px; border:1px solid rgba(148,163,184,0.4); }
    .alert-content { display:flex; flex-direction:column; gap:6px; overflow:hidden; padding-right:24px; }
    .alert-title { font-size:1.05rem; font-weight:600; white-space:nowrap; overflow:hidden; text-overflow:ellipsis; }
    .alert-meta { font-size:.85rem; opacity:.85; }
    .alert-snippet { font-size:.9rem; opacity:.9; max-height:48px; overflow:hidden; }
    .alert-actions { display:flex; flex-direction:column; gap:10px; align-items:stretch; }
    .alert-actions button { padding:10px 14px; border-radius:10px; border:none; font-weight:600; cursor:pointer; }
    .alert-actions button.open { background: rgba(59,130,246,.95); color:#fff; }
    .alert-actions button.read { background: rgba(34,197,94,.9); color:#fff; }
    .alert-actions button.dismiss { background: rgba(15,23,42,.5); color:#f8fafc; }
    .alert-close { position:absolute; top:8px; right:12px; background:transparent; border:none; color:#e2e8f0; font-size:18px; cursor:pointer; padding:4px; line-height:1; }
    .alert-close:hover { color:#fff; }
  `,
})
export class NotificationOverlay implements OnInit, OnDestroy {
  visible = signal<boolean>(true);
  notification = signal<NotificationPayload | null>(null);
  settings: any | null = null;
  unlistenFns: UnlistenFn[] = [];

  constructor(private readonly ipc: Ipc) {}

  async ngOnInit() {
    const state = await this.ipc.invoke<{ settings: any; authorised: boolean }>('initialise');
    this.settings = state.settings;

    this.unlistenFns.push(await this.ipc.on('gmail://notification', async (n: NotificationPayload) => {
      this.notification.set(n);
      this.visible.set(true);
      await this.playSound();
    }));
    this.unlistenFns.push(await this.ipc.on('gmail://settings', (s: any) => { this.settings = s; }));

    await this.restoreCurrent();
  }

  ngOnDestroy() { this.unlistenFns.forEach(u => u()); }

  async open() {
    const n = this.notification();
    if (!n) return;
    try {
      await this.ipc.invoke('open_in_browser', { url: n.url });
    } finally {
      this.visible.set(false);
    }
  }

  async markRead() {
    const n = this.notification();
    if (!n) return;
    try {
      await this.ipc.invoke('mark_message_read', { messageId: n.id });
    } finally {
      this.visible.set(false);
    }
  }

  async dismiss() {
    await this.ipc.invoke('dismiss_notification');
    this.visible.set(false);
  }

  private async restoreCurrent() {
    try {
      const current = await this.ipc.invoke<NotificationPayload | null>('current_notification');
      if (current) {
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
      const src = await convertFileSrc(this.settings.sound_path);
      const audio = new Audio(src);
      audio.volume = this.settings.playback_volume ?? 0.7;
      await audio.play();
    } catch (e) {
      // noop
    }
  }
}
