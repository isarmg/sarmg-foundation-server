import React, { StrictMode, useState } from "react";
import { createRoot } from "react-dom/client";
import { createXcssAdminApplication, useAdminApplication, InstanceHeaderActions, InstanceNameField, AccountPage } from "../../../dist/admin-shell/index.js";
import { createAdministratorApiClient } from "../../../dist/admin-web/index.js";
import { Button, Dialog, FormField, TextField, DateRangeField } from "../../../dist/admin-ui/index.js";
import "../../../web/design-tokens/tokens.css";
import "../../../web/design-tokens/tokens.dark.css";
import "../../../web/design-tokens/reset.css";
import "../../../web/design-tokens/accessibility.css";
import "../../../web/web-fonts/fonts.css";
import "../../../web/admin-ui/styles.css";

function WorkspaceFixture() {
  const [creating,setCreating] = useState(false); const [revision,setRevision] = useState(0);
  return <section className="xcss-content-stack"><InstanceHeaderActions create={() => setCreating(true)} refresh={() => setRevision(value=>value+1)} /><h1>Full-width workspace</h1><p>Revision {revision}</p>{creating && <Dialog title="New instance" onClose={()=>setCreating(false)}><FormField label="Instance name"><InstanceNameField defaultValue={new URLSearchParams(window.location.search).get('instanceName') ?? ''} /></FormField></Dialog>}</section>;
}
function DateRangeFixture() {
  const [range, setRange] = useState({ start: "2022-02-01", end: "2023-02-02" });
  const [count, setCount] = useState(0);
  return <section className="xcss-content-panel"><label htmlFor="fixture-date-start-year">Date range</label>
    <DateRangeField id="fixture-date" value={range} onApply={value => { setRange(value); setCount(count => count + 1); }} />
    <p data-testid="applied-range">{range.start}/{range.end} ({count})</p>
  </section>;
}
function ProductRoutes() {
  const { notify } = useAdminApplication();
  const [dialog, setDialog] = useState(false);
  const [failed, setFailed] = useState(false);
  if (failed) throw Object.assign(new Error("SECRET internal path /private/database"), { requestId: "render-123" });
  if (window.location.hash === "#account") return <AccountPage />;
  if (window.location.hash === "#workspace") return <WorkspaceFixture />;
  if (window.location.hash === "#dates") return <DateRangeFixture />;
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
const App = createXcssAdminApplication({
  product: { name: "xcss acceptance" },
  client: createAdministratorApiClient(),
  navigation: [{ label: "Overview", href: "#overview" }, { label: "Activity", href: "#activity" }],
  loginLandingHref: new URLSearchParams(window.location.search).has("loginLanding")
    ? new URLSearchParams(window.location.search).get("loginLanding") === "path" ? "/#workspace" : "#workspace" : undefined,
  routes: <ProductRoutes />,
  workspace: window.location.search === "?workspace=custom" ? {appearance:"custom-brand",selection:"custom"} : undefined,
});
createRoot(document.getElementById("root")).render(<StrictMode><App /></StrictMode>);
