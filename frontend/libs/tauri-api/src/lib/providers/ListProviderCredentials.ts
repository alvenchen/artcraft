import { invoke } from "@tauri-apps/api/core";
import { CommandResult } from "../common/CommandStatus";
import {
  ProviderCredentialKey,
  ProviderCredentialType,
} from "./ProviderCredentialKey";

export interface ListProviderCredentialsSuccess extends CommandResult {
  payload: ListProviderCredentialsPayload;
}

export interface ListProviderCredentialsPayload {
  providers: ProviderCredentialEntry[];
}

export interface ProviderCredentialEntry {
  provider_credential: ProviderCredentialKey;
  credential_type: ProviderCredentialType;
  has_credentials: boolean;
  maybe_details?: ProviderCredentialDetails;
}

export interface ProviderCredentialDetails {
  // The first few characters of the stored API key, followed by asterisks.
  maybe_key_start?: string;
}

// Returns the Success and Error variants directly.
// Throws on Network/Tauri errors.
export const ListProviderCredentials =
  async (): Promise<ListProviderCredentialsSuccess> => {
    return (await invoke(
      "provider_list_command",
    )) as ListProviderCredentialsSuccess;
  };
