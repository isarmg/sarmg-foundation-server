export type AppShellConfig = {
  application: { id: string; name: string };
};

export function createAppShell(config: AppShellConfig) {
  return config;
}
