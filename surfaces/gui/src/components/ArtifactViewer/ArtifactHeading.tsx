import type { ArtifactInfo } from "../../api";
import { exactTime, timeAgo } from "../../when";
import { formatBytes, kindLabel } from "./kindLabel";

/** Name, then what the name does not say: where it lives, what it is, size, last change. */
export function ArtifactHeading({ artifact, dir }: { artifact: ArtifactInfo; dir: string }) {
  return (
    <div className="artifact-heading">
      <div className="artifact-title">{artifact.name}</div>
      {/* The subline used to repeat the path, which for a file at the top of the
          workspace is the name again — the same word twice, telling you nothing.
          It now carries what you cannot see from the name: where it lives, what
          it is, how big, and when it last changed. */}
      <div className="artifact-meta">
        {dir && <span className="artifact-dir">{dir}</span>}
        {kindLabel(artifact.kind) && <span>{kindLabel(artifact.kind)}</span>}
        {artifact.size > 0 && <span>{formatBytes(artifact.size)}</span>}
        {/* An age with the exact stamp on hover: a bare "4:26 PM" did not say which day. */}
        {artifact.modified_at > 0 && (
          <span title={exactTime(artifact.modified_at)}>changed {timeAgo(artifact.modified_at)}</span>
        )}
      </div>
    </div>
  );
}
