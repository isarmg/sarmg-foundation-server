import { createContext, useContext } from "react";
import type { AdministratorApiClient } from "@xcss/web/admin-web";
import type { AdministratorSessionController } from "@xcss/web/admin-web/react";

export type ApplicationContext = {
  client: AdministratorApiClient;
  session: NonNullable<AdministratorSessionController["session"]>;
  notify(message: string): void;
  accountUpdated(): void;
};
export const Context = createContext<ApplicationContext | null>(null);
export function useAdminApplication(): ApplicationContext {
  const context = useContext(Context);
  if (!context) throw new Error("Product routes must be inside the administrator application");
  return context;
}
