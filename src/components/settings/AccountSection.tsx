/** Settings -> account: Zhimind API is the only available provider route. */
import { ProvidersPanel } from "@/components/ProvidersPanel";
import { resolveLocale } from "@/i18n";
import { useSettingsModel } from "@/providers/SettingsModelContext";
import type { SettingsViewModel } from "./types";

export function AccountSection() {
  const s = useSettingsModel() as SettingsViewModel & Record<string, any>;

  return (
    <ProvidersPanel
      locale={resolveLocale(s.locale)}
      onProvidersChanged={s.onProvidersChanged}
      onProviderActivated={s.onProviderActivated}
      onBalanceLoaded={s.onProviderBalanceLoaded}
    />
  );
}
