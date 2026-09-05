import {
  ADMINISTRATORS_PATH, isAdministratorCreateRequest, isAdministratorPasswordRequest, isAdministratorList,
  type AdministratorSummary,
} from "@sarmg/contracts";
import type { AdministratorApiClient } from "./index.js";

export type AdministratorManagementClient = {
  list(limit?: number, offset?: number, signal?: AbortSignal): Promise<AdministratorSummary[]>;
  create(username: string, password: string): Promise<void>;
  setPassword(id: string, password: string): Promise<void>;
  disable(id: string): Promise<void>;
};

/** Persistent-administrator capability only; no alternate paths or static account writes. */
export function createAdministratorManagementClient(client: AdministratorApiClient): AdministratorManagementClient {
  const empty = (value: unknown): value is undefined => value === undefined;
  const target = (id: string, action: "password" | "disable") => {
    if (!/^[A-Za-z0-9._:-]{1,128}$/.test(id)) throw new TypeError("Administrator ID is invalid");
    return `${ADMINISTRATORS_PATH}/${encodeURIComponent(id)}/${action}`;
  };
  return {
    list(limit = 50, offset = 0, signal) {
      if (!Number.isInteger(limit) || limit < 1 || limit > 100 || !Number.isSafeInteger(offset) || offset < 0) {
        throw new TypeError("Administrator pagination is invalid");
      }
      return client.request(`${ADMINISTRATORS_PATH}?limit=${limit}&offset=${offset}`, isAdministratorList, { signal });
    },
    async create(username, password) {
      const input = { username, password };
      if (!isAdministratorCreateRequest(input)) throw new TypeError("Administrator input is invalid");
      await client.request(ADMINISTRATORS_PATH, empty, { method: "POST", body: JSON.stringify(input) });
    },
    async setPassword(id, password) {
      const input = { password };
      if (!isAdministratorPasswordRequest(input)) throw new TypeError("Administrator password is invalid");
      await client.request(target(id, "password"), empty, { method: "POST", body: JSON.stringify(input) });
    },
    async disable(id) { await client.request(target(id, "disable"), empty, { method: "POST" }); },
  };
}
