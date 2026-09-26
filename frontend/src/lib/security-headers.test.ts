import { describe, it, expect } from "vitest";
import { buildCsp, buildSecurityHeaders } from "./security-headers";

function parseCsp(csp: string): Record<string, string[]> {
  const directives: Record<string, string[]> = {};
  csp.split(';').forEach((part) => {
    const [name, ...values] = part.trim().split(/\s+/);
    if (name) {
      directives[name] = values;
    }
  });
  return directives;
}

describe("security headers", () => {
  it("forbids framing and allows only the needed connect origins", () => {
    const parsed = parseCsp(buildCsp({ wsUrl: "wss://api.example.com/ws" }));
    expect(parsed["frame-ancestors"]).toEqual(["'none'"]);
    expect(parsed["object-src"]).toEqual(["'none'"]);
    expect(parsed["connect-src"]).toContain("wss://api.example.com");
    expect(parsed["connect-src"]).toContain("https://*.stellar.org");
    expect(parsed["connect-src"]).toContain("https://*.sentry.io");
    expect(parsed["script-src"]).not.toContain("'unsafe-eval'");
  });

  it("allows unsafe-eval only in development", () => {
    const parsed = parseCsp(buildCsp({ isDev: true }));
    expect(parsed["script-src"]).toContain("'unsafe-eval'");
  });

  it("sends the CSP as report-only unless enforcement is enabled", () => {
    const reportOnly = buildSecurityHeaders();
    expect(reportOnly.map((h) => h.key)).toContain(
      "Content-Security-Policy-Report-Only",
    );
    const enforced = buildSecurityHeaders({ enforceCsp: true });
    expect(enforced.map((h) => h.key)).toContain("Content-Security-Policy");
    expect(enforced.map((h) => h.key)).not.toContain(
      "Content-Security-Policy-Report-Only",
    );
  });

  it("always includes the standard hardening headers", () => {
    const keys = buildSecurityHeaders().map((h) => h.key);
    for (const key of [
      "X-Frame-Options",
      "X-Content-Type-Options",
      "Referrer-Policy",
      "Permissions-Policy",
      "Strict-Transport-Security",
    ]) {
      expect(keys).toContain(key);
    }
  });

  it("uses nonce in script-src and drops unsafe-inline when nonce is provided", () => {
    const nonce = "test-nonce-abc123";
    const parsed = parseCsp(buildCsp({ nonce }));
    expect(parsed["script-src"]).toContain(`'nonce-${nonce}'`);
    expect(parsed["script-src"]).not.toContain("'unsafe-inline'");
  });

  it("falls back to unsafe-inline when no nonce is provided", () => {
    const parsed = parseCsp(buildCsp());
    expect(parsed["script-src"]).toContain("'unsafe-inline'");
    expect(parsed["script-src"].some(val => val.startsWith("'nonce-"))).toBe(false);
  });

  it("includes report-uri directive when reportTo is provided", () => {
    const parsed = parseCsp(buildCsp({ reportTo: "/api/csp-report" }));
    expect(parsed["report-uri"]).toEqual(["/api/csp-report"]);
  });

  it("omits report-uri when reportTo is not provided", () => {
    const parsed = parseCsp(buildCsp());
    expect(parsed["report-uri"]).toBeUndefined();
  });
});
