import{copyFile}from"node:fs/promises";await copyFile(new URL("../styles.css",import.meta.url),new URL("../dist/styles.css",import.meta.url));
