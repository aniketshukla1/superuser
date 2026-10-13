import { Component, type ErrorInfo, type ReactNode } from "react";

// React unmounts the entire tree on an uncaught render throw. Before this existed, one
// non-conforming response — a 500 on /v1/inbox was enough — took `#root` to zero
// characters, and navigating away did not bring it back because there was nothing left
// to navigate (UX-REPORT §1).
//
// The boundary sits INSIDE the shell, around the surface switch only, so a broken
// surface costs the user that surface and not the sidebar: the nav stays live and they
// can walk to a working one. `resetKey` is the surface id — changing it clears the
// error, which is what makes "click another surface" a real recovery rather than a
// reload.

interface Props {
  children: ReactNode;
  resetKey: string;
  onReset?: () => void;
}

interface State {
  error: Error | null;
}

export class SurfaceBoundary extends Component<Props, State> {
  state: State = { error: null };

  static getDerivedStateFromError(error: Error): State {
    return { error };
  }

  componentDidUpdate(prev: Props) {
    if (prev.resetKey !== this.props.resetKey && this.state.error) this.setState({ error: null });
  }

  componentDidCatch(error: Error, info: ErrorInfo) {
    // Kept on the console for a bug report; the user gets the panel below.
    console.error("surface crashed:", error, info.componentStack);
  }

  render() {
    const { error } = this.state;
    if (!error) return this.props.children;
    return (
      <main className="flex-1 min-w-0 flex bg-paper" data-testid="surface-error">
        <div className="m-auto max-w-md px-8 py-10 text-center">
          <h2 className="text-lg font-semibold text-ink mb-1.5">This screen stopped working</h2>
          <p className="text-md text-muted leading-prose mb-4">
            Something in this view hit an error, so it has been unloaded. The rest of the
            app is unaffected — pick another screen from the sidebar, or try this one again.
          </p>
          <div className="flex items-center justify-center gap-2">
            <button
              className="text-md px-3 py-1.5 rounded-lg bg-accent text-onAccent hover:brightness-105"
              onClick={() => {
                this.setState({ error: null });
                this.props.onReset?.();
              }}
            >
              Try again
            </button>
            <button
              className="text-md px-3 py-1.5 rounded-lg border border-line text-ink hover:bg-panel"
              onClick={() => window.location.reload()}
            >
              Reload the app
            </button>
          </div>
          <details className="mt-5 text-left">
            <summary className="text-sm text-muted cursor-pointer">Error details</summary>
            <pre className="mt-2 text-xs text-muted whitespace-pre-wrap break-words">
              {error.message}
            </pre>
          </details>
        </div>
      </main>
    );
  }
}
