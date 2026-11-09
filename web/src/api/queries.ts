import { queryOptions } from "@tanstack/react-query";
import { api } from "./api";

// Stable project handles resolve individual folders; the startup root stays unchanged.
export const repositoriesQueryOptions = () => queryOptions({
  queryKey: ["repositories"],
  queryFn: async ({ signal }) => (await api.repos(signal)).repos ?? [],
});
