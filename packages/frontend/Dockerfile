FROM node:20-alpine AS base

# Install dependencies only when needed
FROM base AS deps
RUN apk add --no-cache libc6-compat
WORKDIR /app

# Install pnpm
RUN corepack enable && corepack prepare pnpm@8.15.0 --activate

# Install dependencies
COPY package.json pnpm-lock.yaml* ./
RUN pnpm install --frozen-lockfile

# Development stage (for docker-compose)
FROM base AS development
WORKDIR /app

RUN apk add --no-cache libc6-compat

# Install pnpm
RUN corepack enable && corepack prepare pnpm@8.15.0 --activate

COPY package.json pnpm-lock.yaml* ./
RUN pnpm install

COPY . .

EXPOSE 3000

ENV NODE_ENV development
ENV NEXT_TELEMETRY_DISABLED 1

CMD ["pnpm", "dev"]

# Rebuild the source code only when needed
FROM base AS builder
WORKDIR /app

# Install pnpm
RUN corepack enable && corepack prepare pnpm@8.15.0 --activate

COPY --from=deps /app/node_modules ./node_modules
COPY . .

# Build arguments for runtime config
ARG NEXT_PUBLIC_API_URL
ENV NEXT_PUBLIC_API_URL=$NEXT_PUBLIC_API_URL

# Disable telemetry during build
ENV NEXT_TELEMETRY_DISABLED 1

RUN pnpm build

# Production image, copy all the files and run next
FROM base AS runner
WORKDIR /app

ENV NODE_ENV production
ENV NEXT_TELEMETRY_DISABLED 1

RUN addgroup --system --gid 1001 nodejs
RUN adduser --system --uid 1001 nextjs

# Copy built application
COPY --from=builder /app/public ./public
COPY --from=builder --chown=nextjs:nodejs /app/.next/standalone ./
COPY --from=builder --chown=nextjs:nodejs /app/.next/static ./.next/static

USER nextjs

EXPOSE 3000

ENV PORT 3000
ENV HOSTNAME "0.0.0.0"

CMD ["node", "server.js"]

