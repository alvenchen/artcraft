import { useEffect, useState } from "react";
import { LoaderCircleIcon, Trash2Icon } from "lucide-react";
import { Button } from "@storyteller/ui-button";
import { Input } from "@storyteller/ui-input";
import { Switch } from "@storyteller/ui-switch";
import {
  ClearProviderCredential,
  GetAppPreferences,
  ListProviderCredentials,
  PreferenceName,
  ProviderCredentialEntry,
  ProviderCredentialKey,
  SetProviderApiKey,
  UpdateAppPreferences,
} from "@storyteller/tauri-api";

interface ProviderDescriptor {
  key: ProviderCredentialKey;
  name: string;
  helpText: string;
  placeholder: string;
  isSecret: boolean;
}

// Order here is the display order in the pane.
const PROVIDERS: ProviderDescriptor[] = [
  {
    key: ProviderCredentialKey.GrokApiKey,
    name: "Grok (xAI)",
    helpText: "xAI API key, separate from any Grok web login.",
    placeholder: "Enter your xAI API key",
    isSecret: true,
  },
  {
    key: ProviderCredentialKey.WanBaseUrl,
    name: "Wan — base URL",
    helpText:
      "Base URL of your Wan-AI gateway. Models are read from {base}/models.",
    placeholder: "https://your-gateway.example.com",
    isSecret: false,
  },
  {
    key: ProviderCredentialKey.WanApiKey,
    name: "Wan — API key",
    helpText: "API key for the Wan-AI gateway above.",
    placeholder: "Enter your Wan API key",
    isSecret: true,
  },
  {
    key: ProviderCredentialKey.SeedanceBaseUrl,
    name: "Seedance — base URL",
    helpText:
      "Volcano Engine (CN) or BytePlus (international) ARK API base URL, e.g. https://ark.cn-beijing.volces.com/api/v3 — or a ModelVerse base URL including its version prefix, e.g. https://api.modelverse.cn/v1.",
    placeholder: "https://api.modelverse.cn/v1",
    isSecret: false,
  },
  {
    key: ProviderCredentialKey.SeedanceApiKey,
    name: "Seedance — API key",
    helpText: "API key for the endpoint above.",
    placeholder: "Enter your Seedance API key",
    isSecret: true,
  },
];

