# @binary-brawlers/filebase-next

Next.js helpers for [FileBase](https://github.com/binary-brawlers/filebase) —
ship a signed-upload route handler in one line.

This package bundles the server-side admin client
(`@binary-brawlers/filebase-node`) and a route-builder that returns a
`Request → Response` handler compatible with the App Router (and Route
Handlers in general).

## Install

```bash
npm install @binary-brawlers/filebase-next
```

Peer dependency: `next >= 14`.

## Environment variables

```bash
FILEBASE_API_KEY=fb_live_…           # generate in your FileBase dashboard
FILEBASE_GATEWAY_URL=https://uploads.example.com
```

## App Router

```ts
// app/api/upload/sign/route.ts
import { createFileBaseRoute } from "@binary-brawlers/filebase-next";

export const POST = createFileBaseRoute({
  apiKey: process.env.FILEBASE_API_KEY!,
  gatewayUrl: process.env.FILEBASE_GATEWAY_URL!,
});
```

Then on the client (e.g. with `@binary-brawlers/filebase-react`):

```tsx
import { UploadButton } from "@binary-brawlers/filebase-react";

<UploadButton signEndpoint="/api/upload/sign" preset="profile_images">
  Upload
</UploadButton>
```

## Restrict allowed presets

By default the route forwards whatever `preset` the client sends. Lock it
down with an allow-list:

```ts
export const POST = createFileBaseRoute({
  apiKey: process.env.FILEBASE_API_KEY!,
  gatewayUrl: process.env.FILEBASE_GATEWAY_URL!,
  allowedPresets: ["profile_images", "post_attachments"],
});
```

Requests for any other preset get `403`.

## Authorize the request

The `authorize` hook runs before the session is created. Return `false` to
reject, return a partial `FileBaseSignRequest` to override fields the client
sent (typical use: force a `projectId` based on the logged-in user).

```ts
import { getServerSession } from "next-auth";
import { createFileBaseRoute } from "@binary-brawlers/filebase-next";

export const POST = createFileBaseRoute({
  apiKey: process.env.FILEBASE_API_KEY!,
  gatewayUrl: process.env.FILEBASE_GATEWAY_URL!,
  allowedPresets: ["user_uploads"],
  authorize: async (request, sign) => {
    const session = await getServerSession();
    if (!session?.user) return false;
    return { projectId: session.user.projectId };
  },
});
```

## Pages Router

Wrap the handler in a thin adapter:

```ts
// pages/api/upload/sign.ts
import { createFileBaseRoute } from "@binary-brawlers/filebase-next";
import type { NextApiRequest, NextApiResponse } from "next";

const handler = createFileBaseRoute({
  apiKey: process.env.FILEBASE_API_KEY!,
  gatewayUrl: process.env.FILEBASE_GATEWAY_URL!,
});

export default async function (req: NextApiRequest, res: NextApiResponse) {
  if (req.method !== "POST") return res.status(405).end();
  const webReq = new Request(`http://x${req.url}`, {
    method: "POST",
    headers: req.headers as Record<string, string>,
    body: JSON.stringify(req.body ?? {}),
  });
  const r = await handler(webReq);
  res.status(r.status);
  r.headers.forEach((v, k) => res.setHeader(k, v));
  res.send(await r.text());
}
```

## Direct server-side uploads

Sometimes you want to upload from a server action or webhook handler
without involving the browser:

```ts
import { FileBase } from "@binary-brawlers/filebase-next";

const filebase = new FileBase({
  apiKey: process.env.FILEBASE_API_KEY!,
  gatewayUrl: process.env.FILEBASE_GATEWAY_URL!,
});

const file = await filebase.uploadFile(buffer, {
  preset: "exports",
  filename: "report.pdf",
  contentType: "application/pdf",
});
```

## Errors

The route returns:

- `401 { error: { code: "request_failed", message: "not authorized" } }`
- `403 { error: { code: "request_failed", message: "preset is not allowed" } }`
- `5xx { error: { code, message } }` propagated from the FileBase API

## License

MIT
