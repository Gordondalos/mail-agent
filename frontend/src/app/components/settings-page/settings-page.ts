import { Component, OnInit, signal } from '@angular/core';
import { CommonModule } from '@angular/common';
import { FormsModule } from '@angular/forms';
import { Settings, VoicePreset } from '../../services/settings';
import { Ipc } from '../../services/ipc';
import { MatFormFieldModule } from '@angular/material/form-field';
import { MatInputModule } from '@angular/material/input';
import { MatCheckboxModule } from '@angular/material/checkbox';
import { MatButtonModule } from '@angular/material/button';
import { MatSelectModule, MatSelectChange } from '@angular/material/select';

@Component({
  selector: 'app-settings-page',
  imports: [CommonModule, FormsModule, MatFormFieldModule, MatInputModule, MatCheckboxModule, MatButtonModule, MatSelectModule],
  template: `
    <div class="container">
      <h1>Gmail Tray Notifier</h1>

      <div class="status" [class.unauthorised]="!authorised()">
        {{ authorised() ? 'Авторизация выполнена' : 'Требуется вход' }}
      </div>

      <div class="actions">
        <button mat-flat-button color="primary" (click)="connect()" [disabled]="busy()">
          {{ authorised() ? 'Повторная авторизация' : 'Войти в Gmail' }}
        </button>
        <button mat-stroked-button (click)="logout()" [disabled]="busy()">Выйти</button>
        <button mat-stroked-button (click)="checkNow()" [disabled]="busy()">Проверить сейчас</button>
      </div>

      <form class="settings" (ngSubmit)="save()">
        <mat-form-field appearance="outline">
          <mat-label>Интервал проверки (сек.)</mat-label>
          <input matInput type="number" min="15" max="300" [(ngModel)]="model.poll_interval_secs" name="interval" />
        </mat-form-field>

        <div class="row sound-row">
          <mat-checkbox [(ngModel)]="model.sound_enabled" name="soundEnabled">Звук уведомления</mat-checkbox>
          <div class="sound-inputs">
            <mat-form-field appearance="outline" class="flex1">
              <mat-label>Путь к файлу (mp3/wav)</mat-label>
              <input
                matInput
                type="text"
                [(ngModel)]="model.sound_path"
                name="soundPath"
                (ngModelChange)="onSoundPathInput($event)"
              />
            </mat-form-field>
            <mat-form-field
              appearance="outline"
              class="voice-select"
              *ngIf="voicePresets().length > 0"
            >
              <mat-label>Встроенная мелодия</mat-label>
              <mat-select
                [value]="selectedVoicePreset()"
                (selectionChange)="onVoicePresetSelected($event)"
              >
                <mat-option [value]="null">Не выбрано</mat-option>
                <mat-option
                  *ngFor="let preset of voicePresets()"
                  [value]="preset.id"
                >
                  {{ preset.label }}
                </mat-option>
              </mat-select>
            </mat-form-field>
          </div>
        </div>

        <mat-form-field appearance="outline">
          <mat-label>Громкость (0-1)</mat-label>
          <input matInput type="number" step="0.05" min="0" max="1" [(ngModel)]="model.playback_volume" name="volume" />
        </mat-form-field>

        <div class="row">
          <mat-checkbox [(ngModel)]="model.auto_launch" name="autoLaunch">Запускать при входе в систему</mat-checkbox>
        </div>

        <mat-form-field appearance="outline">
          <mat-label>Поисковый запрос Gmail</mat-label>
          <textarea matInput [(ngModel)]="model.gmail_query" name="gmailQuery"></textarea>
        </mat-form-field>

        <mat-form-field appearance="outline">
          <mat-label>OAuth Client ID</mat-label>
          <input matInput type="text" [(ngModel)]="model.oauth_client_id" name="clientId" />
        </mat-form-field>

        <mat-form-field appearance="outline">
          <mat-label>OAuth Client Secret (опционально)</mat-label>
          <input matInput type="text" [(ngModel)]="model.oauth_client_secret" name="clientSecret" />
        </mat-form-field>

        <div class="actions">
          <button mat-flat-button color="primary" type="submit" [disabled]="busy()">Сохранить настройки</button>
        </div>
      </form>

      <div class="notice">
        OAuth2-клиент (Desktop app) создаётся в Google Cloud Console. <br/>
        Укажите Client ID и секрет ниже. Токены автоматически сохраняются в системном хранилище.
      </div>
    </div>
  `,
  styles: `
    .container { padding: 24px; font-family: system-ui, -apple-system, Segoe UI, Roboto, Arial; }
    h1 { margin: 0 0 12px; font-size: 20px; }
    .status { margin: 8px 0 16px; padding: 8px 10px; border-radius: 8px; background: rgba(16,185,129,.15); }
    .status.unauthorised { background: rgba(239,68,68,.15); }
    .actions { display:flex; gap:10px; margin: 8px 0 16px; }
    form.settings { display:grid; gap:12px; }
    .field { display:flex; flex-direction:column; gap:6px; }
    .row { display:flex; align-items:center; gap:16px; }
    .row.sound-row { align-items:flex-start; }
    .sound-inputs { display:flex; gap:12px; flex:1 1 auto; }
    .flex1 { flex: 1 1 auto; }
    .voice-select { min-width: 220px; }
    textarea { min-height: 70px; }
    .notice { margin-top: 14px; padding: 10px 12px; border-radius: 10px; background: rgba(37,99,235,.12); }
  `,
})
export class SettingsPage implements OnInit {
  authorised = signal<boolean>(false);
  busy = signal<boolean>(false);
  voicePresets = signal<VoicePreset[]>([]);
  selectedVoicePreset = signal<string | null>(null);
  model: any = {
    poll_interval_secs: 60,
    sound_enabled: true,
    sound_path: '',
    playback_volume: 0.7,
    auto_launch: true,
    gmail_query: 'in:inbox is:unread',
    oauth_client_id: '',
    oauth_client_secret: ''
  };

