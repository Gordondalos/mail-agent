import { Routes } from '@angular/router';
import { SettingsPage } from './components/settings-page/settings-page';
import { NotificationOverlay } from './components/notification-overlay/notification-overlay';

export const routes: Routes = [
  { path: '', component: SettingsPage },
  { path: 'alert', component: NotificationOverlay },
];
