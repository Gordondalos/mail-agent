const { tauri, event, window: tauriWindow } = window.__TAURI__;
const { invoke, convertFileSrc } = tauri;
const { listen } = event;
const { appWindow } = tauriWindow;

const params = new URLSearchParams(window.location.search);
const view = params.get('view');
let cachedSettings = null;

async function init() {
  if (view === 'alert') {
    document.body.classList.add('alert');
    await initAlert();
  } else {
    await initMain();
  }
}

async function initMain() {
  const root = document.querySelector('#app');
  const state = await invoke('initialise');
  cachedSettings = state.settings;
  root.innerHTML = '';

  const container = document.createElement('div');
  container.className = 'main-container';

  const title = document.createElement('h1');
  title.textContent = 'Gmail Tray Notifier';
  container.appendChild(title);

  const statusChip = document.createElement('div');
  statusChip.className = 'status-chip';
  statusChip.textContent = state.authorised ? 'Авторизация выполнена' : 'Требуется вход';
  if (!state.authorised) {
    statusChip.classList.add('unauthorised');
  }
  container.appendChild(statusChip);

  const actions = document.createElement('div');
  actions.className = 'actions';

  const loginBtn = document.createElement('button');
  loginBtn.className = 'primary';
  loginBtn.textContent = state.authorised ? 'Повторная авторизация' : 'Войти в Gmail';
  loginBtn.addEventListener('click', async () => {
    loginBtn.disabled = true;
    try {
      await invoke('request_authorisation');
      const refreshed = await invoke('initialise');
      cachedSettings = refreshed.settings;
      statusChip.textContent = 'Авторизация выполнена';
      statusChip.classList.remove('unauthorised');
      loginBtn.textContent = 'Повторная авторизация';
    } catch (error) {
      console.error(error);
      alert('Ошибка авторизации: ' + error);
    } finally {
      loginBtn.disabled = false;
    }
  });

  const logoutBtn = document.createElement('button');
  logoutBtn.className = 'secondary';
  logoutBtn.textContent = 'Выйти';
  logoutBtn.addEventListener('click', async () => {
    logoutBtn.disabled = true;
    try {
      await invoke('revoke');
      statusChip.textContent = 'Требуется вход';
      statusChip.classList.add('unauthorised');
      loginBtn.textContent = 'Войти в Gmail';
    } catch (error) {
      console.error(error);
      alert('Не удалось удалить токены: ' + error);
    } finally {
      logoutBtn.disabled = false;
    }
  });

  const checkBtn = document.createElement('button');
  checkBtn.className = 'secondary';
  checkBtn.textContent = 'Проверить сейчас';
  checkBtn.addEventListener('click', async () => {
    checkBtn.disabled = true;
    try {
      await invoke('check_now');
    } catch (error) {
      alert('Не удалось выполнить проверку: ' + error);
    } finally {
      checkBtn.disabled = false;
    }
  });

  actions.append(loginBtn, logoutBtn, checkBtn);
  container.appendChild(actions);

  container.appendChild(buildSettingsForm(state.settings));

  const notice = document.createElement('div');
  notice.className = 'notice';
  notice.innerHTML = `OAuth2-клиент (Desktop app) создаётся в Google Cloud Console. <br/>` +
    `Укажите Client ID и секрет ниже. Токены автоматически сохраняются в системном хранилище.`;
  container.appendChild(notice);

  root.appendChild(container);
}

