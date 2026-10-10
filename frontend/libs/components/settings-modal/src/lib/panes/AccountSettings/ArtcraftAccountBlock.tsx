import { useState, useEffect } from "react";
import { LoaderCircleIcon } from "lucide-react";
import { Button } from "@storyteller/ui-button";

import { UsersApi } from "@storyteller/api";
import { UserInfo } from "@storyteller/api";
// TODO(login-disabled): Restore alongside the triggerRecheck call sites.
// import { useLoginModalStore } from "@storyteller/ui-login-modal";
import { invoke } from "@tauri-apps/api/core";

const usersApi = new UsersApi();

export interface ArtcraftAccountBlockProps {
  globalAccountLogoutCallback: () => void;
}

export const ArtcraftAccountBlock = ({
  globalAccountLogoutCallback,
}: ArtcraftAccountBlockProps) => {
  // TODO(login-disabled): Restore once login entry points are re-enabled:
  //   const { triggerRecheck } = useLoginModalStore();
  const [artcraftSession, setArtcraftSession] = useState<UserInfo | undefined>(
    undefined
  );
  const [isLoggedIn, setIsLoggedIn] = useState<boolean>(false);
  const [isCheckingArtcraftSession, setIsCheckingArtcraftSession] =
    useState(false);

  useEffect(() => {
    const fetchSession = async () => {
      setIsCheckingArtcraftSession(true);
      try {
        const result = await usersApi.GetSession();
        console.log(">>> result", result);
        setArtcraftSession(result?.data?.user);
        setIsLoggedIn(result?.data?.loggedIn || false);
      } catch (e) {
        console.error("Error fetching Artcraft session", e);
        setArtcraftSession(undefined);
        setIsLoggedIn(false);
      } finally {
        setIsCheckingArtcraftSession(false);
      }
    };
    fetchSession();
  }, []);

  const handleArtcraftButton = async () => {
    if (isCheckingArtcraftSession) return;
    if (isLoggedIn) {
      setIsCheckingArtcraftSession(true);
      await usersApi.Logout();
      setArtcraftSession(undefined);
      setIsLoggedIn(false);
      setIsCheckingArtcraftSession(false);
      globalAccountLogoutCallback(); // TODO: This resets the old global application state


      // TODO(login-disabled): Re-enable once login entry points are restored.
      // triggerRecheck(); // Trigger modal to recheck session and potentially open

      await invoke("storyteller_purge_credentials_command");

    } else {
      // TODO(login-disabled): Re-enable once login entry points are restored.
      // triggerRecheck(); // Open login modal instead of redirecting
    }
  };

  return (
    <div className="flex items-center justify-between gap-3">
      <div className="flex min-w-0 flex-col gap-0.5">
        <p className="font-mono text-[11px] font-semibold uppercase tracking-[0.12em]">
          ArtCraft account
        </p>
        <p className="truncate text-sm font-medium text-base-fg/80">
          {artcraftSession?.display_name || "Not logged in"}
        </p>
      </div>
      {/* Login is disabled for now, so the signed-out state has no action. The
          button only renders while the session check is in flight or when a
          signed-in account can log out. */}
      {(isCheckingArtcraftSession || isLoggedIn) && (
        <Button
          variant={isCheckingArtcraftSession ? "secondary" : "destructive"}
          className="h-9 shrink-0 px-3"
          onClick={handleArtcraftButton}
          disabled={isCheckingArtcraftSession}
        >
          {isCheckingArtcraftSession ? (
            <LoaderCircleIcon className="animate-spin text-sm" />
          ) : (
            "Log Out"
          )}
        </Button>
      )}
    </div>
  );
};
