# syntax=docker/dockerfile:1

FROM node:22-alpine AS build

WORKDIR /app

RUN corepack enable

COPY package.json pnpm-lock.yaml ./
RUN pnpm install --frozen-lockfile

COPY . .

ARG PUBLIC_TEPUB_API_BASE=/api
ENV PUBLIC_TEPUB_API_BASE=${PUBLIC_TEPUB_API_BASE}
RUN pnpm build:web

FROM node:22-alpine

WORKDIR /app

COPY --from=build /app/build ./build
COPY server ./server

ENV PORT=5233
ENV TEPUB_WEB_ROOT=/app/build
ENV TEPUB_DATA_DIR=/data

EXPOSE 5233

CMD ["node", "server/web-server.mjs"]
