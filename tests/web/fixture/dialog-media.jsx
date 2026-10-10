import React, { StrictMode, useState } from "react";
import { createRoot } from "react-dom/client";
import { Button, Dialog } from "../../../dist/admin-ui/index.js";
import "../../../web/design-tokens/tokens.css";
import "../../../web/design-tokens/tokens.dark.css";
import "../../../web/design-tokens/reset.css";
import "../../../web/design-tokens/accessibility.css";
import "../../../web/admin-ui/styles.css";

// Two seconds of generated solid-blue VP8 video; no external media/network dependency.
const mediaSource = "data:video/webm;base64,GkXfo59ChoEBQveBAULygQRC84EIQoKEd2VibUKHgQJChYECGFOAZwEAAAAAAAMdEU2bdLpNu4tTq4QVSalmU6yBoU27i1OrhBZUrmtTrIHwTbuMU6uEElTDZ1OsggE9TbuMU6uEHFO7a1OsggMH7AEAAAAAAABZAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAVSalmyirXsYMPQkB7qZdkaWFsb2cta2V5Ym9hcmQtZml4dHVyZU2AjExhdmY2MS43LjEwM1dBjExhdmY2MS43LjEwM0SJiECfQAAAAAAAFlSua8iuAQAAAAAAAD/XgQFzxYjQx6KIOaWVbpyBACK1nIN1bmSIgQCGhVZfVlA4g4EBI+ODhAvrwgDgkLCBoLqBWpqBAlWwhFW5gQESVMNn+3Nzn2PAgGfImUWjh0VOQ09ERVJEh4xMYXZmNjEuNy4xMDNzc9ZjwItjxYjQx6KIOaWVbmfIoUWjh0VOQ09ERVJEh5RMYXZjNjEuMTkuMTAxIGxpYnZweGfIoUWjiERVUkFUSU9ORIeTMDA6MDA6MDIuMDAwMDAwMDAwAB9DtnVBROeBAKPWgQAAgPAFAJ0BKqAAWgAARwiFhYiFhIgCAgJ1qgP4AgaaE+CGqpNdxDqqTXcQ6qk13EOqpNdxDqqTXcQYAP7/TRL//FhX8WFfxYV/8WFf/PzO7cX85gCjmIEAyAARAgABEBAAGAAYWC/0AAiAgQAAAKOYgQGQABECAAEQEAAYABhYL/QACICBAAAAo5iBAlgAEQIAARAQABgAGFgv9AAIgIEAAACjmIEDIAARAgABEBAAGAAYWC/0AAiAgQAAAKOYgQPoABECAAEQEAAYABhYL/QACICBAAAAo5iBBLAAEQIAARAQABgAGFgv9AAIgIEAAACjl4EFeADxAQABEBAUYABhYL/QACICBAAAo5iBBkAAEQIAARAQABgAGFgv9AAIgIEAAACjmIEHCAARAgABEBAAGAAYWC/0AAiAgQAAABxTu2uRu4+zgQC3iveBAfGCAb3wgQM=";
function MediaDialogFixture() {
  const [open, setOpen] = useState(false);
  const [closed, setClosed] = useState(0);
  const [backgroundClicks, setBackgroundClicks] = useState(0);
  return <main style={{ padding: 16 }}>
    <Button data-background="opener" onClick={() => setOpen(true)}>Open camera preview</Button>
    <Button data-background="action" onClick={() => setBackgroundClicks(value => value + 1)}>Background action</Button>
    <a data-background="link" href="#background" onClick={event => { event.preventDefault(); setBackgroundClicks(value => value + 1); }}>Background link</a>
    <p>Background clicks: <output data-testid="background-clicks">{backgroundClicks}</output></p>
    <p>Closed: <output data-testid="closed-count">{closed}</output></p>
    {open && <Dialog title="Fixed camera preview" onClose={() => { setOpen(false); setClosed(value => value + 1); }}>
      <video aria-label="Camera preview" src={mediaSource} controls preload="auto" muted loop playsInline style={{ display: "block", width: 320, maxWidth: "100%" }} />
    </Dialog>}
  </main>;
}
createRoot(document.getElementById("root")).render(<StrictMode><MediaDialogFixture /></StrictMode>);
