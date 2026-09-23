import React, { StrictMode, useState } from "react";
import { createRoot } from "react-dom/client";
import { createSarmgAdminApplication, useAdminApplication, InstanceHeaderActions, InstanceNameField } from "../../../packages/admin-shell/dist/index.js";
import { createAdministratorApiClient } from "../../../packages/admin-web/dist/index.js";
import { Button, Dialog, FormField, TextField } from "../../../packages/admin-ui/dist/index.js";
import "../../../packages/design-tokens/tokens.css";
import "../../../packages/design-tokens/tokens.dark.css";
import "../../../packages/design-tokens/reset.css";
import "../../../packages/design-tokens/accessibility.css";
import "../../../packages/web-fonts/fonts.css";
import "../../../packages/admin-ui/styles.css";

function WorkspaceFixture() {
  const [creating,setCreating] = useState(false); const [revision,setRevision] = useState(0);
  return <section className="sarmg-content-stack"><InstanceHeaderActions create={() => setCreating(true)} refresh={() => setRevision(value=>value+1)} /><h1>Full-width workspace</h1><p>Revision {revision}</p>{creating && <Dialog title="New instance" onClose={()=>setCreating(false)}><FormField label="Instance name"><InstanceNameField defaultValue={new URLSearchParams(window.location.search).get('instanceName') ?? ''} /></FormField></Dialog>}</section>;
}
function ProductRoutes() {
  const { notify } = useAdminApplication();
  const [dialog, setDialog] = useState(false);
  const [failed, setFailed] = useState(false);
  if (failed) throw Object.assign(new Error("SECRET internal path /private/database"), { requestId: "render-123" });
  if (window.location.hash === "#workspace") return <WorkspaceFixture />;
  return <section><h1>Product overview</h1>
    <Button onClick={() => setDialog(true)}>Open modal</Button>
    <Button onClick={() => notify("Saved successfully")}>Show notification</Button>
    <Button onClick={() => setFailed(true)}>Crash product route</Button>
    {dialog && <Dialog title="Test modal" onClose={() => setDialog(false)}>
      <FormField label="Test input"><TextField /></FormField>
      <Button onClick={() => setDialog(false)}>Done</Button>
    </Dialog>}
  </section>;
}
const App = createSarmgAdminApplication({
  product: { name: "Foundation acceptance" },
  client: createAdministratorApiClient(),
  navigation: [{ label: "Overview", href: "#overview" }, { label: "Activity", href: "#activity" }],
  loginLandingHref: new URLSearchParams(window.location.search).has("loginLanding") ? "#workspace" : undefined,
  routes: <ProductRoutes />,
  workspace: window.location.search === "?workspace=custom" ? {appearance:"custom-brand",selection:"custom"} : undefined,
});
createRoot(document.getElementById("root")).render(<StrictMode><App /></StrictMode>);
