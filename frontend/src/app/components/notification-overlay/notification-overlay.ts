import { Component, OnDestroy, OnInit, signal } from '@angular/core';
import { CommonModule } from '@angular/common';
import { Ipc } from '../../services/ipc';
import { UnlistenFn } from '@tauri-apps/api/event';
import { convertFileSrc } from '@tauri-apps/api/core';

@Component({
  selector: 'app-notification-overlay',
  imports: [CommonModule],
  template: `
    <div class="alert-shell" [class.hidden]="!visible()">
      <div class="alert-card">
        <div class="alert-content">
          <div class="alert-title">{{ notification()?.subject || '(без темы)' }}</div>
          <div class="alert-meta">{{ notification()?.sender || 'Gmail' }}</div>
          <div class="alert-snippet">{{ notification()?.snippet || '' }}</div>
        </div>
        <div class="alert-actions">
          <button class="open" (click)="open()">Перейти</button>
          <button class="read" (click)="markRead()">Прочитано</button>
          <button class="dismiss" (click)="dismiss()">Скрыть</button>
        </div>
      </div>
    </div>
  `,
  styles: `
    .alert-shell { width: 800px; height: 150px; display:flex; align-items:stretch; justify-content:center; padding:16px; box-sizing:border-box; }
    .hidden { display: none; }
    .alert-card { width:100%; border-radius:16px; background: linear-gradient(135deg, rgba(15,23,42,0.92), rgba(30,64,175,0.92)); color:#f8fafc; box-shadow: 0 16px 48px rgba(15,23,42,0.4); padding:16px 20px; display:grid; grid-template-columns:1fr auto; gap:12px; backdrop-filter: blur(18px); border:1px solid rgba(148,163,184,0.25); }
    .alert-content { display:flex; flex-direction:column; gap:6px; overflow:hidden; }
    .alert-title { font-size:1.05rem; font-weight:600; white-space:nowrap; overflow:hidden; text-overflow:ellipsis; }
    .alert-meta { font-size:.85rem; opacity:.75; }
    .alert-snippet { font-size:.9rem; opacity:.85; max-height:48px; overflow:hidden; }
    .alert-actions { display:flex; flex-direction:column; gap:10px; align-items:stretch; }
    .alert-actions button { padding:10px 14px; border-radius:10px; border:none; font-weight:600; cursor:pointer; }
    .alert-actions button.open { background: rgba(59,130,246,.9); color:#fff; }
    .alert-actions button.read { background: rgba(34,197,94,.85); color:#fff; }
    .alert-actions button.dismiss { background: rgba(15,23,42,.4); color:#f8fafc; }
  `,
})
export class NotificationOverlay implements OnInit, OnDestroy {
  visible = signal<boolean>(false);
  notification = signal<any | null>(null);
  settings: any | null = null;
  unlistenFns: UnlistenFn[] = [];

  constructor(private readonly ipc: Ipc) {}

  async ngOnInit() {
    const state = await this.ipc.invoke<{ settings: any; authorised: boolean }>('initialise');
    this.settings = state.settings;
    this.unlistenFns.push(await this.ipc.on('gmail://notification', async (n: any) => {
      this.notification.set(n);
      this.visible.set(true);
      await this.playSound();
    }));
    this.unlistenFns.push(await this.ipc.on('gmail://settings', (s: any) => { this.settings = s; }));
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