function buildSettingsForm(settings) {
  const form = document.createElement('form');
  form.className = 'settings';

  const intervalField = document.createElement('div');
  intervalField.className = 'field';
  const intervalLabel = document.createElement('label');
  intervalLabel.textContent = 'Интервал проверки (сек.)';
  const intervalInput = document.createElement('input');
  intervalInput.type = 'number';
  intervalInput.min = '15';
  intervalInput.max = '300';
  intervalInput.value = settings.poll_interval_secs ?? 30;
  intervalInput.name = 'pollInterval';
  intervalField.append(intervalLabel, intervalInput);

  const soundField = document.createElement('div');
  soundField.className = 'field';
  const soundRow = document.createElement('div');
  soundRow.className = 'row';
  const soundLabel = document.createElement('label');
  soundLabel.textContent = 'Звук уведомления';
  const soundToggle = document.createElement('input');
  soundToggle.type = 'checkbox';
  soundToggle.checked = settings.sound_enabled;
  soundToggle.name = 'soundEnabled';
  const soundPath = document.createElement('input');
  soundPath.type = 'text';
  soundPath.placeholder = 'Путь к файлу (mp3/wav)';
  soundPath.value = settings.sound_path ?? '';
  soundPath.name = 'soundPath';
  soundRow.append(soundToggle, soundPath);
  soundField.append(soundLabel, soundRow);

  const volumeField = document.createElement('div');
  volumeField.className = 'field';
  const volumeLabel = document.createElement('label');
  volumeLabel.textContent = 'Громкость (0-1)';
  const volumeInput = document.createElement('input');
  volumeInput.type = 'number';
  volumeInput.step = '0.05';
  volumeInput.min = '0';
  volumeInput.max = '1';
  volumeInput.value = settings.playback_volume ?? 0.7;
  volumeInput.name = 'volume';
  volumeField.append(volumeLabel, volumeInput);

  const launchField = document.createElement('div');
  launchField.className = 'field';
  const launchHeader = document.createElement('label');
  launchHeader.textContent = 'Автозапуск';
  const launchRow = document.createElement('div');
  launchRow.className = 'row';
  const launchToggle = document.createElement('input');
  launchToggle.type = 'checkbox';
  launchToggle.checked = settings.auto_launch;
  launchToggle.name = 'autoLaunch';
  const launchText = document.createElement('span');
  launchText.textContent = 'Запускать при входе в систему';
  launchRow.append(launchToggle, launchText);
  launchField.append(launchHeader, launchRow);

  const queryField = document.createElement('div');
  queryField.className = 'field';
  const queryLabel = document.createElement('label');
  queryLabel.textContent = 'Поисковый запрос Gmail';
  const queryInput = document.createElement('textarea');
  queryInput.name = 'gmailQuery';
  queryInput.value = settings.gmail_query ?? 'is:unread category:primary';
  queryField.append(queryLabel, queryInput);

  const clientField = document.createElement('div');
  clientField.className = 'field';
  const clientLabel = document.createElement('label');
  clientLabel.textContent = 'OAuth Client ID';
  const clientInput = document.createElement('input');
  clientInput.type = 'text';
  clientInput.name = 'clientId';
  clientInput.value = settings.oauth_client_id ?? '';
  clientField.append(clientLabel, clientInput);

  const secretField = document.createElement('div');
  secretField.className = 'field';
  const secretLabel = document.createElement('label');
  secretLabel.textContent = 'OAuth Client Secret (опционально)';
  const secretInput = document.createElement('input');
  secretInput.type = 'text';
  secretInput.name = 'clientSecret';
  secretInput.value = settings.oauth_client_secret ?? '';
  secretField.append(secretLabel, secretInput);

  const submitRow = document.createElement('div');
  submitRow.className = 'actions';
  const submitBtn = document.createElement('button');
  submitBtn.type = 'submit';
  submitBtn.className = 'primary';
  submitBtn.textContent = 'Сохранить настройки';
  submitRow.append(submitBtn);

  form.append(
    intervalField,
    soundField,
    volumeField,
    launchField,
    queryField,
    clientField,
    secretField,
    submitRow
  );

  form.addEventListener('submit', async (event) => {
    event.preventDefault();
    submitBtn.disabled = true;
    try {
      const update = {
        poll_interval_secs: Number(intervalInput.value),
        sound_enabled: soundToggle.checked,
        sound_path: soundPath.value ? soundPath.value : null,
        auto_launch: launchToggle.checked,
        gmail_query: queryInput.value,
        oauth_client_id: clientInput.value,
        oauth_client_secret: secretInput.value ? secretInput.value : null,
        playback_volume: Number(volumeInput.value),
      };
      const saved = await invoke('update_settings', { update });
      cachedSettings = saved;
      submitBtn.textContent = 'Сохранено!';
      setTimeout(() => (submitBtn.textContent = 'Сохранить настройки'), 2000);
    } catch (error) {
      alert('Не удалось сохранить настройки: ' + error);
    } finally {
      submitBtn.disabled = false;
    }
  });

  return form;
}

