export default function HelloPage({
  rootPath,
  domains,
  tutorialDomainName,
  setTutorialDomainName,
  onOpenExistingRoot,
  onCreateRoot,
  onCreateFirstDomain,
  onContinue,
  busy,
  error,
}) {
  const hasRoot = !!rootPath;
  const hasDomain = domains.length > 0;

  return (
    <div
      style={{
        flex: 1,
        display: "flex",
        alignItems: "center",
        justifyContent: "center",
        padding: 20,
      }}
    >
      <div
        style={{
          width: "min(760px, 100%)",
          background: "#ffffff",
          border: "1px solid #dbe5f4",
          borderRadius: 14,
          boxShadow: "0 20px 40px rgba(15,23,42,0.08)",
          padding: 18,
          display: "grid",
          gap: 14,
        }}
      >
        <div>
          <h1 style={{ margin: 0, fontSize: 22 }}>Welcome to Project L</h1>
          <p style={{ margin: "6px 0 0", color: "#64748b", fontSize: 14 }}>
            This app helps you learn autonomously with a 3-level model: root
            overview, domain intelligence, and item-level detail.
          </p>
        </div>

        <div style={{ borderTop: "1px solid #e2e8f0", paddingTop: 12 }}>
          <h3 style={{ margin: 0, fontSize: 15 }}>Step 1 — Choose your root</h3>
          <p style={{ margin: "4px 0 10px", color: "#64748b", fontSize: 13 }}>
            Open an existing folder or create a new root folder.
          </p>
          <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
            <button type="button" onClick={onOpenExistingRoot} disabled={busy}>
              Open folder as root
            </button>
            <button type="button" onClick={onCreateRoot} disabled={busy}>
              Create new root folder
            </button>
          </div>
          <div style={{ marginTop: 8, fontSize: 12, color: "#334155" }}>
            {hasRoot ? `Root: ${rootPath}` : "No root selected yet."}
          </div>
        </div>

        {hasRoot ? (
          <div style={{ borderTop: "1px solid #e2e8f0", paddingTop: 12 }}>
            <h3 style={{ margin: 0, fontSize: 15 }}>
              Step 2 — Tutorial: create your first domain
            </h3>
            <p style={{ margin: "4px 0 10px", color: "#64748b", fontSize: 13 }}>
              A domain is one learning area (e.g., Mathematics, Japanese,
              Programming).
            </p>

            <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
              <input
                value={tutorialDomainName}
                onChange={(e) => setTutorialDomainName(e.target.value)}
                placeholder="Domain name"
                disabled={busy || hasDomain}
                style={{ minWidth: 220 }}
              />
              <button
                type="button"
                onClick={onCreateFirstDomain}
                disabled={busy || hasDomain}
              >
                Create domain folder
              </button>
            </div>

            <div style={{ marginTop: 8, fontSize: 12, color: "#334155" }}>
              {hasDomain
                ? `Tutorial complete: ${domains.length} domain(s) detected.`
                : "You must create at least one domain folder to continue."}
            </div>
          </div>
        ) : null}

        {error ? (
          <div
            style={{
              border: "1px solid #fecaca",
              background: "#fef2f2",
              color: "#b91c1c",
              borderRadius: 10,
              padding: 8,
              fontSize: 12,
            }}
          >
            {error}
          </div>
        ) : null}

        <div style={{ display: "flex", justifyContent: "flex-end" }}>
          <button
            type="button"
            disabled={!hasRoot || !hasDomain || busy}
            onClick={onContinue}
          >
            Continue to app
          </button>
        </div>
      </div>
    </div>
  );
}
