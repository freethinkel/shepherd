import solidPlugin from "@opentui/solid/bun-plugin";

// Standalone binary — the solid JSX transform runs through the bunfig preload while developing,
// so a compiled build has to register the same plugin explicitly.
await Bun.build({
  entrypoints: ["./src/modules/cli/index.ts"],
  target: "bun",
  plugins: [solidPlugin],
  compile: { outfile: "dist/shepherd" },
});