export const ProviderApiKeysSettingsPane = () => {
  const [entries, setEntries] = useState<
    Record<string, ProviderCredentialEntry>
  >({});
  const [drafts, setDrafts] = useState<Record<string, string>>({});
  const [isLoading, setIsLoading] = useState(true);
  const [errorMessage, setErrorMessage] = useState<string | undefined>(
    undefined,
  );
  const [busyKey, setBusyKey] = useState<ProviderCredentialKey | undefined>(
    undefined,
  );
  const [useOwnApiKeys, setUseOwnApiKeys] = useState(false);

  useEffect(() => {
    const fetchPreferences = async () => {
      try {
        const result = await GetAppPreferences();
        setUseOwnApiKeys(result.preferences.use_own_api_keys ?? false);
      } catch (e) {
        console.error("Failed to load API keys preference", e);
      }
    };
    fetchPreferences();
  }, []);

  const toggleUseOwnApiKeys = async (enabled: boolean) => {
    try {
      await UpdateAppPreferences({ preference: PreferenceName.UseOwnApiKeys, value: enabled });
      setUseOwnApiKeys(enabled);
    } catch (e) {
      console.error("Failed to save API keys preference", e);
      setErrorMessage("Failed to save API keys preference.");
    }
  };

  const refresh = async () => {
    setIsLoading(true);
    try {
      const result = await ListProviderCredentials();
      const byKey: Record<string, ProviderCredentialEntry> = {};
      for (const entry of result.payload.providers) {
        byKey[entry.provider_credential] = entry;
      }
      setEntries(byKey);
      setErrorMessage(undefined);
    } catch (e) {
      console.error("Failed to load provider credentials", e);
      setErrorMessage("Failed to load provider credentials.");
    } finally {
      setIsLoading(false);
    }
  };

  useEffect(() => {
    refresh();
  }, []);

  const handleSave = async (descriptor: ProviderDescriptor) => {
    const value = (drafts[descriptor.key] ?? "").trim();
    if (!value) return;
    setBusyKey(descriptor.key);
    try {
      await SetProviderApiKey(descriptor.key, value);
      setDrafts((prev) => ({ ...prev, [descriptor.key]: "" }));
      await refresh();
    } catch (e) {
      console.error(`Failed to save ${descriptor.name} API key`, e);
      setErrorMessage(`Failed to save ${descriptor.name} API key.`);
    } finally {
      setBusyKey(undefined);
    }
  };

  const handleClear = async (descriptor: ProviderDescriptor) => {
    setBusyKey(descriptor.key);
    try {
      await ClearProviderCredential(descriptor.key);
      await refresh();
    } catch (e) {
      console.error(`Failed to clear ${descriptor.name} API key`, e);
      setErrorMessage(`Failed to clear ${descriptor.name} API key.`);
    } finally {
      setBusyKey(undefined);
    }
  };

  return (
    <div className="space-y-4 pt-3 text-base-fg">
      <p className="text-xs text-base-fg/70">
        Add your own provider API keys. Keys are stored locally on this device
        and used to generate directly with each provider — no ArtCraft account
        required.
      </p>

      <div className="flex items-center justify-between gap-3 rounded-[3px] border border-ui-controls-border bg-base-fg/[0.03] p-3">
        <div className="flex min-w-0 flex-col gap-0.5">
          <span className="font-medium">Use my own API keys</span>
          <span className="text-xs text-base-fg/60">
            Generate directly with the provider keys below instead of the ArtCraft backend.
          </span>
        </div>
        <Switch
          id="use-own-api-keys"
          label="Use my own API keys"
          enabled={useOwnApiKeys}
          setEnabled={toggleUseOwnApiKeys}
        />
      </div>

      {errorMessage && (
        <p className="text-xs text-red-500">{errorMessage}</p>
      )}

      {isLoading ? (
        <div className="flex items-center gap-2 font-mono text-[11px] uppercase tracking-[0.12em] text-base-fg/70">
          <LoaderCircleIcon className="animate-spin" />
          Loading...
        </div>
      ) : (
        <div className="flex flex-col gap-5">
          {PROVIDERS.map((descriptor) => {
            const entry = entries[descriptor.key];
            const hasCredentials = entry?.has_credentials ?? false;
            const isBusy = busyKey === descriptor.key;

            return (
              <div
                key={descriptor.key}
                className="flex flex-col gap-2 rounded-[3px] border border-ui-controls-border bg-base-fg/[0.03] p-3"
              >
                <div className="flex items-center justify-between gap-3">
                  <div className="flex min-w-0 flex-col gap-0.5">
                    <span className="font-medium">{descriptor.name}</span>
                    <span className="truncate text-xs text-base-fg/60">
                      {hasCredentials
                        ? `Configured (${entry?.maybe_details?.maybe_key_start ?? "********"})`
                        : descriptor.helpText}
                    </span>
                  </div>
                  {hasCredentials && (
                    <Button
                      variant="destructive"
                      className="h-9 shrink-0 px-3"
                      onClick={() => handleClear(descriptor)}
                      disabled={isBusy}
                    >
                      {isBusy ? (
                        <LoaderCircleIcon className="animate-spin text-sm" />
                      ) : (
                        <Trash2Icon className="text-sm" />
                      )}
                    </Button>
                  )}
                </div>

                <div className="flex items-center gap-2">
                  <Input
                    type={descriptor.isSecret ? "password" : "text"}
                    value={drafts[descriptor.key] ?? ""}
                    placeholder={descriptor.placeholder}
                    onChange={(e) =>
                      setDrafts((prev) => ({
                        ...prev,
                        [descriptor.key]: e.target.value,
                      }))
                    }
                    onKeyDown={(e) => {
                      if (e.key === "Enter") handleSave(descriptor);
                    }}
                    disabled={isBusy}
                    className="flex-1"
                  />
                  <Button
                    variant="secondary"
                    className="h-10 shrink-0 px-3"
                    onClick={() => handleSave(descriptor)}
                    disabled={isBusy || !(drafts[descriptor.key] ?? "").trim()}
                  >
                    {isBusy ? "Saving..." : "Save"}
                  </Button>
                </div>
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
};
