/** Bun loads a markdown import as text, and `bun build --compile` embeds it in the binary. */
declare module "*.md" {
  const content: string;
  export default content;
}
