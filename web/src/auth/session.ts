import { queryOptions } from "@tanstack/react-query";

import { getSession } from "./api";

export const sessionQueryKey = ["auth", "session"] as const;

export const sessionQueryOptions = () =>
  queryOptions({
    queryKey: sessionQueryKey,
    queryFn: getSession,
    retry: false,
    staleTime: 60_000,
  });
