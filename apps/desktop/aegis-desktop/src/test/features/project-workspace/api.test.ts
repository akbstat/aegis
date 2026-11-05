import { describe, expect, it, vi } from "vitest";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

import { api } from "../../../shared/api";
import { mockCommands, mockInvoke } from "../../helpers/tauri-mock";

/**
 * The invoke payload keys each command expects.
 *
 * Tauri v2 derives them from the Rust commands' Rust parameter names:
 * `tauri-macros` lower-camel-cases each parameter, and `tauri`'s
 * resolver looks that key up in the payload with no whole-payload
 * fallback. The Rust side keeps its own copy of this list, checked
 * against the command signatures by
 * `commands::project::tests::invoke_keys_match_the_documented_shape`.
 */
const EXPECTED_INVOKE_KEYS: Record<string, string[]> = {
  list_project_branches: ["code"],
  create_project_branch: ["code", "body"],
  update_project_branch: ["code", "id", "body"],
};

const BRANCH = {
  id: 4,
  projectId: 9,
  branch: "2026Q1",
  createdAt: "2026-01-02T03:04:05Z",
  updatedAt: "2026-01-02T03:04:05Z",
};

describe("project branch api", () => {
  it("sends exactly the invoke keys the Rust commands expect", async () => {
    const cases: Array<[string, () => Promise<unknown>]> = [
      ["list_project_branches", () => api.listProjectBranches("pr1")],
      [
        "create_project_branch",
        () => api.createProjectBranch("pr1", { branch: "2026Q1" }),
      ],
      [
        "update_project_branch",
        () => api.updateProjectBranch("pr1", 4, { branch: "2026Q2" }),
      ],
    ];

    mockCommands({
      list_project_branches: () => [],
      create_project_branch: () => BRANCH,
      update_project_branch: () => BRANCH,
    });

    for (const [command, call] of cases) {
      mockInvoke.mockClear();
      await call();
      const payload = mockInvoke.mock.calls[0][1] as Record<string, unknown>;
      expect(Object.keys(payload).sort(), command).toEqual(
        [...EXPECTED_INVOKE_KEYS[command]].sort(),
      );
    }
  });

  it("invokes list_project_branches with the project code", async () => {
    mockCommands({ list_project_branches: () => [] });
    await api.listProjectBranches("pr1");
    expect(mockInvoke).toHaveBeenCalledWith("list_project_branches", {
      code: "pr1",
    });
  });

  it("resolves the branch list as a bare array", async () => {
    // The server wraps the list in `{ "branches": [...] }`; the Rust
    // http layer unwraps it, so the frontend sees a plain array — the
    // same shape `listProjects` returns.
    mockCommands({ list_project_branches: () => [BRANCH] });
    const branches = await api.listProjectBranches("pr1");
    expect(branches).toHaveLength(1);
    expect(branches[0].branch).toBe("2026Q1");
    expect(branches[0].projectId).toBe(9);
  });

  it("invokes create_project_branch with the branch name in the body", async () => {
    mockCommands({ create_project_branch: () => BRANCH });
    await api.createProjectBranch("pr1", { branch: "2026Q1" });
    expect(mockInvoke).toHaveBeenCalledWith("create_project_branch", {
      code: "pr1",
      body: { branch: "2026Q1" },
    });
  });

  it("invokes update_project_branch with the id and the body", async () => {
    mockCommands({ update_project_branch: () => BRANCH });
    await api.updateProjectBranch("pr1", 4, { branch: "2026Q2" });
    expect(mockInvoke).toHaveBeenCalledWith("update_project_branch", {
      code: "pr1",
      id: 4,
      body: { branch: "2026Q2" },
    });
  });
});
