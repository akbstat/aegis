import "@testing-library/jest-dom/vitest";
import { act, cleanup, renderHook, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import type { ReactNode } from "react";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

// The active branch lives in the settings store, so the hooks read and
// write it through the same `settings.bin` mock the settings feature
// uses.
const store = new Map<string, unknown>();
vi.mock("@tauri-apps/plugin-store", () => ({
  load: () =>
    Promise.resolve({
      get: (k: string) => Promise.resolve(store.get(k)),
      set: (k: string, v: unknown) => {
        store.set(k, v);
        return Promise.resolve();
      },
      save: () => Promise.resolve(),
    }),
}));

import { invoke } from "@tauri-apps/api/core";

import {
  useActiveBranchId,
  useCreateProjectBranch,
  useProjectBranches,
  useSetActiveBranchId,
  useUpdateProjectBranch,
} from "../../../features/project-workspace/data/project-branches";
import { queryKeys } from "../../../shared/query";

const BRANCH = {
  id: 4,
  projectId: 9,
  branch: "2026Q1",
  createdAt: "2026-01-02T03:04:05Z",
  updatedAt: "2026-01-02T03:04:05Z",
};

function makeClient() {
  return new QueryClient({
    defaultOptions: {
      queries: { retry: false },
      mutations: { retry: false },
    },
  });
}

function wrapperFor(client: QueryClient) {
  return function Wrapper({ children }: { children: ReactNode }) {
    return <QueryClientProvider client={client}>{children}</QueryClientProvider>;
  };
}

beforeEach(() => {
  vi.mocked(invoke).mockReset();
  store.clear();
});
afterEach(() => {
  cleanup();
  vi.mocked(invoke).mockReset();
});

describe("useProjectBranches", () => {
  it("lists the project's branches", async () => {
    vi.mocked(invoke).mockResolvedValueOnce([BRANCH]);
    const { result } = renderHook(() => useProjectBranches("pr1"), {
      wrapper: wrapperFor(makeClient()),
    });
    await waitFor(() => expect(result.current.isSuccess).toBe(true));
    expect(result.current.data).toEqual([BRANCH]);
    expect(invoke).toHaveBeenCalledWith("list_project_branches", {
      code: "pr1",
    });
  });
});

describe("useCreateProjectBranch", () => {
  it("creates the branch and invalidates the branch list", async () => {
    vi.mocked(invoke).mockResolvedValueOnce(BRANCH);
    const client = makeClient();
    const invalidate = vi.spyOn(client, "invalidateQueries");
    const { result } = renderHook(() => useCreateProjectBranch("pr1"), {
      wrapper: wrapperFor(client),
    });

    await act(async () => {
      await result.current.mutateAsync({ branch: "2026Q1" });
    });

    expect(invoke).toHaveBeenCalledWith("create_project_branch", {
      code: "pr1",
      body: { branch: "2026Q1" },
    });
    expect(invalidate).toHaveBeenCalledWith({
      queryKey: queryKeys.project.branches("pr1"),
    });
  });
});

describe("useUpdateProjectBranch", () => {
  it("renames the branch and invalidates the branch list", async () => {
    vi.mocked(invoke).mockResolvedValueOnce(BRANCH);
    const client = makeClient();
    const invalidate = vi.spyOn(client, "invalidateQueries");
    const { result } = renderHook(() => useUpdateProjectBranch("pr1"), {
      wrapper: wrapperFor(client),
    });

    await act(async () => {
      await result.current.mutateAsync({ id: 4, body: { branch: "2026Q2" } });
    });

    expect(invoke).toHaveBeenCalledWith("update_project_branch", {
      code: "pr1",
      id: 4,
      body: { branch: "2026Q2" },
    });
    expect(invalidate).toHaveBeenCalledWith({
      queryKey: queryKeys.project.branches("pr1"),
    });
  });
});

describe("useActiveBranchId", () => {
  it("reads the active branch out of the settings store", async () => {
    store.set("activeBranch.pr1", 4);
    const { result } = renderHook(() => useActiveBranchId("pr1"), {
      wrapper: wrapperFor(makeClient()),
    });
    await waitFor(() => expect(result.current.isSuccess).toBe(true));
    expect(result.current.data).toBe(4);
  });

  it("reports no active branch when the store has none", async () => {
    const { result } = renderHook(() => useActiveBranchId("pr1"), {
      wrapper: wrapperFor(makeClient()),
    });
    await waitFor(() => expect(result.current.isSuccess).toBe(true));
    expect(result.current.data).toBeNull();
  });
});

describe("useSetActiveBranchId", () => {
  it("writes the active branch and updates the cached value", async () => {
    const client = makeClient();
    const { result } = renderHook(
      () => ({
        active: useActiveBranchId("pr1"),
        setActive: useSetActiveBranchId("pr1"),
      }),
      { wrapper: wrapperFor(client) },
    );
    await waitFor(() => expect(result.current.active.isSuccess).toBe(true));

    await act(async () => {
      await result.current.setActive.mutateAsync(4);
    });

    expect(store.get("activeBranch.pr1")).toBe(4);
    // The observer re-renders on the next tick, so the cached value is
    // awaited rather than read synchronously.
    await waitFor(() => expect(result.current.active.data).toBe(4));
  });

  it("clears the active branch when given null", async () => {
    store.set("activeBranch.pr1", 4);
    const client = makeClient();
    const { result } = renderHook(
      () => ({
        active: useActiveBranchId("pr1"),
        setActive: useSetActiveBranchId("pr1"),
      }),
      { wrapper: wrapperFor(client) },
    );
    await waitFor(() => expect(result.current.active.isSuccess).toBe(true));
    expect(result.current.active.data).toBe(4);

    await act(async () => {
      await result.current.setActive.mutateAsync(null);
    });

    expect(store.get("activeBranch.pr1")).toBeNull();
    await waitFor(() => expect(result.current.active.data).toBeNull());
  });
});
