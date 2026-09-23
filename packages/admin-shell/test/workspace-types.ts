import { createSarmgAdminApplication, resolveWorkspaceConfig, validInstanceName, type WorkspaceConfig } from "../dist/index.js";
const custom: Partial<WorkspaceConfig> = {appearance:"my-brand",fontFamily:"sans-serif",instanceNameMaxCharacters:16,selection:"custom",headerControls:"text"};
resolveWorkspaceConfig(custom);
validInstanceName("Example",16);
createSarmgAdminApplication({product:{name:"Custom"},navigation:[],routes:null,workspace:custom});
// @ts-expect-error WorkspaceConfig accepts only public workspace properties.
resolveWorkspaceConfig({layout:"custom"});
