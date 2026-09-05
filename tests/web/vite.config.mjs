import { fileURLToPath } from "node:url";
export default {
  root: fileURLToPath(new URL("./fixture", import.meta.url)),
  resolve: { dedupe: ["react", "react-dom"] },
};
