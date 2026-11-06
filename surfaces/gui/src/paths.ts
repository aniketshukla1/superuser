/** The folder name at the end of a path, for showing a workspace or a root. Both separators
 *  count: the sidecar reports paths in the operating system's own form, and a Windows path
 *  such as "C:\Users\X\proj" must not show as one long segment. */
export function baseName(path: string): string {
  const parts = path.split(/[/\\]/).filter((part) => part.length > 0);
  return parts.length > 0 ? parts[parts.length - 1] : path;
}
