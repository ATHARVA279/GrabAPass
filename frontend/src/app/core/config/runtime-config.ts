declare global {
  interface Window {
    __googleMapsAuthFailed?: boolean;
    google?: any;
    __runtimeConfig?: {
      GOOGLE_MAPS_API_KEY?: string;
    };
    gm_authFailure?: () => void;
  }
}

export function getRuntimeConfigValue(key: 'GOOGLE_MAPS_API_KEY'): string {
  return window.__runtimeConfig?.[key] ?? '';
}
