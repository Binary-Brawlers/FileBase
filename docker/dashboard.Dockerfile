FROM node:20-alpine AS builder
WORKDIR /app
RUN corepack enable
COPY . .
RUN pnpm install --no-frozen-lockfile && pnpm --filter @filebase/dashboard build

FROM node:20-alpine
WORKDIR /app
ENV NODE_ENV=production
RUN corepack enable
COPY --from=builder /app/apps/dashboard ./apps/dashboard
COPY --from=builder /app/node_modules ./node_modules
EXPOSE 3000
CMD ["pnpm", "--filter", "@filebase/dashboard", "start"]