async function initAlert() {
  const root = document.querySelector('#app');
  const state = await invoke('initialise');
  cachedSettings = state.settings;

  await appWindow.hide();
  await appWindow.setAlwaysOnTop(true);

  const shell = document.createElement('div');
  shell.className = 'alert-shell toast-hidden';

  const card = document.createElement('div');
  card.className = 'alert-card';

  const content = document.createElement('div');
  content.className = 'alert-content';

  const title = document.createElement('div');
  title.className = 'alert-title';

  const meta = document.createElement('div');
  meta.className = 'alert-meta';

  const snippet = document.createElement('div');
  snippet.className = 'alert-snippet';

  content.append(title, meta, snippet);

  const actions = document.createElement('div');
  actions.className = 'alert-actions';

  const openBtn = document.createElement('button');
  openBtn.className = 'open';
  openBtn.textContent = 'Перейти';

  const readBtn = document.createElement('button');
  readBtn.className = 'read';
  readBtn.textContent = 'Прочитано';

  const dismissBtn = document.createElement('button');
  dismissBtn.className = 'dismiss';
  dismissBtn.textContent = 'Скрыть';

  actions.append(openBtn, readBtn, dismissBtn);
  card.append(content, actions);
  shell.append(card);
  root.append(shell);

  let currentNotification = null;
  let autoHideTimer = null;
  let isHovering = false;

  const scheduleHide = () => {
    clearTimeout(autoHideTimer);
    autoHideTimer = setTimeout(async () => {
      if (!isHovering) {
        await hideToast(false);
      }
    }, 9000);
  };

  const hideToast = async (notifyBackend = true) => {
    clearTimeout(autoHideTimer);
    shell.classList.add('toast-hidden');
    await appWindow.hide();
    currentNotification = null;
    if (!notifyBackend) {
      return;
    }
    try {
      await invoke('dismiss_notification');
    } catch (error) {
      console.warn('dismiss failed', error);
    }
  };

  shell.addEventListener('mouseenter', () => {
    isHovering = true;
    clearTimeout(autoHideTimer);
  });
  shell.addEventListener('mouseleave', () => {
    isHovering = false;
    scheduleHide();
  });

  openBtn.addEventListener('click', async () => {
    if (!currentNotification) return;
    try {
      await invoke('open_in_browser', { url: currentNotification.url });
      await hideToast(false);
    } catch (error) {
      console.error(error);
    }
  });

  readBtn.addEventListener('click', async () => {
    if (!currentNotification) return;
    try {
      await invoke('mark_message_read', { messageId: currentNotification.id });
      await hideToast(false);
    } catch (error) {
      console.error(error);
    }
  });

  dismissBtn.addEventListener('click', () => hideToast(true));

  const renderNotification = async (notification) => {
    currentNotification = notification;
    title.textContent = notification.subject;
    meta.textContent = notification.sender ?? 'Gmail';
    snippet.textContent = notification.snippet ?? '';
    shell.classList.remove('toast-hidden');
    await appWindow.show();
    await appWindow.setFocus();
    playSound();
    scheduleHide();
  };

  const playSound = async () => {
    if (!cachedSettings || !cachedSettings.sound_enabled) {
      return;
    }
    if (!cachedSettings.sound_path) {
      return;
    }
    try {
      const src = await convertFileSrc(cachedSettings.sound_path);
      const audio = new Audio(src);
      audio.volume = cachedSettings.playback_volume ?? 0.7;
      await audio.play();
    } catch (error) {
      console.warn('failed to play sound', error);
    }
  };

  listen('gmail://notification', async (event) => {
    const notification = event.payload;
    if (notification) {
      await renderNotification(notification);
    }
  });

  listen('gmail://settings', (event) => {
    if (event.payload) {
      cachedSettings = event.payload;
    }
  });
}

document.addEventListener('DOMContentLoaded', init);
