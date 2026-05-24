import { FileBase, type FileBaseOptions } from "@binary-brawlers/filebase-node";
import { FileBaseError, type FileBaseSignRequest, type FileBaseUploadSession } from "@binary-brawlers/filebase-shared";

export {
  FileBase,
  FileBaseError,
  type FileBaseOptions,
  type FileBaseSignRequest,
  type FileBaseUploadSession,
};
export { FileBaseClient } from "@binary-brawlers/filebase-client";

export type CreateFileBaseRouteOptions = FileBaseOptions & {
  /**
   * Restrict which presets clients can request. If set, the route rejects any
   * `preset` / `presetId` not in this list.
   */
  allowedPresets?: string[];
  /**
   * Optional hook to authorize the request (e.g. check the user session). Throw
   * or return `false` to reject. Return a partial `FileBaseSignRequest` to
   * override fields the client sent (useful for forcing `projectId`).
   */
  authorize?: (
    request: Request,
    sign: FileBaseSignRequest
  ) =>
    | Promise<FileBaseSignRequest | boolean | void>
    | FileBaseSignRequest
    | boolean
    | void;
};

export type FileBaseRouteHandler = (request: Request) => Promise<Response>;

/**
 * Returns a Next.js App Router POST handler that exchanges the developer's
 * private API key for a signed upload session that can be returned to the
 * browser.
 *
 * @example
 *   // app/api/upload/sign/route.ts
 *   export const POST = createFileBaseRoute({
 *     apiKey: process.env.FILEBASE_API_KEY!,
 *     gatewayUrl: process.env.FILEBASE_GATEWAY_URL!,
 *   });
 */
export function createFileBaseRoute(options: CreateFileBaseRouteOptions): FileBaseRouteHandler {
  const { allowedPresets, authorize, ...clientOptions } = options;
  const client = new FileBase(clientOptions);
  const allowed = allowedPresets ? new Set(allowedPresets) : null;

  return async function POST(request: Request): Promise<Response> {
    let body: FileBaseSignRequest = {};
    if (request.headers.get("content-type")?.includes("application/json")) {
      body = (await request.json().catch(() => ({}))) as FileBaseSignRequest;
    }

    if (allowed) {
      const requested = body.preset ?? body.presetId;
      if (!requested || !allowed.has(requested)) {
        return jsonError(403, "preset is not allowed");
      }
    }

    if (authorize) {
      const result = await authorize(request, body);
      if (result === false) return jsonError(401, "not authorized");
      if (result && typeof result === "object") {
        body = { ...body, ...result };
      }
    }

    try {
      const session = await client.createUploadSession(body);
      return Response.json({ data: session satisfies FileBaseUploadSession });
    } catch (cause) {
      if (cause instanceof FileBaseError) {
        return jsonError(cause.status ?? 502, cause.message, cause.code);
      }
      return jsonError(500, "failed to create upload session");
    }
  };
}

function jsonError(status: number, message: string, code?: string): Response {
  return Response.json(
    { error: { message, code: code ?? "request_failed" } },
    { status }
  );
}
