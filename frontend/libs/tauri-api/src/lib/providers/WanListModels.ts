import { invoke } from "@tauri-apps/api/core";
import { CommandResult } from "../common/CommandStatus";

export interface WanListModelsSuccess extends CommandResult {
  payload: WanListModelsPayload;
}

export interface WanListModelsPayload {
  models: WanModelEntry[];
}

export interface WanModelEntry {
  id: string;
}

// Lists the Wan models exposed by the user's configured gateway
// (`wan_list_models_command`). Requires the Wan base URL + API key to be set.
export const WanListModels = async (): Promise<WanListModelsSuccess> => {
  return (await invoke("wan_list_models_command")) as WanListModelsSuccess;
};
