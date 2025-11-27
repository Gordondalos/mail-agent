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
import { MatSliderModule } from '@angular/material/slider';

@Component({
  selector: 'app-settings-page',
  imports: [CommonModule, FormsModule, MatFormFieldModule, MatInputModule, MatCheckboxModule, MatButtonModule, MatSelectModule, MatSliderModule],
  templateUrl: './settings-page.component.html',
  styleUrls: ['./settings-page.component.css']
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
    oauth_client_secret: '',
    snooze_duration_mins: 20,
    notification_width: 650,
    notification_height: 150,
    notification_opacity: 0.95
  };

  constructor(private readonly settingsSvc: Settings, private readonly ipc: Ipc) { }

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
        playback_volume: Number(String(this.model.playback_volume).replace(',', '.')),
        snooze_duration_mins: Number(this.model.snooze_duration_mins),
        notification_width: Number(this.model.notification_width),
        notification_height: Number(this.model.notification_height),
        notification_opacity: Number(String(this.model.notification_opacity).replace(',', '.'))
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
