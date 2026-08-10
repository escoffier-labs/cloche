#!/usr/bin/env node
/**
 * Regression guard for issue #10: Remotion must stay beyond 4.0.478 and ws >= 8.21.0.
 */
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const MIN_REMOTION = "4.0.479";
const MIN_WS = "8.21.0";

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

function packageNameFromPath(path) {
  const marker = "node_modules/";
  const index = path.lastIndexOf(marker);
  return index === -1 ? undefined : path.slice(index + marker.length);
}

function lockEntries(lock, packageName) {
  return Object.entries(lock.packages ?? {}).filter(
    ([path]) => packageNameFromPath(path) === packageName,
  );
}

function remotionEntries(lock) {
  return Object.entries(lock.packages ?? {}).filter((entry) =>
    packageNameFromPath(entry[0])?.startsWith("@remotion/"),
  );
}

/** Returns lockfile failures without printing, so the guard is unit-testable. */
export function verifyDependencyLock(lock) {
  const failures = [];
  const rootRemotion = lock.packages?.["node_modules/remotion"];
  const scopedRemotion = remotionEntries(lock);
  const wsEntries = lockEntries(lock, "ws");

  if (!rootRemotion?.version) {
    failures.push("missing root node_modules/remotion entry in package-lock.json");
  } else if (!gte(rootRemotion.version, MIN_REMOTION)) {
    failures.push(
      `remotion ${rootRemotion.version} is below patched floor ${MIN_REMOTION}`,
    );
  }

  if (scopedRemotion.length === 0) {
    failures.push("no @remotion/* packages found in package-lock.json");
  }

  const remotionVersions = new Set();
  if (rootRemotion?.version) {
    remotionVersions.add(rootRemotion.version);
  }
  for (const [path, pkg] of scopedRemotion) {
    if (!pkg.version) {
      failures.push(`missing version for ${path}`);
      continue;
    }
    remotionVersions.add(pkg.version);
    if (!gte(pkg.version, MIN_REMOTION)) {
      failures.push(
        `@remotion package ${path} ${pkg.version} is below patched floor ${MIN_REMOTION}`,
      );
    }
  }

  if (remotionVersions.size > 1) {
    failures.push(
      `remotion packages are not lockstep: ${[...remotionVersions].sort().join(", ")}`,
    );
  }

  if (wsEntries.length === 0) {
    failures.push("no ws packages found in package-lock.json");
  }
  for (const [path, pkg] of wsEntries) {
    if (!pkg.version) {
      failures.push(`missing version for ${path}`);
    } else if (!gte(pkg.version, MIN_WS)) {
      failures.push(`ws ${path} ${pkg.version} is below patched floor ${MIN_WS}`);
    }
  }

  return {
    failures,
    remotionVersion: rootRemotion?.version,
    wsVersions: wsEntries.map(([, pkg]) => pkg.version).filter(Boolean),
  };
}

function main() {
  const root = join(dirname(fileURLToPath(import.meta.url)), "..");
  const lock = JSON.parse(readFileSync(join(root, "package-lock.json"), "utf8"));
  const result = verifyDependencyLock(lock);

  if (result.failures.length > 0) {
    console.error("remotion dependency verification failed:");
    for (const failure of result.failures) {
      console.error(`  - ${failure}`);
    }
    process.exitCode = 1;
    return;
  }

  console.log(
    `remotion dependency verification passed (remotion/@remotion ${result.remotionVersion}, ws ${[...new Set(result.wsVersions)].sort().join(", ")})`,
  );
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  main();
}
