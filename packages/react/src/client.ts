import { useMemo } from "react";
import {
  FileBaseClient,
  type FileBaseClientOptions,
} from "@filebase/client";

export function useFileBaseClient(options: FileBaseClientOptions): FileBaseClient {
  return useMemo(
    () => new FileBaseClient(options),
    [
      options.signEndpoint,
      options.fetch,
      options.signCredentials,
      JSON.stringify(options.signHeaders ?? null),
    ]
  );
}
