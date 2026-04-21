/**
 * i18n.svelte.ts — Reactive internationalization for Voice-typeless.
 *
 * Module-level $state tracks the active language.  Any component that calls
 * t() during rendering will automatically re-render when the language changes,
 * because Svelte 5's fine-grained reactivity tracks $state reads inside templates.
 *
 * Usage:
 *   import { t, setLang } from '../i18n.svelte';
 *   // In template:  {t('history.title')}
 *   // On language change:  setLang('zh')
 */

// ─── Reactive language state ─────────────────────────────────────────────────

const _lang = $state({ value: 'en' as 'en' | 'zh' });

export function setLang(lang: 'en' | 'zh'): void {
  _lang.value = lang;
}

export function getLang(): 'en' | 'zh' {
  return _lang.value;
}

// ─── Translation tables ──────────────────────────────────────────────────────

const _translations: Record<'en' | 'zh', Record<string, string>> = {
  en: {
    // App status
    'status.idle':       'Ready',
    'status.recording':  'Recording…',
    'status.processing': 'Processing…',
    'status.error':      'Error',

    // Navigation
    'nav.openSettings':  'Open settings',
    'nav.closeSettings': 'Close settings',

    // History panel
    'history.title':               'History',
    'history.empty.title':         'No history yet',
    'history.empty.subtitle':      'Press your hotkey and start speaking, or click ▶ for a demo.',
    'history.search.placeholder':  'Search history…',
    'history.noResults.title':     'No results',
    'history.noResults.subtitle':  'Try a different search term.',
    'history.clearConfirm':        'Clear all?',
    'history.results.singular':    'result',
    'history.results.plural':      'results',

    // Settings sections
    'settings.section.hotkeys':    'Hotkeys',
    'settings.section.audio':      'Audio',
    'settings.section.model':      'Speech Model',
    'settings.section.text':       'Text Processing',
    'settings.section.interface':  'Interface',
    'settings.section.system':     'System',

    // Hotkey fields
    'settings.hotkeys.desc':       'Click a field then press your desired key combination.',
    'settings.hotkeys.pushToTalk': 'Push-to-Talk',
    'settings.hotkeys.freeSpeech': 'Free Speech (toggle)',
    'settings.hotkeys.cancel':     'Cancel / Dismiss',

    // Audio
    'settings.audio.sounds':       'Notification sounds',
    'settings.audio.soundsHint':   'Marimba-style start/stop tones',
    'settings.audio.volume':       'Sound volume',

    // Model
    'settings.model.active':       'Active model',
    'settings.model.device':       'Inference device',
    'settings.model.deviceHint':   'DirectML recommended on Windows',

    // Text processing
    'settings.text.filterFiller':      'Filter filler words',
    'settings.text.filterFillerHint':  'Remove "um", "uh", "那個", "えーと"…',
    'settings.text.mixedLang':         'Mixed-language optimisation',
    'settings.text.mixedLangHint':     'Auto-space CJK/Latin boundaries, capitalise sentences',
    'settings.text.vadThreshold':      'Auto-stop silence',

    // Interface
    'settings.ui.theme':              'Theme',
    'settings.ui.language':           'Display language',
    'settings.ui.indicator':          'Floating indicator',
    'settings.ui.indicatorHint':      'Draggable overlay shown while recording',
    'settings.ui.retention':          'History retention',
    'settings.ui.maxHistory':         'Max history entries',
    'settings.ui.keepForever':        'Keep forever',

    // System
    'settings.system.autoStart':      'Launch at login',
    'settings.system.autoStartHint':  'Start Voice-typeless when you log in',
    'settings.system.tray':           'Minimize to tray',
    'settings.system.trayHint':       'Keep running in system tray when window is closed',
    'settings.system.updates':        'Check for updates',
    'settings.system.updatesHint':    'Auto-check GitHub releases on startup',

    // Save/Reset
    'settings.save':    'Save settings',
    'settings.saving':  'Saving…',
    'settings.saved':   'Settings saved successfully.',
    'settings.reset':   'Reset defaults',
  },

  zh: {
    // App status
    'status.idle':       '就緒',
    'status.recording':  '錄音中…',
    'status.processing': '處理中…',
    'status.error':      '錯誤',

    // Navigation
    'nav.openSettings':  '開啟設定',
    'nav.closeSettings': '關閉設定',

    // History panel
    'history.title':               '歷史記錄',
    'history.empty.title':         '尚無記錄',
    'history.empty.subtitle':      '按下快捷鍵開始說話，或點擊 ▶ 試用示範。',
    'history.search.placeholder':  '搜尋記錄…',
    'history.noResults.title':     '找不到結果',
    'history.noResults.subtitle':  '請嘗試其他搜尋詞。',
    'history.clearConfirm':        '清除全部？',
    'history.results.singular':    '筆結果',
    'history.results.plural':      '筆結果',

    // Settings sections
    'settings.section.hotkeys':    '快捷鍵',
    'settings.section.audio':      '音效',
    'settings.section.model':      '語音模型',
    'settings.section.text':       '文字處理',
    'settings.section.interface':  '介面',
    'settings.section.system':     '系統',

    // Hotkey fields
    'settings.hotkeys.desc':       '點擊欄位後按下您想要的按鍵組合。',
    'settings.hotkeys.pushToTalk': '按住說話',
    'settings.hotkeys.freeSpeech': '自由說話（切換）',
    'settings.hotkeys.cancel':     '取消 / 關閉',

    // Audio
    'settings.audio.sounds':       '提示音效',
    'settings.audio.soundsHint':   '馬林巴風格提示音',
    'settings.audio.volume':       '音量',

    // Model
    'settings.model.active':       '目前模型',
    'settings.model.device':       '推理裝置',
    'settings.model.deviceHint':   'Windows 建議使用 DirectML',

    // Text processing
    'settings.text.filterFiller':      '過濾語氣詞',
    'settings.text.filterFillerHint':  '移除「嗯」、「那個」、"um"、"uh"…',
    'settings.text.mixedLang':         '中英混合優化',
    'settings.text.mixedLangHint':     '自動處理中英文邊界與句首大寫',
    'settings.text.vadThreshold':      '自動停止靜音',

    // Interface
    'settings.ui.theme':              '主題',
    'settings.ui.language':           '顯示語言',
    'settings.ui.indicator':          '浮動指示器',
    'settings.ui.indicatorHint':      '錄音時顯示可拖曳的浮動視窗',
    'settings.ui.retention':          '歷史保留期間',
    'settings.ui.maxHistory':         '最多歷史條目',
    'settings.ui.keepForever':        '永久保留',

    // System
    'settings.system.autoStart':      '開機自動啟動',
    'settings.system.autoStartHint':  'Windows 啟動時自動開啟 Voice-typeless',
    'settings.system.tray':           '最小化至系統匣',
    'settings.system.trayHint':       '關閉視窗時繼續在系統匣運行',
    'settings.system.updates':        '檢查更新',
    'settings.system.updatesHint':    '啟動時自動檢查 GitHub 最新版本',

    // Save/Reset
    'settings.save':    '儲存設定',
    'settings.saving':  '儲存中…',
    'settings.saved':   '設定已成功儲存。',
    'settings.reset':   '重設為預設值',
  },
};

// ─── Translation function ─────────────────────────────────────────────────────

/**
 * Return the translation for `key` in the current language.
 * Falls back to English if the key is missing in the active language.
 * Falls back to the raw key string if missing in both languages.
 */
export function t(key: string): string {
  return _translations[_lang.value][key]
    ?? _translations.en[key]
    ?? key;
}