  constructor(private readonly settingsSvc: Settings, private readonly ipc: Ipc) {}

  async ngOnInit() {
    const state = await this.settingsSvc.initialise();
    this.authorised.set(state.authorised);
    this.model = { ...this.model, ...state.settings };
    await this.loadVoicePresets();
    this.syncVoicePresetSelection();
  }

  async connect() {
    this.busy.set(true);
    try {
      await this.ipc.invoke('request_authorisation');
      const state = await this.settingsSvc.initialise();
      this.authorised.set(state.authorised);
      this.model = { ...this.model, ...state.settings };
    } catch (e) {
      alert('Ошибка авторизации: ' + e);
    } finally {
      this.busy.set(false);
    }
  }

  async logout() {
    this.busy.set(true);
    try {
      await this.ipc.invoke('revoke');
      this.authorised.set(false);
    } catch (e) {
      alert('Не удалось выйти: ' + e);
    } finally {
      this.busy.set(false);
    }
  }

  async checkNow() {
    this.busy.set(true);
    try {
      await this.ipc.invoke('check_now');
    } catch (e) {
      alert('Не удалось выполнить проверку: ' + e);
    } finally {
      this.busy.set(false);
    }
  }

  async save() {
    this.busy.set(true);
    try {
      const update = {
        poll_interval_secs: Number(this.model.poll_interval_secs),
        sound_enabled: !!this.model.sound_enabled,
        sound_path: this.model.sound_path || null,
        auto_launch: !!this.model.auto_launch,
        gmail_query: this.model.gmail_query,
        oauth_client_id: this.model.oauth_client_id,
        oauth_client_secret: this.model.oauth_client_secret || null,
        playback_volume: Number(this.model.playback_volume)
      };
      const saved = await this.settingsSvc.update(update);
      this.model = { ...this.model, ...saved };
      this.syncVoicePresetSelection();
    } catch (e) {
      alert('Не удалось сохранить: ' + e);
    } finally {
      this.busy.set(false);
    }
  }

  async loadVoicePresets() {
    try {
      const presets = await this.settingsSvc.voicePresets();
      this.voicePresets.set(presets);
    } catch (e) {
      console.warn('Не удалось загрузить встроенные мелодии', e);
      this.voicePresets.set([]);
    }
    this.syncVoicePresetSelection();
  }

  onSoundPathInput(_: string) {
    this.syncVoicePresetSelection();
  }

  onVoicePresetSelected(event: MatSelectChange) {
    const selectedId: string | null = event.value ?? null;
    if (!selectedId) {
      this.selectedVoicePreset.set(null);
      this.model.sound_path = '';
      return;
    }
    const preset = this.voicePresets().find((p) => p.id === selectedId);
    if (!preset) {
      return;
    }
    this.model.sound_path = preset.path;
    this.selectedVoicePreset.set(preset.id);
  }

  private syncVoicePresetSelection() {
    const current = this.model.sound_path || '';
    const preset = this.voicePresets().find((p) => p.path === current);
    this.selectedVoicePreset.set(preset ? preset.id : null);
  }
}
