import React, { StrictMode, useState } from "react";
import { createRoot } from "react-dom/client";
import { AdministratorsPanel, createSarmgAdminApplication, useAdminApplication } from "../../../packages/admin-shell/dist/index.js";
import { createAdministratorApiClient } from "../../../packages/admin-web/dist/index.js";
import { Button, Dialog, FormField, TextField } from "../../../packages/admin-ui/dist/index.js";
import "../../../packages/design-tokens/tokens.css";
import "../../../packages/design-tokens/tokens.dark.css";
import "../../../packages/design-tokens/reset.css";
import "../../../packages/web-fonts/fonts.css";
import "../../../packages/admin-ui/styles.css";

function ProductRoutes() {
  const { notify } = useAdminApplication();
  const [dialog, setDialog] = useState(false);
  const [failed, setFailed] = useState(false);
  if (failed) throw Object.assign(new Error("SECRET internal path /private/database"), { requestId: "render-123" });
  if (window.location.hash === "#administrators") return <AdministratorsPanel />;
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
  product: { name: "Foundation acceptance", version: "0.5.0" },
  client: createAdministratorApiClient(),
  navigation: [{ label: "Overview", href: "#overview" }, { label: "Activity", href: "#activity" }, { label: "Administrators", href: "#administrators" }],
  routes: <ProductRoutes />,
});
createRoot(document.getElementById("root")).render(<StrictMode><App /></StrictMode>);
