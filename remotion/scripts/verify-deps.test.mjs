import assert from "node:assert/strict";
import test from "node:test";
import { verifyDependencyLock } from "./verify-deps.mjs";

function validLock() {
  return {
    packages: {
      "node_modules/remotion": { version: "4.0.507" },
      "node_modules/@remotion/cli": { version: "4.0.507" },
      "node_modules/@remotion/renderer": { version: "4.0.507" },
      "node_modules/ws": { version: "8.21.0" },
      "node_modules/wrapper/node_modules/@remotion/player": { version: "4.0.507" },
      "node_modules/wrapper/node_modules/ws": { version: "8.21.0" },
      "node_modules/remotionish": { version: "0.0.1" },
      "node_modules/wrapper/node_modules/not-ws": { version: "0.0.1" },
    },
  };
}

test("accepts nested patched Remotion and ws packages without matching lookalikes", () => {
  assert.deepEqual(verifyDependencyLock(validLock()).failures, []);
});

test("rejects every stale nested ws copy", () => {
  const lock = validLock();
  lock.packages["node_modules/wrapper/node_modules/ws"] = { version: "8.20.1" };

  assert.match(
    verifyDependencyLock(lock).failures.join("\n"),
    /node_modules\/wrapper\/node_modules\/ws.*8\.20\.1/,
  );
});

test("requires root remotion and scoped packages to stay lockstep", () => {
  const lock = validLock();
  lock.packages["node_modules/wrapper/node_modules/@remotion/player"] = {
    version: "4.0.506",
  };

  assert.match(
    verifyDependencyLock(lock).failures.join("\n"),
    /not lockstep/,
  );
});

test("rejects a Remotion prerelease at the stable floor", () => {
  const lock = validLock();
  lock.packages["node_modules/remotion"] = { version: "4.0.479-alpha.1" };

  assert.throws(
    () => verifyDependencyLock(lock),
    /unparseable version: 4\.0\.479-alpha\.1/,
  );
});

test("rejects a ws prerelease at the stable floor", () => {
  const lock = validLock();
  lock.packages["node_modules/ws"] = { version: "8.21.0-beta.1" };

  assert.throws(
    () => verifyDependencyLock(lock),
    /unparseable version: 8\.21\.0-beta\.1/,
  );
});
