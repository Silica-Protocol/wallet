import { Injectable, signal, computed, inject, Renderer2, RendererFactory2 } from '@angular/core';

export type Theme = 'light' | 'dark' | 'system';

@Injectable({
  providedIn: 'root'
})
export class ThemeService {
  private renderer: Renderer2;
  private readonly STORAGE_KEY = 'chert_wallet_theme';

  // Theme state
  private currentTheme = signal<Theme>('system');
  private systemPrefersDark = signal(false);

  // Computed signals
  readonly theme = this.currentTheme.asReadonly();
  readonly isDark = computed(() => {
    const theme = this.currentTheme();
    if (theme === 'system') {
      return this.systemPrefersDark();
    }
    return theme === 'dark';
  });

  readonly themeClass = computed(() => this.isDark() ? 'dark' : 'light');

  constructor(rendererFactory: RendererFactory2) {
    this.renderer = rendererFactory.createRenderer(null, null);

    // Initialize theme from storage or system preference
    this.initializeTheme();

    // Listen for system theme changes
    this.listenForSystemThemeChanges();

    // Apply initial theme
    this.applyTheme();
  }

  private initializeTheme(): void {
    // Check for saved preference
    const savedTheme = localStorage.getItem(this.STORAGE_KEY) as Theme;
    if (savedTheme && ['light', 'dark', 'system'].includes(savedTheme)) {
      this.currentTheme.set(savedTheme);
    }

    // Check system preference
    this.updateSystemPreference();
  }

  private updateSystemPreference(): void {
    const prefersDark = window.matchMedia('(prefers-color-scheme: dark)').matches;
    this.systemPrefersDark.set(prefersDark);
  }

  private listenForSystemThemeChanges(): void {
    const mediaQuery = window.matchMedia('(prefers-color-scheme: dark)');
    const handler = (event: MediaQueryListEvent) => {
      this.systemPrefersDark.set(event.matches);
      // Re-apply theme if using system preference
      if (this.currentTheme() === 'system') {
        this.applyTheme();
      }
    };

    // Modern browsers
    if (mediaQuery.addEventListener) {
      mediaQuery.addEventListener('change', handler);
    } else {
      // Fallback for older browsers
      mediaQuery.addListener(handler);
    }
  }

  setTheme(theme: Theme): void {
    this.currentTheme.set(theme);
    localStorage.setItem(this.STORAGE_KEY, theme);
    this.applyTheme();
  }

  toggleTheme(): void {
    const current = this.currentTheme();
    if (current === 'light') {
      this.setTheme('dark');
    } else if (current === 'dark') {
      this.setTheme('system');
    } else {
      this.setTheme('light');
    }
  }

  private applyTheme(): void {
    const isDark = this.isDark();
    const document = this.renderer.selectRootElement('html', true);

    // Remove existing theme classes
    this.renderer.removeClass(document, 'light');
    this.renderer.removeClass(document, 'dark');

    // Add current theme class
    this.renderer.addClass(document, isDark ? 'dark' : 'light');

    // Update meta theme-color for mobile browsers
    const metaThemeColor = document.querySelector('meta[name="theme-color"]');
    if (metaThemeColor) {
      metaThemeColor.setAttribute('content', isDark ? '#0a0a0f' : '#ffffff');
    }
  }

  getThemeIcon(): string {
    const current = this.currentTheme();
    switch (current) {
      case 'light': return '☀️';
      case 'dark': return '🌙';
      case 'system': return '💻';
      default: return '☀️';
    }
  }

  getThemeLabel(): string {
    const current = this.currentTheme();
    switch (current) {
      case 'light': return 'Light Mode';
      case 'dark': return 'Dark Mode';
      case 'system': return 'System';
      default: return 'Light Mode';
    }
  }
}