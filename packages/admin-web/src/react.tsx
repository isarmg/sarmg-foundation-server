import { useCallback, useEffect, useRef, useState } from "react";

import type { AdministratorSession } from "@sarmg/contracts";
import { isApiClientError } from "@sarmg/http-client";

import type { AdministratorApiClient } from "./index.js";

export type AdministratorSessionState =
  | { phase: "loading"; session: null; error: null }
  | { phase: "anonymous"; session: null; error: null }
  | { phase: "authenticated"; session: AdministratorSession; error: null }
  | { phase: "error"; session: null; error: unknown };

export type AdministratorSessionController = AdministratorSessionState & {
  login(username: string, password: string): Promise<void>;
  logout(): Promise<void>;
  restore(): Promise<void>;
};

/** One current admin-only authentication state machine shared by React apps. */
export function useAdministratorSession(
  client: AdministratorApiClient,
): AdministratorSessionController {
  const activeClient = useRef(client);
  const stateGeneration = useRef(0);
  activeClient.current = client;
  const [state, setState] = useState<AdministratorSessionState>(() => {
    const session = client.currentSession();
    return session === null
      ? { phase: "loading", session: null, error: null }
      : { phase: "authenticated", session, error: null };
  });

  const restore = useCallback(async () => {
    const generation = stateGeneration.current + 1;
    stateGeneration.current = generation;
    setState({ phase: "loading", session: null, error: null });
    try {
      const session = await client.restore();
      if (
        activeClient.current !== client ||
        stateGeneration.current !== generation
      ) return;
      setState({ phase: "authenticated", session, error: null });
    } catch (error) {
      if (
        activeClient.current !== client ||
        stateGeneration.current !== generation
      ) return;
      const current = client.currentSession();
      if (current !== null) {
        setState({ phase: "authenticated", session: current, error: null });
      } else if (
        isApiClientError(error) &&
        (error.status === 401 || error.code === "auth_operation_superseded")
      ) {
        setState({ phase: "anonymous", session: null, error: null });
      } else {
        setState({ phase: "error", session: null, error });
      }
    }
  }, [client]);

  useEffect(() => {
    const unsubscribe = client.subscribe((session) => {
      if (activeClient.current !== client) return;
      stateGeneration.current += 1;
      setState(
        session === null
          ? { phase: "anonymous", session: null, error: null }
          : { phase: "authenticated", session, error: null },
      );
    });
    void restore();
    return () => {
      unsubscribe();
      if (activeClient.current === client) stateGeneration.current += 1;
    };
  }, [client, restore]);

  const login = useCallback(async (username: string, password: string) => {
    const generation = stateGeneration.current + 1;
    stateGeneration.current = generation;
    setState({ phase: "loading", session: null, error: null });
    try {
      const session = await client.login(username, password);
      if (
        activeClient.current === client &&
        stateGeneration.current === generation
      ) {
        setState({ phase: "authenticated", session, error: null });
      }
    } catch (error) {
      if (
        activeClient.current === client &&
        stateGeneration.current === generation
      ) {
        const current = client.currentSession();
        setState(
          current === null
            ? { phase: "anonymous", session: null, error: null }
            : { phase: "authenticated", session: current, error: null },
        );
      }
      throw error;
    }
  }, [client]);

  const logout = useCallback(async () => {
    const generation = stateGeneration.current + 1;
    stateGeneration.current = generation;
    try {
      await client.logout();
    } finally {
      if (
        activeClient.current === client &&
        stateGeneration.current === generation
      ) {
        const current = client.currentSession();
        setState(
          current === null
            ? { phase: "anonymous", session: null, error: null }
            : { phase: "authenticated", session: current, error: null },
        );
      }
    }
  }, [client]);

  return { ...state, login, logout, restore };
}
