#!/usr/bin/env node
/**
 * Regression guard for issue #10: Remotion must stay beyond 4.0.478 and ws >= 8.21.0.
 */
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const MIN_REMOTION = "4.0.479";
const MIN_WS = "8.21.0";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const lock = JSON.parse(readFileSync(join(root, "package-lock.json"), "utf8"));

function parseVersion(version) {
  const match = /^(\d+)\.(\d+)\.(\d+)/.exec(version);
  if (!match) {
    throw new Error(`unparseable version: ${version}`);
  }
  return [Number(match[1]), Number(match[2]), Number(match[3])];
}

function cmp(a, b) {
  for (let i = 0; i < 3; i += 1) {
    if (a[i] !== b[i]) {
      return a[i] - b[i];
    }
  }
  return 0;
}

function gte(version, floor) {
  return cmp(parseVersion(version), parseVersion(floor)) >= 0;
}

const failures = [];

const remotionPkg = lock.packages?.["node_modules/remotion"];
if (!remotionPkg?.version) {
  failures.push("missing node_modules/remotion entry in package-lock.json");
} else if (!gte(remotionPkg.version, MIN_REMOTION)) {
  failures.push(
    `remotion ${remotionPkg.version} is below patched floor ${MIN_REMOTION}`,
  );
}

const wsPkg = lock.packages?.["node_modules/ws"];
if (!wsPkg?.version) {
  failures.push("missing node_modules/ws entry in package-lock.json");
} else if (!gte(wsPkg.version, MIN_WS)) {
  failures.push(`ws ${wsPkg.version} is below patched floor ${MIN_WS}`);
}

const remotionVersions = new Set();
for (const [name, pkg] of Object.entries(lock.packages ?? {})) {
  if (!name.startsWith("node_modules/@remotion/")) {
    continue;
  }
  if (!pkg.version) {
    continue;
  }
  remotionVersions.add(pkg.version);
}

if (remotionVersions.size === 0) {
  failures.push("no @remotion/* packages found in package-lock.json");
} else if (remotionVersions.size > 1) {
  failures.push(
    `@remotion packages are not lockstep: ${[...remotionVersions].sort().join(", ")}`,
  );
} else {
  const only = [...remotionVersions][0];
  if (!gte(only, MIN_REMOTION)) {
    failures.push(
      `@remotion/* ${only} is below patched floor ${MIN_REMOTION}`,
    );
  }
}

if (failures.length > 0) {
  console.error("remotion dependency verification failed:");
  for (const failure of failures) {
    console.error(`  - ${failure}`);
  }
  process.exit(1);
}

console.log(
  `remotion dependency verification passed (remotion/@remotion ${[...remotionVersions][0]}, ws ${wsPkg.version})`,
);
