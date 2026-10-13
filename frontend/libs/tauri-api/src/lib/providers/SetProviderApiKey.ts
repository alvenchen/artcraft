import { invoke } from "@tauri-apps/api/core";
import { CommandResult } from "../common/CommandStatus";
import { ProviderCredentialKey } from "./ProviderCredentialKey";

export interface SetProviderApiKeySuccess extends CommandResult {
  payload: Record<string, never>;
}

// Returns the Success and Error variants directly.
// Throws on Network/Tauri errors.
export const SetProviderApiKey = async (
  providerCredential: ProviderCredentialKey,
  apiKey: string,
): Promise<SetProviderApiKeySuccess> => {
  return (await invoke("provider_set_api_key_command", {
    request: {
      provider_credential: providerCredential,
      api_key: apiKey,
    },
  })) as SetProviderApiKeySuccess;
};
