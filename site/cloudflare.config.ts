import { defineConfig } from "cf/config";

export default defineConfig({
  worker: {
    name: "fnspeak-site",
    compatibilityDate: "2026-10-19",
    workersDev: false,
    domains: ["fnspeak.com", "www.fnspeak.com"],
    observability: { enabled: true },
  },
});
