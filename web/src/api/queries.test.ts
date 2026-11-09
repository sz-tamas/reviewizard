import { afterEach, expect, test, vi } from "vitest";
import { ApiError, api } from "./api";
import { repositoriesQueryOptions } from "./queries";
import { createQueryClient } from "../app/queryClient";

vi.mock("./api", async (original) => {
  const mod = await original<typeof import("./api")>();
  return { ...mod, api: { ...mod.api, repos: vi.fn() } };
});
afterEach(() => vi.clearAllMocks());

test("repository reads share a cache and normalize nullable API lists", async () => {
  const client = createQueryClient();
  vi.mocked(api.repos).mockResolvedValue({ repos: null as never });
  const [first, second] = await Promise.all([
    client.fetchQuery(repositoriesQueryOptions()),
    client.fetchQuery(repositoriesQueryOptions()),
  ]);
  expect(first).toEqual([]);
  expect(second).toEqual([]);
  await client.fetchQuery(repositoriesQueryOptions());
  expect(api.repos).toHaveBeenCalledTimes(1);
  expect(api.repos).toHaveBeenCalledWith(expect.any(AbortSignal));
  client.clear();
});

test("a failed repository read stays an error and a subsequent read can recover", async () => {
  const client = createQueryClient();
  const failure = new ApiError("Git unavailable", 500);
  vi.mocked(api.repos).mockRejectedValueOnce(failure).mockResolvedValueOnce({ repos: [] });
  await expect(client.fetchQuery(repositoriesQueryOptions())).rejects.toBe(failure);
  expect(client.getQueryData(repositoriesQueryOptions().queryKey)).toBeUndefined();
  expect(api.repos).toHaveBeenCalledTimes(1);
  await expect(client.fetchQuery(repositoriesQueryOptions())).resolves.toEqual([]);
  client.clear();
});
