import { invoke } from "@tauri-apps/api/core";
import { CommandResult } from "../common/CommandStatus";
import { ProviderCredentialKey } from "./ProviderCredentialKey";

export interface ClearProviderCredentialSuccess extends CommandResult {
  payload: Record<string, never>;
}

// Returns the Success and Error variants directly.
// Throws on Network/Tauri errors.
export const ClearProviderCredential = async (
  providerCredential: ProviderCredentialKey,
): Promise<ClearProviderCredentialSuccess> => {
  return (await invoke("provider_clear_command", {
    request: {
      provider_credential: providerCredential,
    },
  })) as ClearProviderCredentialSuccess;
};
